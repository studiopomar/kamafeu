use std::collections::BTreeMap;

use super::super::parser::OtoParseDiagnostic;

pub(super) fn format_parse_diagnostics(diagnostics: &[OtoParseDiagnostic]) -> String {
    if crate::terminal_log::is_verbose() {
        return format!(
            "{} entrada(s) inválida(s)\n{}",
            diagnostics.len(),
            diagnostics
                .iter()
                .map(OtoParseDiagnostic::display)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for diagnostic in diagnostics {
        groups
            .entry(diagnostic.summary())
            .or_default()
            .push(diagnostic.line);
    }

    let details = groups
        .into_iter()
        .map(|(reason, mut lines)| {
            lines.sort_unstable();
            lines.dedup();
            format!(
                "{} ocorrência(s): {} | linhas {}",
                lines.len(),
                reason,
                format_line_ranges(&lines)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{} entrada(s) inválida(s)\n{}", diagnostics.len(), details)
}

pub(super) fn format_line_ranges(lines: &[usize]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut ranges = Vec::new();
    let mut start = lines[0];
    let mut end = start;
    for &line in &lines[1..] {
        if line == end + 1 {
            end = line;
        } else {
            ranges.push(if start == end {
                start.to_string()
            } else {
                format!("{start}-{end}")
            });
            start = line;
            end = line;
        }
    }
    ranges.push(if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    });
    ranges.join(", ")
}
