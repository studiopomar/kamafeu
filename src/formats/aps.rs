use crate::project::model::UProject;
use std::fs;
use std::path::Path;

pub struct ApsFormat;

impl ApsFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let clean = content.trim_start_matches('\u{feff}').trim();
        if let Ok(project) = serde_json::from_str::<UProject>(clean) {
            return Ok(project);
        }
        if let Ok(project) = yaml_serde::from_str::<UProject>(clean) {
            return Ok(project);
        }
        crate::formats::UstxFormat::parse_str(clean)
    }

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_str = serde_json::to_string_pretty(project)?;
        fs::write(path, json_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::UNote;

    #[test]
    fn test_aps_roundtrip() {
        let mut proj = UProject::default();
        proj.name = "Saturno Song".to_string();
        proj.bpm = 140.0;
        proj.voicebank = Some("VIICTOR VCCV BRAPA".to_string());
        proj.voicebank_path = Some("/Users/victor/Downloads/VIICTOR VCCV BRAPA".to_string());
        proj.phonemizer = Some(crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV);
        proj.resampler = Some("straycat-rs (UtaUtaUtau)".to_string());
        proj.wavtool = Some("wavtool-yawu".to_string());
        proj.sample_rate = Some(48000);
        proj.render_threads = Some(8);
        let mut note = UNote::new("k ae.ae n.", "C4", 0.0, 480.0);
        note.set_phoneme_boundary(2, 1, 360.0);
        note.envelope.p2 = 25.0;
        note.envelope.p5 = 45.0;
        note.envelope.crossfade_ms = 60.0;
        proj.parts[0].notes.push(note);

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("song.aps");

        ApsFormat::save_file(&proj, &path).unwrap();
        let loaded = ApsFormat::load_file(&path).unwrap();

        assert_eq!(loaded.name, "Saturno Song");
        assert_eq!(loaded.bpm, 140.0);
        assert_eq!(loaded.voicebank.as_deref(), Some("VIICTOR VCCV BRAPA"));
        assert_eq!(
            loaded.voicebank_path.as_deref(),
            Some("/Users/victor/Downloads/VIICTOR VCCV BRAPA")
        );
        assert_eq!(
            loaded.phonemizer,
            Some(crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV)
        );
        assert_eq!(
            loaded.resampler.as_deref(),
            Some("straycat-rs (UtaUtaUtau)")
        );
        assert_eq!(loaded.wavtool.as_deref(), Some("wavtool-yawu"));
        assert_eq!(loaded.sample_rate, Some(48000));
        assert_eq!(loaded.render_threads, Some(8));
        assert_eq!(loaded.parts[0].notes.len(), 1);
        assert_eq!(loaded.parts[0].notes[0].lyric, "k ae.ae n.");
        assert_eq!(
            loaded.parts[0].notes[0].phoneme_durations_ms,
            [360.0, 120.0]
        );
        assert_eq!(loaded.parts[0].notes[0].envelope.p2, 25.0);
        assert_eq!(loaded.parts[0].notes[0].envelope.p5, 45.0);
        assert_eq!(loaded.parts[0].notes[0].envelope.crossfade_ms, 60.0);
    }
}
