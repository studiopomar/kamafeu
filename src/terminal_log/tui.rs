//! FTXUI-backed dashboard process for native desktop runs.

use std::{
    env,
    io::{self, IsTerminal},
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static TRANSPORT: OnceLock<Mutex<Transport>> = OnceLock::new();

struct Transport {
    socket: UdpSocket,
    target: SocketAddr,
}

pub(super) struct TuiHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for TuiHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        send("STOP");
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        ACTIVE.store(false, Ordering::Relaxed);
        env::remove_var("KAMAFEU_TUI_ACTIVE");

        // Failsafe: Reset xterm mouse tracking and return cursor/screen to normal.
        // If kamafeu-tui terminated or didn't get to flush its cleanup, this prevents
        // mouse motion escape codes (e.g. 35;32;16M) from printing into the shell.
        if io::stdout().is_terminal() {
            use std::io::Write;
            let _ = io::stdout().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l\x1b[?25h\x1b[?1049l");
            let _ = io::stdout().flush();
        }
    }
}

pub(super) fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub(super) fn push(level: &str, scope: &str, message: &str) -> bool {
    let message = message.lines().next().unwrap_or_default();
    send(&format!(
        "{}\t{}\t{}",
        clean(level),
        clean(scope),
        clean(message)
    ))
}

fn send(message: &str) -> bool {
    if !active() && message != "STOP" {
        return false;
    }
    let Some(transport) = TRANSPORT.get() else {
        return false;
    };
    let Ok(transport) = transport.lock() else {
        return false;
    };
    match transport
        .socket
        .send_to(message.as_bytes(), transport.target)
    {
        Ok(_) => true,
        Err(_) => {
            ACTIVE.store(false, Ordering::Relaxed);
            false
        }
    }
}

fn clean(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .take(4096)
        .collect()
}

fn dashboard_path() -> Option<PathBuf> {
    let executable_dir = env::current_exe().ok()?.parent()?.to_path_buf();
    let name = if cfg!(windows) {
        "kamafeu-tui.exe"
    } else {
        "kamafeu-tui"
    };
    let bundled = executable_dir.join(name);
    if bundled.is_file() {
        return Some(bundled);
    }
    let development = executable_dir
        .join("..")
        .join("console-tui-build")
        .join("bin")
        .join("Release")
        .join(name);
    if development.is_file() {
        return Some(development);
    }
    if cfg!(target_os = "macos") {
        let outside_app = executable_dir.join("..").join("..").join("..").join(name);
        if outside_app.is_file() {
            return Some(outside_app);
        }
    }
    None
}

fn wait_until_ready(socket: &UdpSocket, child: &mut Child) -> Option<SocketAddr> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut buffer = [0_u8; 128];
    while Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            return None;
        }
        match socket.recv_from(&mut buffer) {
            Ok((length, _)) => {
                let message = std::str::from_utf8(&buffer[..length]).ok()?;
                let port = message.strip_prefix("READY\t")?.parse::<u16>().ok()?;
                return Some(SocketAddr::from(([127, 0, 0, 1], port)));
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return None,
        }
    }
    None
}

pub(super) fn start() -> Option<TuiHandle> {
    let setting = env::var("KAMAFEU_TUI")
        .unwrap_or_else(|_| "auto".to_string())
        .to_ascii_lowercase();
    if matches!(setting.as_str(), "0" | "off" | "false" | "no")
        || !io::stdout().is_terminal()
        || !io::stderr().is_terminal()
    {
        return None;
    }

    let Some(path) = dashboard_path() else {
        eprintln!("[WARN] Console dashboard helper kamafeu-tui is missing; using plain logs");
        return None;
    };
    let socket = UdpSocket::bind(("127.0.0.1", 0)).ok()?;
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .ok()?;
    let parent_port = socket.local_addr().ok()?.port();
    let mut child = Command::new(path)
        .arg(parent_port.to_string())
        .arg(std::process::id().to_string())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("KAMAFEU_TUI_ACTIVE", "1")
        .spawn()
        .ok()?;
    let Some(target) = wait_until_ready(&socket, &mut child) else {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    let _ = TRANSPORT.set(Mutex::new(Transport { socket, target }));
    env::set_var("KAMAFEU_TUI_ACTIVE", "1");
    ACTIVE.store(true, Ordering::Relaxed);

    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let monitor = thread::Builder::new()
        .name("kamafeu-ftxui-monitor".into())
        .spawn(move || monitor_child(&mut child, thread_stop))
        .ok()?;

    Some(TuiHandle {
        stop,
        thread: Some(monitor),
    })
}

fn monitor_child(child: &mut Child, stop: Arc<AtomicBool>) {
    let mut stop_sent_at = None;
    loop {
        if stop.load(Ordering::Relaxed) {
            let sent_at = stop_sent_at.get_or_insert_with(Instant::now);
            if sent_at.elapsed() > Duration::from_secs(3) {
                let _ = child.kill();
            }
        }
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => break,
            Ok(None) => thread::sleep(Duration::from_millis(100)),
        }
    }
    ACTIVE.store(false, Ordering::Relaxed);
    env::remove_var("KAMAFEU_TUI_ACTIVE");
}
