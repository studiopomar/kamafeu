use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static WINE_PATH_CACHE: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);
static WINE_CUSTOM_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);
static WINE_VERSION_CACHE: Mutex<Option<Option<String>>> = Mutex::new(None);

pub fn set_custom_wine_path(path: Option<PathBuf>) {
    if let Ok(mut guard) = WINE_CUSTOM_OVERRIDE.lock() {
        *guard = path;
    }
    // Invalidate version cache
    if let Ok(mut vguard) = WINE_VERSION_CACHE.lock() {
        *vguard = None;
    }
}

pub fn rescan_wine_executable() -> Option<(PathBuf, String)> {
    let uncached = find_wine_executable_uncached();
    if let Ok(mut guard) = WINE_PATH_CACHE.lock() {
        *guard = Some(uncached.clone());
    }
    if let Ok(mut vguard) = WINE_VERSION_CACHE.lock() {
        *vguard = None;
    }

    let wine_path = uncached?;
    let ver = wine_version_uncached(&wine_path).unwrap_or_else(|| "Wine".to_string());
    if let Ok(mut vguard) = WINE_VERSION_CACHE.lock() {
        *vguard = Some(Some(ver.clone()));
    }
    Some((wine_path, ver))
}

pub fn find_wine_executable() -> Option<PathBuf> {
    if let Ok(guard) = WINE_CUSTOM_OVERRIDE.lock() {
        if let Some(ref custom) = *guard {
            if custom.is_file() {
                return Some(custom.clone());
            }
        }
    }

    if let Ok(guard) = WINE_PATH_CACHE.lock() {
        if let Some(ref cached) = *guard {
            return cached.clone();
        }
    }

    let found = find_wine_executable_uncached();
    if let Ok(mut guard) = WINE_PATH_CACHE.lock() {
        *guard = Some(found.clone());
    }
    found
}

static WINE_SERVER_RUNNING: Mutex<bool> = Mutex::new(false);

pub fn ensure_wine_server_running() {
    let is_running = match WINE_SERVER_RUNNING.lock() {
        Ok(guard) => *guard,
        Err(_) => false,
    };
    if is_running {
        return;
    }

    if let Some(wine_bin) = find_wine_executable() {
        // Find wineserver companion binary next to wine or in PATH
        let server_bin = wine_bin
            .parent()
            .map(|p| p.join("wineserver"))
            .filter(|p| p.is_file());
        if let Some(wineserver) = server_bin {
            // wineserver -p keeps the server persistent indefinitely
            let _ = Command::new(wineserver)
                .arg("-p")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        } else {
            // Fallback: spawn a persistent background wine process
            let _ = Command::new(&wine_bin)
                .arg("cmd.exe")
                .arg("/c")
                .arg("echo prewarm")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output();
        }
        if let Ok(mut guard) = WINE_SERVER_RUNNING.lock() {
            *guard = true;
        }
    }
}

pub fn prewarm_wine_in_background() {
    std::thread::Builder::new()
        .name("wine-prewarm".to_string())
        .spawn(|| {
            ensure_wine_server_running();
        })
        .ok();
}

