use crate::project::model::UProject;
use std::fs;
use std::path::Path;

pub struct ApsFormat;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApsMigrationReport {
    pub source_version: u32,
    pub target_version: u32,
    pub warnings: Vec<String>,
}

impl ApsFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let clean = content.trim_start_matches('\u{feff}').trim();
        if let Ok(mut project) = serde_json::from_str::<UProject>(clean) {
            project.normalize();
            migrate_project_schema(&mut project);
            return Ok(project);
        }
        if let Ok(project) = yaml_serde::from_str::<UProject>(clean) {
            let mut project = project;
            project.normalize();
            migrate_project_schema(&mut project);
            return Ok(project);
        }
        crate::formats::UstxFormat::parse_str(clean)
    }

    /// Parses an APS document and reports compatibility work performed while
    /// loading it. Legacy APS files intentionally remain valid: absent schema
    /// metadata means version 0, while the in-memory model is upgraded to the
    /// current contract.
    pub fn parse_str_with_report(
        content: &str,
    ) -> Result<(UProject, ApsMigrationReport), Box<dyn std::error::Error>> {
        let clean = content.trim_start_matches('\u{feff}').trim();
        let source_json = serde_json::from_str::<serde_json::Value>(clean).ok();
        let source_version = source_json
            .as_ref()
            .and_then(|value| value.get("schema_version").and_then(|v| v.as_u64()))
            .map(|value| value as u32)
            .unwrap_or(0);
        let project = Self::parse_str(clean)?;
        let mut report = ApsMigrationReport {
            source_version,
            target_version: crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION,
            warnings: Vec::new(),
        };
        if source_version == 0 {
            report
                .warnings
                .push("APS legado sem schema_version migrado para o contrato atual".into());
        } else if source_version > report.target_version {
            report.warnings.push(format!(
                "APS declara versão {source_version}, mas este host conhece até {}",
                report.target_version
            ));
        } else if source_version == 1
            && source_json
                .as_ref()
                .is_some_and(|value| value.get("extensions").is_none())
        {
            report.warnings.push(
                "APS versão 1 migrado: extensões associadas não existiam e foram inicializadas vazias"
                    .into(),
            );
        } else if source_version == 2 {
            report.warnings.push(
                "APS versão 2 migrado para v3: controles avançados por fonema ausentes recebem herança da nota"
                    .into(),
            );
        } else if source_version == 3 {
            report.warnings.push(
                "APS versão 3 migrado para v4: estado de renderização progressiva inicializado vazio"
                    .into(),
            );
        }
        Ok((project, report))
    }

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut project = project.clone();
        if project.schema_version > crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION {
            return Err(format!(
                "não é seguro salvar APS da versão futura {} (host conhece até {})",
                project.schema_version,
                crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION
            )
            .into());
        }
        project.schema_version = crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION;
        project.normalize();
        let json_str = serde_json::to_string_pretty(&project)?;
        fs::write(path, json_str)?;
        Ok(())
    }
}

