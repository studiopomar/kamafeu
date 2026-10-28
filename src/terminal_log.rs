//! Compact, ASCII-only terminal presentation for native Kamafeu logs.
//!
//! Runtime diagnostics go to stderr. Human-readable command results use the
//! `out_*` helpers and go to stdout; this keeps `--json` safe for pipelines.

use std::collections::HashSet;
use std::fmt::Display;
use std::sync::{Mutex, OnceLock};

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod tui;

const RULE: &str = "+------------------------------------------------------------------------+";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

fn verbosity() -> Verbosity {
    match std::env::var("KAMAFEU_LOG")
        .unwrap_or_else(|_| "normal".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "quiet" | "silent" => Verbosity::Quiet,
        "verbose" | "debug" | "trace" => Verbosity::Verbose,
        _ => Verbosity::Normal,
    }
}

/// Returns whether optional command details should be shown.
pub fn is_verbose() -> bool {
    verbosity() == Verbosity::Verbose
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub struct TuiGuard {
    _inner: tui::TuiHandle,
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
pub struct TuiGuard;

/// Starts the live terminal dashboard when the process owns an interactive TTY.
/// `KAMAFEU_TUI=off` keeps the traditional line-oriented console.
pub fn start_tui() -> Option<TuiGuard> {
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    {
        return tui::start().map(|inner| TuiGuard { _inner: inner });
    }
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    {
        None
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn tui_active() -> bool {
    tui::active()
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
fn tui_active() -> bool {
    false
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn tui_push(level: &str, scope: &str, message: &str) -> bool {
    tui::push(level, scope, message)
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
fn tui_push(_level: &str, _scope: &str, _message: &str) -> bool {
    false
}

fn level_tag(level: &str) -> String {
    format!("{level:^7}")
}

fn enabled(level: &str) -> bool {
    match level {
        "DEBUG" => verbosity() >= Verbosity::Verbose,
        "INFO" => verbosity() >= Verbosity::Normal,
        _ => verbosity() >= Verbosity::Quiet,
    }
}

pub fn info(scope: &str, message: impl Display) {
    // Normal mode is intentionally quiet: startup and subsystem details are
    // useful while diagnosing, but they should not compete with warnings.
    if is_verbose() || tui_active() {
        emit("INFO", scope, message);
    }
}

pub fn success(scope: &str, message: impl Display) {
    emit("OK", scope, message);
}

pub fn warn(scope: &str, message: impl Display) {
    emit("WARN", scope, message);
}

pub fn warn_once(key: impl Into<String>, scope: &str, message: impl Display) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut seen) = seen.lock() else {
        warn(scope, message);
        return;
    };
    if seen.insert(key.into()) {
        warn(scope, message);
    }
}

pub fn error(scope: &str, message: impl Display) {
    emit("ERROR", scope, message);
}

pub fn debug(scope: &str, message: impl Display) {
    emit("DEBUG", scope, message);
}

fn emit(level: &str, scope: &str, message: impl Display) {
    if !enabled(level) && !tui_active() {
        return;
    }
    let message = message.to_string();
    if tui_push(level, scope, &message) {
        return;
    }
    let mut lines = message.lines();
    let first = lines.next().unwrap_or_default();
    eprintln!("[{}] {:<12} {}", level_tag(level), scope, first);
    for line in lines {
        eprintln!("         |   {}", line);
    }
}

pub fn banner(version: &str, codename: &str) {
    if tui_push(
        "INFO",
        "Kamafeu",
        &format!("Studio v{version} · {codename} · OpenUTAU-compatible vocal editor"),
    ) {
        return;
    }
    if verbosity() == Verbosity::Quiet {
        return;
    }
    eprintln!("{RULE}");
    eprintln!(
        "| {:<72} |",
        format!("KAMAFEU STUDIO v{version} - {codename}")
    );
    eprintln!("| {:<72} |", "OpenUTAU-compatible vocal editor");
    eprintln!("{RULE}");
}

pub fn section(title: &str) {
    eprintln!("+-- {:<70}+", title.to_ascii_uppercase());
}

pub fn render(session: u64, elapsed_ms: u128, progress: f32, message: &str) {
    // Render progress is intentionally opt-in. The GUI already exposes this
    // state, and printing every resampler/wavtool step makes the terminal
    // unusable during long projects.
    if !is_verbose() && !tui_active() {
        return;
    }
    let progress = (progress.clamp(0.0, 1.0) * 100.0).round();
    let dashboard_message = format!("{progress:.0}% · {elapsed_ms}ms · {message}");
    if tui_push(
        "RENDER",
        &format!("Render #{session:03}"),
        &dashboard_message,
    ) {
        return;
    }
    let mut lines = message.lines();
    let first = lines.next().unwrap_or_default();
    eprintln!(
        "[RENDER #{session:03} {:>3.0}% {:>5}ms] {}",
        progress, elapsed_ms, first
    );
    for line in lines {
        eprintln!("                 | {}", line);
    }
}

// Human-readable command output. These intentionally use stdout.
pub fn out_header(title: &str, subtitle: impl Display) {
    println!("[{}] {}", title.to_ascii_uppercase(), subtitle);
}

pub fn out_kv(label: &str, value: impl Display) {
    println!("{label:<18} {value}");
}

pub fn out_status(status: &str, scope: &str, message: impl Display) {
    println!("[{status:^6}] {scope:<14} {message}");
}

pub fn out_list(value: impl Display) {
    println!("  - {value}");
}

pub fn out_rule() {
    println!("{RULE}");
}

#[cfg(test)]
mod tests {
    #[test]
    fn level_padding_is_stable() {
        assert_eq!(super::level_tag("INFO"), " INFO  ");
        assert_eq!(super::level_tag("ERROR"), " ERROR ");
    }

    #[test]
    fn rule_is_ascii_and_fixed_width() {
        assert_eq!(super::RULE.len(), 74);
        assert!(super::RULE.is_ascii());
    }
}
