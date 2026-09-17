//! Verificação estática de voicebanks antes de iniciar o renderer.

use super::{OtoEntry, Voicebank};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoicebankIssueKind {
    MissingWav,
    InvalidWav,
    EmptyWav,
    InvalidOtoTiming,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoicebankIssue {
    pub kind: VoicebankIssueKind,
    pub alias: String,
    pub wav_filename: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct VoicebankDiagnosticReport {
    pub root_path: PathBuf,
    pub entry_count: usize,
    pub unique_wav_count: usize,
    pub readable_wav_count: usize,
    pub missing_wav_count: usize,
    pub invalid_wav_count: usize,
    pub empty_wav_count: usize,
    pub invalid_oto_timing_count: usize,
    pub sample_rates: BTreeMap<u32, usize>,
    pub issues: Vec<VoicebankIssue>,
}

impl VoicebankDiagnosticReport {
    pub fn is_healthy(&self) -> bool {
        self.issues.is_empty()
    }
}

impl Voicebank {
    /// Examina cabeçalhos WAV e limites básicos de `oto.ini` sem renderizar.
    /// Um mesmo WAV referenciado por vários aliases é aberto apenas uma vez.
    pub fn diagnostic_report(&self) -> VoicebankDiagnosticReport {
        let mut report = VoicebankDiagnosticReport {
            root_path: self.root_path.clone(),
            entry_count: self.entries.len(),
            ..VoicebankDiagnosticReport::default()
        };
        let mut inspected_wavs = HashSet::new();

        for entry in self.entries.values() {
            validate_oto_timing(entry, &mut report);

            if !inspected_wavs.insert(entry.wav_filename.clone()) {
                continue;
            }
            report.unique_wav_count += 1;
            let wav_path = self.root_path.join(&entry.wav_filename);
            if !wav_path.is_file() {
                report.missing_wav_count += 1;
                report.issues.push(issue(
                    VoicebankIssueKind::MissingWav,
                    entry,
                    "arquivo WAV referenciado não encontrado",
                ));
                continue;
            }

            match hound::WavReader::open(&wav_path) {
                Ok(reader) => {
                    let spec = reader.spec();
                    if reader.duration() == 0 {
                        report.empty_wav_count += 1;
                        report.issues.push(issue(
                            VoicebankIssueKind::EmptyWav,
                            entry,
                            "WAV não contém amostras de áudio",
                        ));
                    } else {
                        report.readable_wav_count += 1;
                    }
                    *report.sample_rates.entry(spec.sample_rate).or_default() += 1;
                }
                Err(error) => {
                    report.invalid_wav_count += 1;
                    report.issues.push(issue(
                        VoicebankIssueKind::InvalidWav,
                        entry,
                        &format!("WAV não pôde ser lido: {error}"),
                    ));
                }
            }
        }
        report
    }
}

fn issue(kind: VoicebankIssueKind, entry: &OtoEntry, detail: &str) -> VoicebankIssue {
    VoicebankIssue {
        kind,
        alias: entry.alias.clone(),
        wav_filename: entry.wav_filename.clone(),
        detail: detail.to_string(),
    }
}

fn validate_oto_timing(entry: &OtoEntry, report: &mut VoicebankDiagnosticReport) {
    let finite = [
        entry.offset,
        entry.consonant,
        entry.cutoff,
        entry.preutterance,
        entry.overlap,
    ]
    .iter()
    .all(|value| value.is_finite());
    let invalid = !finite
        || entry.offset < 0.0
        || entry.consonant < 0.0
        || entry.preutterance < 0.0
        || entry.overlap < 0.0
        || entry.overlap > entry.preutterance;
    if invalid {
        report.invalid_oto_timing_count += 1;
        report.issues.push(issue(
            VoicebankIssueKind::InvalidOtoTiming,
            entry,
            "offset/consoante/preutterance/overlap inválido ou overlap maior que preutterance",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::OnceLock;

    fn voicebank(root_path: PathBuf, entries: HashMap<String, OtoEntry>) -> Voicebank {
        Voicebank {
            root_path,
            name: "test".to_string(),
            author: String::new(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: OnceLock::new(),
            prefix_map: Default::default(),
            temp_dir: None,
        }
    }

    #[test]
    fn reports_missing_invalid_and_suspicious_oto_entries() {
        let directory = tempfile::tempdir().expect("temp voicebank");
        std::fs::write(directory.path().join("broken.wav"), b"not a wav").expect("broken wav");
        let mut entries = HashMap::new();
        entries.insert(
            "missing".to_string(),
            OtoEntry::new(
                "missing.wav".to_string(),
                "missing".to_string(),
                0.0,
                1.0,
                0.0,
                5.0,
                1.0,
            ),
        );
        entries.insert(
            "broken".to_string(),
            OtoEntry::new(
                "broken.wav".to_string(),
                "broken".to_string(),
                0.0,
                1.0,
                0.0,
                5.0,
                1.0,
            ),
        );
        entries.insert(
            "timing".to_string(),
            OtoEntry::new(
                "broken.wav".to_string(),
                "timing".to_string(),
                0.0,
                1.0,
                0.0,
                5.0,
                9.0,
            ),
        );
        let report = voicebank(directory.path().to_path_buf(), entries).diagnostic_report();
        assert_eq!(report.missing_wav_count, 1);
        assert_eq!(report.invalid_wav_count, 1);
        assert_eq!(report.invalid_oto_timing_count, 1);
        assert!(!report.is_healthy());
    }
}
