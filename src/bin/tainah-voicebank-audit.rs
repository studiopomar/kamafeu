//! Read-only inventory of oto.ini entries and referenced WAV files.
//! Usage: cargo run --bin tainah-voicebank-audit -- <voicebank-root> <report.json>

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    voicebank: String,
    oto_files: usize,
    entry_lines: usize,
    entries: Vec<EntryReport>,
}

#[derive(Serialize)]
struct EntryReport {
    oto: String,
    line: usize,
    alias: Option<String>,
    wav: Option<String>,
    offset_ms: Option<f64>,
    consonant_ms: Option<f64>,
    cutoff_ms: Option<f64>,
    sample_rate_hz: Option<u32>,
    duration_ms: Option<f64>,
    peak: Option<f32>,
    rms: Option<f32>,
    status: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("tainah-voicebank-audit: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err(
            "usage: cargo run --bin tainah-voicebank-audit -- <voicebank-root> <report.json>"
                .into(),
        );
    }
    let root = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    if !root.is_dir() {
        return Err(format!("voicebank directory not found: {}", root.display()));
    }
    let mut otos = Vec::new();
    collect_otos(&root, &mut otos).map_err(|error| error.to_string())?;
    otos.sort();
    let mut entries = Vec::new();
    let mut entry_lines = 0;
    for oto_path in &otos {
        let bytes = std::fs::read(oto_path)
            .map_err(|error| format!("read {}: {error}", oto_path.display()))?;
        let (text, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        let text = if let Ok(utf8) = std::str::from_utf8(&bytes) {
            utf8.to_owned()
        } else {
            text.into_owned()
        };
        for (index, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            entry_lines += 1;
            let rel_oto = oto_path
                .strip_prefix(&root)
                .unwrap_or(oto_path)
                .to_string_lossy()
                .replace('\\', "/");
            match kamafeu::oto::parser::OtoParser::parse_str(line) {
                Ok(parsed) => {
                    for (_key, oto) in parsed {
                        let wav_path = oto_path.parent().unwrap_or(&root).join(&oto.wav_filename);
                        entries.push(read_entry(&root, &rel_oto, index + 1, oto, wav_path));
                    }
                }
                Err(error) => entries.push(EntryReport {
                    oto: rel_oto,
                    line: index + 1,
                    alias: None,
                    wav: None,
                    offset_ms: None,
                    consonant_ms: None,
                    cutoff_ms: None,
                    sample_rate_hz: None,
                    duration_ms: None,
                    peak: None,
                    rms: None,
                    status: format!("invalid oto line: {error}"),
                }),
            }
        }
    }
    entries.sort_by(|a, b| (&a.oto, a.line).cmp(&(&b.oto, b.line)));
    let report = Report {
        schema: "kamafeu.tainah-voicebank-audit.v1",
        voicebank: root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        oto_files: otos.len(),
        entry_lines,
        entries,
    };
    let json = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    std::fs::write(&output, json)
        .map_err(|error| format!("write {}: {error}", output.display()))?;
    eprintln!(
        "audited {} oto.ini files and {} active lines; report: {}",
        report.oto_files,
        report.entry_lines,
        output.display()
    );
    Ok(())
}

fn collect_otos(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect_otos(&entry.path(), out)?;
        } else if kind.is_file()
            && entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("oto.ini")
        {
            out.push(entry.path());
        }
    }
    Ok(())
}

fn read_entry(
    root: &Path,
    oto: &str,
    line: usize,
    entry: kamafeu::oto::entry::OtoEntry,
    wav_path: PathBuf,
) -> EntryReport {
    let wav_rel = wav_path
        .strip_prefix(root)
        .unwrap_or(&wav_path)
        .to_string_lossy()
        .replace('\\', "/");
    let mut row = EntryReport {
        oto: oto.into(),
        line,
        alias: Some(entry.alias),
        wav: Some(wav_rel),
        offset_ms: Some(entry.offset),
        consonant_ms: Some(entry.consonant),
        cutoff_ms: Some(entry.cutoff),
        sample_rate_hz: None,
        duration_ms: None,
        peak: None,
        rms: None,
        status: String::new(),
    };
    if !wav_path.is_file() {
        row.status = "missing wav".into();
        return row;
    }
    match hound::WavReader::open(&wav_path) {
        Err(error) => row.status = format!("unreadable wav: {error}"),
        Ok(mut reader) => {
            let spec = reader.spec();
            row.sample_rate_hz = Some(spec.sample_rate);
            let mut count = 0u64;
            let mut peak = 0f64;
            let mut energy = 0f64;
            let samples = reader.samples::<i32>();
            for sample in samples {
                match sample {
                    Ok(value) => {
                        let normalized = value as f64
                            / 2f64.powi((spec.bits_per_sample.saturating_sub(1)).into());
                        peak = peak.max(normalized.abs());
                        energy += normalized * normalized;
                        count += 1;
                    }
                    Err(error) => {
                        row.status = format!("wav decode error: {error}");
                        break;
                    }
                }
            }
            let frame_count = count / u64::from(spec.channels.max(1));
            row.duration_ms =
                Some(frame_count as f64 * 1000.0 / f64::from(spec.sample_rate.max(1)));
            row.peak = Some(peak as f32);
            row.rms = Some(if count > 0 {
                (energy / count as f64).sqrt() as f32
            } else {
                0.0
            });
            if row.status.is_empty() {
                let useful_ms =
                    row.duration_ms.unwrap_or_default() - entry.offset - entry.cutoff.max(0.0);
                row.status = if useful_ms <= 0.0 {
                    "invalid oto region".into()
                } else if peak < 1e-5 {
                    "silent wav".into()
                } else {
                    "ok".into()
                };
            }
        }
    }
    row
}