fn migrate_project_schema(project: &mut UProject) {
    if project.schema_version < crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION {
        project.schema_version = crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{
        UNote, UProjectExtension, UProjectRenderChunk, UProjectRenderState,
    };

    #[test]
    fn test_aps_roundtrip() {
        let mut proj = UProject::default();
        proj.name = "Saturno Song".to_string();
        proj.bpm = 140.0;
        proj.voicebank = Some("Standard Voicebank".to_string());
        proj.voicebank_path = Some("/path/to/voicebank".to_string());
        proj.phonemizer = Some(crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV);
        proj.resampler = Some("straycat-rs (UtaUtaUtau)".to_string());
        proj.wavtool = Some("wavtool-yawu".to_string());
        proj.flags = "P86Q5".to_string();
        proj.tracks[0].flags = "g-4".to_string();
        proj.sample_rate = Some(48000);
        proj.render_threads = Some(8);
        proj.extensions.push(UProjectExtension {
            id: "org.example.engine".to_string(),
            version: "1.2.0".to_string(),
            origin: "/plugins/engine/kamafeu-extension.json".to_string(),
            manifest_fingerprint: "abc123".to_string(),
        });
        proj.render_state = Some(UProjectRenderState {
            pipeline_fingerprint: "pipeline-test".to_string(),
            chunks: vec![UProjectRenderChunk {
                index: 2,
                start_ms: 1000.0,
                end_ms: 2000.0,
                is_final: false,
                status: "failed".to_string(),
                error: Some("engine unavailable".to_string()),
            }],
        });
        let mut note = UNote::new("k ae.ae n.", "C4", 0.0, 480.0);
        note.timbre = Some("Soft".to_string());
        note.voicebank_pitch = Some("C4".to_string());
        note.set_phoneme_boundary(2, 1, 360.0);
        note.envelope.p2 = 25.0;
        note.envelope.p5 = 45.0;
        note.envelope.crossfade_ms = 60.0;
        note.phoneme_overrides
            .push(crate::project::model::UPhonemeOverride {
                index: 0,
                phoneme: Some("k a".to_string()),
                flags: Some("P86Qcustom".to_string()),
                envelope: Some(crate::dsp::envelope::UtauEnvelope {
                    p2: 12.0,
                    ..Default::default()
                }),
                vibrato: Some(crate::dsp::pitch::VibratoParam {
                    length_pct: 40.0,
                    ..Default::default()
                }),
                pitch_bend: Some(crate::project::model::UPitchBend {
                    portamento_length_ms: 55.0,
                    ..Default::default()
                }),
                ..Default::default()
            });
        proj.parts[0].notes.push(note);

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("song.aps");

        ApsFormat::save_file(&proj, &path).unwrap();
        let loaded = ApsFormat::load_file(&path).unwrap();

        assert_eq!(loaded.name, "Saturno Song");
        assert_eq!(loaded.bpm, 140.0);
        assert_eq!(loaded.voicebank.as_deref(), Some("Standard Voicebank"));
        assert_eq!(loaded.voicebank_path.as_deref(), Some("/path/to/voicebank"));
        assert_eq!(
            loaded.phonemizer,
            Some(crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV)
        );
        assert_eq!(
            loaded.resampler.as_deref(),
            Some("straycat-rs (UtaUtaUtau)")
        );
        assert_eq!(loaded.wavtool.as_deref(), Some("wavtool-yawu"));
        assert_eq!(loaded.flags, "P86Q5");
        assert_eq!(loaded.tracks[0].flags, "g-4");
        assert_eq!(loaded.sample_rate, Some(48000));
        assert_eq!(loaded.render_threads, Some(8));
        assert_eq!(loaded.extensions.len(), 1);
        assert_eq!(loaded.extensions[0].id, "org.example.engine");
        assert_eq!(loaded.extensions[0].manifest_fingerprint, "abc123");
        assert_eq!(loaded.render_state, proj.render_state);
        assert_eq!(loaded.parts[0].notes.len(), 1);
        assert_eq!(loaded.parts[0].notes[0].timbre.as_deref(), Some("Soft"));
        assert_eq!(
            loaded.parts[0].notes[0].voicebank_pitch.as_deref(),
            Some("C4")
        );
        assert_eq!(loaded.parts[0].notes[0].lyric, "k ae.ae n.");
        assert_eq!(
            loaded.parts[0].notes[0].phoneme_durations_ms,
            [360.0, 120.0]
        );
        assert_eq!(loaded.parts[0].notes[0].envelope.p2, 25.0);
        assert_eq!(loaded.parts[0].notes[0].envelope.p5, 45.0);
        assert_eq!(loaded.parts[0].notes[0].envelope.crossfade_ms, 60.0);
        let phoneme_override = &loaded.parts[0].notes[0].phoneme_overrides[0];
        assert_eq!(phoneme_override.phoneme.as_deref(), Some("k a"));
        assert_eq!(phoneme_override.flags.as_deref(), Some("P86Qcustom"));
        assert_eq!(phoneme_override.envelope.as_ref().unwrap().p2, 12.0);
        assert_eq!(phoneme_override.vibrato.as_ref().unwrap().length_pct, 40.0);
        assert_eq!(
            phoneme_override
                .pitch_bend
                .as_ref()
                .unwrap()
                .portamento_length_ms,
            55.0
        );
    }

    #[test]
    fn aps_v3_migration_reports_the_render_state_upgrade() {
        let mut legacy = serde_json::to_value(UProject::default()).unwrap();
        legacy["schema_version"] = serde_json::json!(3);
        legacy.as_object_mut().unwrap().remove("render_state");
        let (project, report) = ApsFormat::parse_str_with_report(&legacy.to_string()).unwrap();
        assert_eq!(
            project.schema_version,
            crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION
        );
        assert!(project.render_state.is_none());
        assert!(report.warnings.iter().any(|warning| warning.contains("v4")));
    }

    #[test]
    fn test_compute_next_incremental_path() {
        use crate::gui::KamafeuStudioApp;

        let temp = tempfile::tempdir().unwrap();
        let base_path = temp.path().join("my_song.aps");

        // Initial incremental from my_song.aps should be my_song_v2.aps
        let v2 = KamafeuStudioApp::compute_next_incremental_path(&base_path);
        assert_eq!(v2.file_name().unwrap(), "my_song_v2.aps");

        // If my_song_v2.aps exists on disk, next is v3
        std::fs::write(&v2, "{}").unwrap();
        let v3 = KamafeuStudioApp::compute_next_incremental_path(&v2);
        assert_eq!(v3.file_name().unwrap(), "my_song_v3.aps");
    }

    #[test]
    fn legacy_aps_is_migrated_and_reported_without_losing_notes() {
        let (project, report) = ApsFormat::parse_str_with_report(
            r#"{"name":"legacy","bpm":120,"tracks":[],"parts":[]}"#,
        )
        .unwrap();
        assert_eq!(
            project.schema_version,
            crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION
        );
        assert_eq!(report.source_version, 0);
        assert_eq!(
            report.target_version,
            crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION
        );
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn aps_schema_version_roundtrips() {
        let project = UProject::default();
        let json = serde_json::to_string(&project).unwrap();
        let (loaded, report) = ApsFormat::parse_str_with_report(&json).unwrap();
        assert_eq!(loaded.schema_version, project.schema_version);
        assert_eq!(report.source_version, project.schema_version);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn aps_v1_migration_reports_missing_extension_metadata() {
        let content = r#"{
            "schema_version": 1,
            "name": "legacy",
            "bpm": 120.0,
            "tracks": [],
            "parts": []
        }"#;
        let (project, report) = ApsFormat::parse_str_with_report(content).unwrap();
        assert_eq!(
            project.schema_version,
            crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION
        );
        assert_eq!(
            report.target_version,
            crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION
        );
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("extensões associadas")));
    }

    #[test]
    fn aps_v2_migration_reports_advanced_phoneme_inheritance() {
        let content = r#"{
            "schema_version": 2,
            "name": "legacy-v2",
            "bpm": 120.0,
            "tracks": [],
            "parts": []
        }"#;
        let (project, report) = ApsFormat::parse_str_with_report(content).unwrap();
        assert_eq!(
            project.schema_version,
            crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION
        );
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("controles avançados por fonema")));
    }

    #[test]
    fn future_aps_schema_is_not_silently_downgraded_on_save() {
        let mut project = UProject::default();
        project.schema_version = crate::project::model::CURRENT_PROJECT_SCHEMA_VERSION + 1;
        let path = tempfile::tempdir().unwrap().path().join("future.aps");
        let error = ApsFormat::save_file(&project, path).expect_err("future schema");
        assert!(error.to_string().contains("versão futura"));
    }
}
