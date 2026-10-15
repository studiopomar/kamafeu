//! Compact, ASCII-only terminal presentation for native Kamafeu logs.

use std::fmt::Display;

const RULE: &str = "+--------------------------------------------------------------------------+";

fn level_tag(level: &str) -> String {
    format!("{level:^7}")
}

pub fn info(scope: &str, message: impl Display) {
    emit("INFO", scope, message);
}

pub fn warn(scope: &str, message: impl Display) {
    emit("WARN", scope, message);
}

pub fn error(scope: &str, message: impl Display) {
    emit("ERROR", scope, message);
}

pub fn debug(scope: &str, message: impl Display) {
    emit("DEBUG", scope, message);
}

fn emit(level: &str, scope: &str, message: impl Display) {
    let message = message.to_string();
    let mut lines = message.lines();
    let first = lines.next().unwrap_or_default();
    eprintln!("[{}] [{}] {}", level_tag(level), scope, first);
    for line in lines {
        eprintln!("           | {}", line);
    }
}

pub fn banner(version: &str, codename: &str) {
    eprintln!("{RULE}");
    eprintln!(
        "| {:<72} |",
        format!("KAMAFEU STUDIO v{version} ({codename})")
    );
    eprintln!(
        "| {:<72} |",
        "Vocal editor / OpenUTAU-compatible render pipeline"
    );
    eprintln!("{RULE}");
}

pub fn render(session: u64, elapsed_ms: u128, progress: f32, message: &str) {
    let progress = (progress.clamp(0.0, 1.0) * 100.0).round();
    let mut lines = message.lines();
    let first = lines.next().unwrap_or_default();
    eprintln!(
        "[RENDER #{session:03} {:>6.0}% {:>5}ms] {}",
        progress, elapsed_ms, first
    );
    for line in lines {
        eprintln!("                 | {}", line);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn level_padding_is_stable() {
        assert_eq!(super::level_tag("INFO"), " INFO  ");
        assert_eq!(super::level_tag("ERROR"), " ERROR ");
    }
}