fn find_wine_executable_uncached() -> Option<PathBuf> {
    for env_var in &["WINE", "WINELOADER", "WINE_PATH"] {
        if let Some(val) = std::env::var_os(env_var) {
            let path = PathBuf::from(val);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    if let Some(path_env) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_env) {
            for binary in &["wine", "wine64", "wine-development"] {
                let candidate = dir.join(binary);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mac_candidates = [
            PathBuf::from("/opt/homebrew/bin/wine"),
            PathBuf::from("/opt/homebrew/bin/wine64"),
            PathBuf::from("/usr/local/bin/wine"),
            PathBuf::from("/usr/local/bin/wine64"),
            PathBuf::from("/opt/local/bin/wine"),
            PathBuf::from("/opt/local/bin/wine64"),
            PathBuf::from("/Applications/Wine.app/Contents/Resources/wine/bin/wine"),
            PathBuf::from("/Applications/Wine.app/Contents/Resources/wine/bin/wine64"),
            PathBuf::from("/Applications/Wine Stable.app/Contents/Resources/wine/bin/wine"),
            PathBuf::from("/Applications/Wine Stable.app/Contents/Resources/wine/bin/wine64"),
            PathBuf::from("/Applications/Wine Stable.app/Contents/MacOS/wine"),
            PathBuf::from("/Applications/Wine Devel.app/Contents/Resources/wine/bin/wine"),
            PathBuf::from("/Applications/Wine Devel.app/Contents/Resources/wine/bin/wine64"),
            PathBuf::from("/Applications/Wine Staging.app/Contents/Resources/wine/bin/wine"),
            PathBuf::from("/Applications/Wine Staging.app/Contents/Resources/wine/bin/wine64"),
            PathBuf::from("/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine"),
            PathBuf::from(
                "/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine64",
            ),
            PathBuf::from("/Applications/Whisky.app/Contents/Resources/Wine/bin/wine"),
            PathBuf::from("/Applications/Whisky.app/Contents/Resources/Wine/bin/wine64"),
            PathBuf::from("/Applications/Whisky.app/Contents/Resources/Libraries/Wine/bin/wine"),
            PathBuf::from("/Applications/Porting Kit.app/Contents/MacOS/portingkit"),
        ];

        for candidate in mac_candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            let user_mac_candidates = [
                home.join(".local/bin/wine"),
                home.join(".local/bin/wine64"),
                home.join("Applications/Wine.app/Contents/Resources/wine/bin/wine"),
                home.join("Applications/Wine Stable.app/Contents/Resources/wine/bin/wine"),
                home.join("Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine"),
                home.join("Applications/Whisky.app/Contents/Resources/Wine/bin/wine"),
                home.join("Applications/Whisky.app/Contents/Resources/Libraries/Wine/bin/wine"),
                home.join("Library/Application Support/CrossOver/bin/wine"),
            ];
            for candidate in user_mac_candidates {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let linux_candidates = [
            PathBuf::from("/usr/bin/wine"),
            PathBuf::from("/usr/bin/wine64"),
            PathBuf::from("/usr/bin/wine-development"),
            PathBuf::from("/usr/local/bin/wine"),
            PathBuf::from("/usr/local/bin/wine64"),
            PathBuf::from("/bin/wine"),
            PathBuf::from("/bin/wine64"),
            PathBuf::from("/usr/lib/wine/wine"),
            PathBuf::from("/usr/lib/wine/wine64"),
            PathBuf::from("/usr/lib32/wine/wine"),
            PathBuf::from("/usr/lib64/wine/wine"),
            PathBuf::from("/run/current-system/sw/bin/wine"),
            PathBuf::from("/var/lib/flatpak/exports/bin/org.winehq.Wine"),
        ];

        for candidate in linux_candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            let user_linux_candidates = [
                home.join(".local/bin/wine"),
                home.join(".local/bin/wine64"),
                home.join(".nix-profile/bin/wine"),
                home.join(".local/share/flatpak/exports/bin/org.winehq.Wine"),
            ];
            for candidate in user_linux_candidates {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    for binary in &["wine", "wine64", "wine-development"] {
        if let Ok(output) = Command::new("which").arg(binary).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                let candidate = PathBuf::from(path);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

pub fn wine_version() -> Option<String> {
    if let Ok(guard) = WINE_VERSION_CACHE.lock() {
        if let Some(ref cached) = *guard {
            return cached.clone();
        }
    }

    let wine_bin = find_wine_executable()?;
    let ver = wine_version_uncached(&wine_bin);
    if let Ok(mut guard) = WINE_VERSION_CACHE.lock() {
        *guard = Some(ver.clone());
    }
    ver
}

pub fn wine_version_uncached(wine_bin: &Path) -> Option<String> {
    let mut cmd = Command::new(wine_bin);
    cmd.arg("--version");
    cmd.env("WINEDEBUG", "-all");
    let output = cmd.output().ok()?;
    if output.status.success() {
        let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ver.is_empty() {
            return Some(ver);
        }
    }
    None
}

pub fn to_wine_windows_path(path: &Path) -> std::ffi::OsString {
    // Resolve symlinks (e.g. /var/folders/... -> /private/var/folders/...) so Wine's
    // Z: drive (which maps to /) maps directly to the actual physical filesystem location.
    let canonical = if path.is_file() || path.is_dir() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else if let Some(parent) = path.parent() {
        if let Ok(canon_parent) = parent.canonicalize() {
            if let Some(file_name) = path.file_name() {
                canon_parent.join(file_name)
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    };

    let s = canonical.to_string_lossy();
    if s.starts_with('/') {
        std::ffi::OsString::from(format!("Z:{}", s.replace('/', "\\")))
    } else {
        std::ffi::OsString::from(s.replace('/', "\\"))
    }
}

pub fn prepare_command(executable_path: &Path) -> Result<Command, String> {
    if !executable_path.is_file() {
        return Err(format!(
            "Executável não encontrado em {:?}",
            executable_path
        ));
    }

    // Canonicalize the path so relative paths (e.g. ./resamplers/straycat-rs) are
    // resolved to absolute paths *before* we set current_dir on the Command.
    let executable_path = &executable_path
        .canonicalize()
        .unwrap_or_else(|_| executable_path.to_path_buf());

    let ext = executable_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    #[cfg(unix)]
    {
        if ext == "exe" {
            let wine_binary = find_wine_executable().ok_or_else(|| {
                format!(
                    "O executável '{:?}' é um binário Windows (.exe), mas o Wine não foi encontrado no sistema.\n\
                     Para executá-lo no macOS ou Linux, instale o Wine (ex: 'brew install --cask wine-stable' no Mac ou 'sudo apt install wine' no Linux).",
                    executable_path.file_name().unwrap_or_default()
                )
            })?;
            let mut cmd = Command::new(wine_binary);
            cmd.arg(executable_path);
            // Suppress verbose Wine debug messages
            cmd.env("WINEDEBUG", "-all");
            // Speed up Wine startup and avoid attempting to install/load Mono and Gecko
            cmd.env("WINEDLLOVERRIDES", "mscoree,mshtml=");
            // Standard UTF-8 locale
            cmd.env("LC_ALL", "C.UTF-8");
            cmd.env("LANG", "C.UTF-8");

            if let Some(parent) = executable_path.parent() {
                if parent.is_dir() {
                    cmd.current_dir(parent);
                }
            }
            return Ok(cmd);
        }

        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(executable_path) {
            let mut permissions = metadata.permissions();
            let mode = permissions.mode();
            if mode & 0o111 == 0 {
                permissions.set_mode(mode | 0o755);
                let _ = std::fs::set_permissions(executable_path, permissions);
            }
        }
    }

    let mut cmd = Command::new(executable_path);
    if let Some(parent) = executable_path.parent() {
        if parent.is_dir() {
            cmd.current_dir(parent);
        }
    }
    Ok(cmd)
}

pub(crate) fn run_with_timeout(
    command: &mut Command,
    timeout: Duration,
    cancel: Option<&AtomicBool>,
) -> Result<Output, String> {
    const MAX_CAPTURE_BYTES: u64 = 1024 * 1024;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("falha ao iniciar processo externo: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("stdout do processo indisponível")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("stderr do processo indisponível")?;

    let stdout_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        stdout
            .take(MAX_CAPTURE_BYTES + 1)
            .read_to_end(&mut buffer)
            .map(|_| buffer)
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        stderr
            .take(MAX_CAPTURE_BYTES + 1)
            .read_to_end(&mut buffer)
            .map(|_| buffer)
    });

    let start = Instant::now();
    let status = loop {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("execução cancelada pelo usuário".to_string());
        }

        match child
            .try_wait()
            .map_err(|error| format!("falha ao consultar status do processo: {error}"))?
        {
            Some(status) => break status,
            None if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "tempo limite excedido ({:.1}s) aguardando o processo",
                    timeout.as_secs_f64()
                ));
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };

    let mut stdout = stdout_reader
        .join()
        .map_err(|_| "falha ao coletar stdout do processo".to_string())?
        .map_err(|error| format!("falha ao ler stdout do processo: {error}"))?;
    let mut stderr = stderr_reader
        .join()
        .map_err(|_| "falha ao coletar stderr do processo".to_string())?
        .map_err(|error| format!("falha ao ler stderr do processo: {error}"))?;
    stdout.truncate(MAX_CAPTURE_BYTES as usize);
    stderr.truncate(MAX_CAPTURE_BYTES as usize);
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wine_detection_on_supported_system() {
        let wine_opt = find_wine_executable();
        if let Some(wine) = wine_opt {
            assert!(wine.is_file(), "Wine path must point to a file: {:?}", wine);
            let ver = wine_version();
            assert!(
                ver.is_some(),
                "Wine version should be readable if wine is found"
            );
            println!("Wine detected: {:?} ({})", wine, ver.unwrap());
        }
    }

    #[test]
    fn test_prepare_command_for_exe() {
        let temp_dir = tempfile::tempdir().unwrap();
        let exe_path = temp_dir.path().join("dummy_resampler.exe");
        std::fs::write(&exe_path, b"MZ").unwrap();

        let cmd_res = prepare_command(&exe_path);
        if find_wine_executable().is_some() {
            assert!(
                cmd_res.is_ok(),
                "prepare_command should succeed for .exe when Wine is installed: {:?}",
                cmd_res
            );
        }
    }

    #[test]
    fn test_to_wine_windows_path() {
        let p = Path::new("/tmp/test_audio.wav");
        let win_path = to_wine_windows_path(p);
        let win_str = win_path.to_string_lossy();
        assert!(
            win_str.starts_with("Z:\\"),
            "Must map root to Z:\\: {}",
            win_str
        );
        assert!(
            !win_str.contains('/'),
            "Must not contain forward slashes: {}",
            win_str
        );
    }
}
