use super::*;
use kamafeu::extensions::{ExtensionKind, ExtensionManifest, EXTENSION_API_VERSION};

#[test]
fn extension_cli_parameters_parse_json_and_reject_duplicates() {
    let parameters = extensions::parse_extension_parameters(&[
        "mix=0.75".to_string(),
        "enabled=true".to_string(),
        "mode=fast".to_string(),
    ])
    .expect("parse extension parameters");
    assert_eq!(parameters["mix"], json!(0.75));
    assert_eq!(parameters["enabled"], json!(true));
    assert_eq!(parameters["mode"], json!("fast"));

    let duplicate =
        extensions::parse_extension_parameters(&["mix=1".to_string(), "mix=2".to_string()])
            .expect_err("duplicate parameters must be rejected");
    assert!(duplicate.to_string().contains("duplicado"));
}

#[test]
fn automation_extension_invocation_returns_structured_bytes() {
    let directory = tempfile::tempdir().expect("temporary automation extension directory");
    let extension_dir = directory.path().join("example");
    std::fs::create_dir(&extension_dir).expect("extension directory");
    std::fs::write(
        extension_dir.join("kamafeu-extension.json"),
        serde_json::to_vec(&json!({
            "id": "org.kamafeu.automation-test",
            "name": "Automation test",
            "version": "0.1.0",
            "api_version": 1,
            "kind": "analysis",
            "permissions": [],
            "parameters": {
                "mix": {"kind": "number", "minimum": 0.0, "maximum": 1.0}
            },
            "entrypoint": "plugin.wasm"
        }))
        .expect("manifest JSON"),
    )
    .expect("manifest");
    let module = wat::parse_str(
        r#"(module
                (memory (export "memory") 1)
                (data (i32.const 0) "hello")
                (func (export "kamafeu_extension_api_version") (result i32) i32.const 1)
                (func (export "analyze") (param i32) (result i64) i64.const 5)
            )"#,
    )
    .expect("WAT fixture");
    std::fs::write(extension_dir.join("plugin.wasm"), module).expect("plugin");

    let mut parameters = serde_json::Map::new();
    parameters.insert("mix".to_string(), json!(0.5));
    let result = extensions::invoke_extension_json(
        directory.path(),
        "org.kamafeu.automation-test",
        "analyze",
        0,
        true,
        &[],
        &parameters,
    )
    .expect("automation invocation");
    assert_eq!(result["bytes_length"], json!(5));
    assert_eq!(result["utf8"], json!("hello"));
    assert_eq!(result["hex"], json!("68656c6c6f"));
}

#[test]
fn extension_catalog_reports_entrypoint_states_without_execution() {
    let directory = tempfile::tempdir().expect("temporary extension directory");
    let manifest_path = directory.path().join("kamafeu-extension.json");
    let wasm = directory.path().join("plugin.wasm");
    std::fs::write(&wasm, [0_u8]).expect("entrypoint fixture");
    let mut extension = DiscoveredExtension {
        manifest_path,
        manifest: ExtensionManifest {
            id: "org.kamafeu.catalog-test".to_string(),
            name: "Catalog test".to_string(),
            version: "0.1.0".to_string(),
            api_version: EXTENSION_API_VERSION,
            abi_min_version: None,
            abi_max_version: None,
            kind: ExtensionKind::Analysis,
            capabilities: Default::default(),
            formats: Default::default(),
            phonemizers: Default::default(),
            permissions: Default::default(),
            platforms: Default::default(),
            limitations: Default::default(),
            parameters: Default::default(),
            description: String::new(),
            entrypoint: Some("plugin.wasm".to_string()),
        },
    };
    std::fs::write(
        &extension.manifest_path,
        serde_json::to_vec(&extension.manifest).expect("catalog manifest"),
    )
    .expect("catalog manifest file");
    let invalid_dir = directory.path().join("invalid");
    std::fs::create_dir_all(&invalid_dir).expect("invalid extension directory");
    std::fs::write(invalid_dir.join("kamafeu-extension.json"), "{not json}")
        .expect("invalid manifest fixture");
    let catalog =
        extensions::extension_catalog_json(&directory.path().to_path_buf(), false, &[], None, None);
    let catalog_extension = &catalog["extensions"][0];
    assert_eq!(catalog_extension["id"], "org.kamafeu.catalog-test");
    assert_eq!(catalog_extension["version"], "0.1.0");
    assert_eq!(catalog_extension["limitations"], json!([]));
    assert!(catalog_extension["manifest_fingerprint"].is_string());
    assert_eq!(catalog["diagnostic_records"].as_array().unwrap().len(), 1);
    assert!(catalog["diagnostic_records"][0]["manifest_path"]
        .as_str()
        .unwrap()
        .ends_with("invalid/kamafeu-extension.json"));
    assert_eq!(
        extensions::extension_entrypoint_status(&extension),
        "available"
    );
    extension.manifest.entrypoint = Some("missing.wasm".to_string());
    assert_eq!(
        extensions::extension_entrypoint_status(&extension),
        "missing"
    );
    extension.manifest.entrypoint = Some("plugin.txt".to_string());
    assert_eq!(
        extensions::extension_entrypoint_status(&extension),
        "not_wasm"
    );
    extension.manifest.entrypoint = None;
    assert_eq!(
        extensions::extension_entrypoint_status(&extension),
        "not_declared"
    );
}

#[test]
fn converts_between_native_and_interchange_formats() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let aps = directory.path().join("source.aps");
    let ufdata = directory.path().join("converted.ufdata");
    let mut project = UProject {
        name: "CLI conversion".to_string(),
        ..UProject::default()
    };
    let mut note = UNote::new("a", "C4", 0.0, 500.0);
    note.phoneme_overrides
        .push(kamafeu::project::UPhonemeOverride {
            index: 0,
            phoneme: Some("k a".into()),
            ..Default::default()
        });
    project.parts[0].notes.push(note);

    save_project(&project, &aps).expect("save APS");
    let loaded = load_project(&aps).expect("load APS");
    save_project(&loaded, &ufdata).expect("save UFData");
    let converted = load_project(&ufdata).expect("load UFData");

    assert_eq!(converted.parts.len(), 1);
    assert_eq!(converted.parts[0].notes.len(), 1);
    assert_eq!(converted.parts[0].notes[0].lyric, "a");
}

#[test]
fn conversion_reports_source_and_reopened_destination_features() {
    let directory = tempfile::tempdir().expect("temporary conversion directory");
    let input = directory.path().join("source.aps");
    let output = directory.path().join("copy.aps");
    let mut project = UProject::default();
    let mut note = UNote::new("a", "C4", 0.0, 500.0);
    note.phoneme_overrides
        .push(kamafeu::project::UPhonemeOverride {
            index: 0,
            phoneme: Some("k a".into()),
            ..Default::default()
        });
    project.parts[0].notes.push(note);
    save_project(&project, &input).expect("save source");

    let report = conversion::convert_project(&input, &output).expect("convert and inspect");
    assert_eq!(report["source"]["note_count"], 1);
    assert_eq!(report["destination"]["note_count"], 1);
    assert_eq!(report["tracked_features_preserved"], true);
}

#[test]
fn conversion_audit_reports_time_signature_loss() {
    let directory = tempfile::tempdir().expect("temporary meter conversion directory");
    let input = directory.path().join("source.aps");
    let output = directory.path().join("converted.ust");
    let mut project = UProject {
        time_signature_numerator: 3,
        time_signature_denominator: 4,
        ..UProject::default()
    };
    project
        .sections
        .push(kamafeu::project::UProjectSection::new(
            "Verse", 0.0, 1_500.0,
        ));
    project
        .extensions
        .push(kamafeu::project::UProjectExtension {
            id: "org.example.transform".into(),
            version: "1.0.0".into(),
            origin: "extensions/transform/manifest.json".into(),
            manifest_fingerprint: "abc".into(),
        });
    let mut note = UNote::new("a", "C4", 0.0, 500.0);
    note.phoneme_overrides
        .push(kamafeu::project::UPhonemeOverride {
            index: 0,
            phoneme: Some("k a".into()),
            ..Default::default()
        });
    project.parts[0].notes.push(note);
    save_project(&project, &input).expect("save source");

    let report = conversion::convert_project(&input, &output).expect("convert UST");
    assert_eq!(report["source"]["time_signature"], "3/4");
    assert!(report["warnings"]
        .as_array()
        .is_some_and(|warnings| warnings.iter().any(|warning| {
            warning
                .as_str()
                .is_some_and(|warning| warning.contains("compasso"))
        })));
    assert!(report["warnings"]
        .as_array()
        .is_some_and(|warnings| warnings.iter().any(|warning| {
            warning
                .as_str()
                .is_some_and(|warning| warning.contains("seções de projeto"))
        })));
    assert!(report["warnings"]
        .as_array()
        .is_some_and(|warnings| warnings.iter().any(|warning| {
            warning
                .as_str()
                .is_some_and(|warning| warning.contains("overrides persistidos por fonema"))
        })));
    assert!(report["warnings"]
        .as_array()
        .is_some_and(|warnings| warnings.iter().any(|warning| {
            warning
                .as_str()
                .is_some_and(|warning| warning.contains("extensões associadas"))
        })));
}

#[test]
fn conversion_fingerprint_detects_note_content_changes_with_same_count() {
    let mut source = UProject::default();
    source.parts[0]
        .notes
        .push(UNote::new("la", "C4", 0.0, 500.0));
    let mut changed = source.clone();
    changed.parts[0].notes[0].lyric = "li".to_string();

    assert_eq!(
        conversion::ConversionFingerprint::from_project(&source).note_count,
        conversion::ConversionFingerprint::from_project(&changed).note_count
    );
    assert_ne!(
        conversion::ConversionFingerprint::from_project(&source).note_content_signature,
        conversion::ConversionFingerprint::from_project(&changed).note_content_signature
    );
}

#[test]
fn convert_cli_accepts_fail_on_loss_for_ci() {
    let cli = Cli::try_parse_from([
        "kamafeu",
        "convert",
        "--fail-on-loss",
        "source.aps",
        "target.ust",
    ])
    .expect("parse strict conversion command");
    assert!(matches!(
        cli.command,
        Some(Commands::Convert {
            fail_on_loss: true,
            ..
        })
    ));
}

#[test]
fn migrate_voicebank_cli_accepts_profile_mapping_and_destination() {
    let cli = Cli::try_parse_from([
        "kamafeu",
        "migrate-voicebank",
        "/voices/legacy",
        "/voices/native-copy",
        "--profile",
        "/profiles/pt-br.json",
        "--mapping",
        "/profiles/aliases.json",
        "--midi",
        "62",
    ])
    .expect("parse oto.ini migration command");
    assert!(matches!(
        cli.command,
        Some(Commands::MigrateVoicebank {
            midi: 62,
            mapping: Some(_),
            ..
        })
    ));
}

#[test]
fn diff_cli_accepts_two_project_revisions() {
    let cli =
        Cli::try_parse_from(["kamafeu", "diff", "before.aps", "after.aps"]).expect("diff command");
    assert!(matches!(cli.command, Some(Commands::Diff { .. })));
}

#[test]
fn automation_diff_is_read_only_and_structured() {
    let directory = tempfile::tempdir().expect("diff directory");
    let before = directory.path().join("before.aps");
    let after = directory.path().join("after.aps");
    let mut changed = UProject::default();
    changed.parts[0]
        .notes
        .push(UNote::new("a", "C4", 0.0, 400.0));
    save_project(&UProject::default(), &before).expect("before");
    save_project(&changed, &after).expect("after");
    let result = automation::handle_automation_request(AutomationRequest::Diff { before, after })
        .expect("diff response");
    assert_eq!(result["added_notes"], serde_json::json!(1));
    assert!(result["changed_notes"].is_number());
}

#[test]
fn project_validation_includes_persisted_extension_diagnostics() {
    let directory = tempfile::tempdir().expect("validation directory");
    let catalog = directory.path().join("extensions");
    std::fs::create_dir_all(&catalog).expect("catalog");
    std::fs::write(
        catalog.join("kamafeu-extension.json"),
        r#"{
                "id": "org.example.validation",
                "name": "Validation extension",
                "version": "1.0.0",
                "api_version": 1,
                "kind": "analysis"
            }"#,
    )
    .expect("manifest");

    let project_path = directory.path().join("project.aps");
    let mut project = UProject::default();
    project
        .extensions
        .push(kamafeu::project::UProjectExtension {
            id: "org.example.validation".into(),
            version: "1.0.0".into(),
            origin: "extensions/kamafeu-extension.json".into(),
            manifest_fingerprint: "stale".into(),
        });
    save_project(&project, &project_path).expect("project");

    let report =
        conversion::project_validation_json(&project_path, Some(&catalog)).expect("report");
    assert!(report["extension_diagnostics"]
        .as_array()
        .is_some_and(|diagnostics| diagnostics.iter().any(|diagnostic| diagnostic
            .as_str()
            .is_some_and(|value| value.contains("alterado")))));
}

#[test]
fn validate_project_cli_accepts_strict_warning_mode() {
    let cli = Cli::try_parse_from([
        "kamafeu",
        "validate-project",
        "--fail-on-warning",
        "song.aps",
    ])
    .expect("parse strict project validation command");
    assert!(matches!(
        cli.command,
        Some(Commands::ValidateProject {
            fail_on_warning: true,
            ..
        })
    ));
}

#[test]
fn automation_returns_structured_project_information() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let path = directory.path().join("project.aps");
    let mut project = UProject {
        name: "Automation test".to_string(),
        ..UProject::default()
    };
    project.parts[0]
        .notes
        .push(UNote::new("a", "C4", 0.0, 500.0));
    save_project(&project, &path).expect("save project");

    let value = automation::handle_automation_request(AutomationRequest::ProjectInfo { path })
        .expect("automation project info");
    assert_eq!(value["name"], "Automation test");
    assert_eq!(value["note_count"], 1);
}

#[test]
fn voicebank_catalog_discovers_singers_in_explicit_folders() {
    let directory = tempfile::tempdir().expect("temporary singer library");
    let singer = directory.path().join("Test Singer");
    fs::create_dir(&singer).expect("create singer directory");
    fs::write(
        singer.join("character.txt"),
        "name=Automation Singer\nauthor=Kamafeu Tests\n",
    )
    .expect("write singer metadata");

    let catalog = automation::voicebank_catalog_json(vec![directory.path().to_path_buf()], false);
    assert_eq!(catalog["count"], 1);
    assert_eq!(catalog["voicebanks"][0]["name"], "Automation Singer");
    assert_eq!(catalog["voicebanks"][0]["author"], "Kamafeu Tests");
}

#[test]
fn automation_splits_and_applies_lyrics_in_time_order() {
    let directory = tempfile::tempdir().expect("temporary lyric automation directory");
    let input = directory.path().join("source.aps");
    let output = directory.path().join("lyrics.aps");
    let mut project = UProject::default();
    project.parts[0]
        .notes
        .push(UNote::new("old-c", "E4", 1_000.0, 250.0));
    project.parts[0]
        .notes
        .push(UNote::new("old-a", "C4", 0.0, 250.0));
    project.parts[0]
        .notes
        .push(UNote::new("old-b", "D4", 500.0, 250.0));
    save_project(&project, &input).expect("save lyric source");

    let response = automation::handle_automation_request(AutomationRequest::ApplyLyrics {
        input,
        output: output.clone(),
        part_index: 0,
        text: "ka-ma feu".to_string(),
        mode: Some("hyphens_and_spaces".to_string()),
        start_note_index: Some(1),
    })
    .expect("apply lyrics");
    assert_eq!(response["applied_count"], 3);
    let project = load_project(&output).expect("load lyric output");
    assert_eq!(project.parts[0].notes[0].lyric, "feu");
    assert_eq!(project.parts[0].notes[1].lyric, "ka");
    assert_eq!(project.parts[0].notes[2].lyric, "ma");
}

#[test]
fn automation_adds_and_removes_persistent_markers() {
    let directory = tempfile::tempdir().expect("temporary marker automation directory");
    let input = directory.path().join("source.aps");
    let marked = directory.path().join("marked.aps");
    let without_marker = directory.path().join("without-marker.aps");
    save_project(&UProject::default(), &input).expect("save marker source");

    let added = automation::handle_automation_request(AutomationRequest::AddMarker {
        input,
        output: marked.clone(),
        name: "Chorus".to_string(),
        position_ms: 2_000.0,
        color: Some("#00ffaa".to_string()),
    })
    .expect("add marker");
    assert_eq!(added["marker"]["name"], "Chorus");
    assert_eq!(
        load_project(&marked)
            .expect("load marked project")
            .markers
            .len(),
        1
    );

    let removed = automation::handle_automation_request(AutomationRequest::RemoveMarker {
        input: marked,
        output: without_marker.clone(),
        marker_index: 0,
    })
    .expect("remove marker");
    assert_eq!(removed["removed"]["name"], "Chorus");
    assert!(load_project(&without_marker)
        .expect("load marker-free project")
        .markers
        .is_empty());
}

#[test]
fn automation_adds_and_removes_persistent_sections() {
    let directory = tempfile::tempdir().expect("temporary section automation directory");
    let input = directory.path().join("source.aps");
    let sectioned = directory.path().join("sectioned.aps");
    let without_section = directory.path().join("without-section.aps");
    save_project(&UProject::default(), &input).expect("save section source");

    let added = automation::handle_automation_request(AutomationRequest::AddSection {
        input,
        output: sectioned.clone(),
        name: "Chorus".to_string(),
        start_ms: 2_000.0,
        end_ms: 4_000.0,
        color: Some("#00ffaa".to_string()),
    })
    .expect("add section");
    assert_eq!(added["section"]["name"], "Chorus");
    assert_eq!(
        load_project(&sectioned)
            .expect("load sectioned project")
            .sections
            .len(),
        1
    );

    let removed = automation::handle_automation_request(AutomationRequest::RemoveSection {
        input: sectioned,
        output: without_section.clone(),
        section_index: 0,
    })
    .expect("remove section");
    assert_eq!(removed["removed"]["name"], "Chorus");
    assert!(load_project(&without_section)
        .expect("load section-free project")
        .sections
        .is_empty());
}

#[test]
fn automation_can_create_compose_and_retime_a_project() {
    let directory = tempfile::tempdir().expect("temporary automation directory");
    let created = directory.path().join("created.aps");
    let with_note = directory.path().join("with_note.aps");
    let retimed = directory.path().join("retimed.aps");
    let edited = directory.path().join("edited.aps");
    let tuned = directory.path().join("tuned.aps");
    let expressive = directory.path().join("expressive.aps");
    let without_note = directory.path().join("without-note.aps");
    automation::handle_automation_request(AutomationRequest::CreateProject {
        output: created.clone(),
        name: Some("Automation composition".to_string()),
        bpm: Some(120.0),
    })
    .expect("create project");
    automation::handle_automation_request(AutomationRequest::AddNote {
        input: created,
        output: with_note.clone(),
        lyric: "la".to_string(),
        pitch: "C4".to_string(),
        position_ms: 1_000.0,
        duration_ms: 500.0,
        track_index: Some(1),
    })
    .expect("add note");
    let result = automation::handle_automation_request(AutomationRequest::SetTempo {
        input: with_note,
        output: retimed.clone(),
        bpm: 240.0,
    })
    .expect("set tempo");
    assert_eq!(result["time_scale"], 0.5);
    automation::handle_automation_request(AutomationRequest::SetNote {
        input: retimed,
        output: edited.clone(),
        part_index: 1,
        note_index: 0,
        lyric: Some("li".to_string()),
        pitch: Some("D4".to_string()),
    })
    .expect("edit note");
    automation::handle_automation_request(AutomationRequest::SetPitchBend {
        input: edited,
        output: tuned.clone(),
        part_index: 1,
        note_index: 0,
        points: vec![
            AutomationPitchPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: -25.0,
                shape: "s".to_string(),
            },
            AutomationPitchPoint {
                time_offset_ms: 250.0,
                pitch_offset_cents: 0.0,
                shape: "s".to_string(),
            },
        ],
        snap_first: Some(false),
    })
    .expect("tune note");
    let response = automation::handle_automation_request(AutomationRequest::SetExpression {
        input: tuned,
        output: expressive.clone(),
        part_index: 1,
        note_index: 0,
        dynamics: Some(35.0),
        pitch_delta: None,
        gender: Some(-20.0),
        breathiness: Some(45.0),
        velocity: None,
        modulation: Some(25.0),
        volume: Some(120.0),
        attack: None,
        decay: None,
    })
    .expect("set expression");
    assert_eq!(response["expressions"]["dynamics"], 35.0);
    let project = load_project(&expressive).expect("load retimed project");
    assert_eq!(project.bpm, 240.0);
    assert_eq!(project.parts[1].notes[0].position_ms, 500.0);
    assert_eq!(project.parts[1].notes[0].duration_ms, 250.0);
    assert_eq!(project.parts[1].notes[0].lyric, "li");
    assert_eq!(project.parts[1].notes[0].pitch, "D4");
    assert_eq!(project.parts[1].notes[0].pitch_bend.points.len(), 2);
    assert_eq!(project.parts[1].notes[0].expressions.gender, -20.0);
    assert_eq!(project.parts[1].notes[0].expressions.volume, 120.0);

    let removed = automation::handle_automation_request(AutomationRequest::RemoveNote {
        input: expressive,
        output: without_note.clone(),
        part_index: 1,
        note_index: 0,
    })
    .expect("remove note");
    assert_eq!(removed["removed"]["lyric"], "li");
    assert!(load_project(&without_note)
        .expect("load project without note")
        .parts[1]
        .notes
        .is_empty());
}

#[test]
fn render_format_follows_extension_and_rejects_mismatches() {
    assert_eq!(
        RenderOutputFormat::Auto
            .resolve(std::path::Path::new("mix.flac"))
            .expect("auto flac"),
        AudioExportFormat::Flac16
    );
    assert_eq!(
        RenderOutputFormat::Wav24
            .resolve(std::path::Path::new("mix.wav"))
            .expect("24-bit wav"),
        AudioExportFormat::Wav24
    );
    assert!(RenderOutputFormat::Flac24
        .resolve(std::path::Path::new("mix.wav"))
        .is_err());
}

#[test]
fn automation_adds_an_audio_part_to_the_arrangement() {
    let directory = tempfile::tempdir().expect("temporary audio arrangement directory");
    let input = directory.path().join("source.aps");
    let output = directory.path().join("arrangement.aps");
    let audio_path = directory.path().join("backing.wav");
    kamafeu::renderer::TrackRenderer::save_wav_samples(&audio_path, &[0.0, 0.1, -0.1, 0.0], 44_100)
        .expect("backing audio");
    save_project(&UProject::default(), &input).expect("project");

    let result = automation::handle_automation_request(AutomationRequest::AddAudioPart {
        input,
        output: output.clone(),
        file_path: audio_path.clone(),
        position_ms: 250.0,
        track_index: Some(1),
        name: Some("Backing".to_string()),
        volume_db: Some(-3.0),
    })
    .expect("add audio part");
    assert_eq!(result["wave_part_count"], 1);
    let project = load_project(&output).expect("arrangement");
    assert_eq!(project.wave_parts.len(), 1);
    assert_eq!(project.wave_parts[0].name, "Backing");
    assert_eq!(project.wave_parts[0].track_index, 1);
    assert_eq!(project.wave_parts[0].position_ms, 250.0);
    assert_eq!(project.wave_parts[0].volume_db, -3.0);
    assert_eq!(
        project.wave_parts[0].file_path,
        audio_path.to_string_lossy()
    );
}

#[test]
fn mcp_exposes_and_executes_kamafeu_tools() {
    assert!(mcp::mcp_tools()
        .iter()
        .any(|tool| tool["name"] == "kamafeu_render"));
    let response = mcp::mcp_tool_call(
        json!(42),
        json!({ "name": "kamafeu_cache_info", "arguments": {} }),
    );
    assert_eq!(response["id"], 42);
    assert_eq!(response["result"]["isError"], false);
    assert!(response["result"]["structuredContent"]["directory"].is_string());
}

#[test]
fn headless_render_writes_audio_inferred_from_output_extension() {
    let directory = tempfile::tempdir().expect("temporary render directory");
    let voicebank = directory.path().join("voicebank");
    fs::create_dir_all(&voicebank).expect("voicebank directory");
    let samples = (0..22_050)
        .map(|index| (index as f32 * std::f32::consts::TAU * 220.0 / 44_100.0).sin() * 0.2)
        .collect::<Vec<_>>();
    kamafeu::renderer::TrackRenderer::save_wav_samples(
        voicebank.join("source.wav"),
        &samples,
        44_100,
    )
    .expect("source WAV");
    fs::write(voicebank.join("oto.ini"), "source.wav=a,0,50,-450,40,10\n").expect("oto");

    let input = directory.path().join("project.aps");
    let output = directory.path().join("render.flac");
    let mut project = UProject {
        flags: "P86g-4Xcustom".to_string(),
        ..UProject::default()
    };
    project
        .extensions
        .push(kamafeu::project::model::UProjectExtension {
            id: "org.example.render-engine".to_string(),
            version: "2.0.0".to_string(),
            origin: "extensions/render-engine/kamafeu-extension.json".to_string(),
            manifest_fingerprint: "manifest-test".to_string(),
        });
    project.parts[0]
        .notes
        .push(UNote::new("a", "C4", 0.0, 350.0));
    save_project(&project, &input).expect("project");

    let summary = render::render_project_to_file(
        &voicebank,
        &input,
        &output,
        RenderOutputFormat::Auto,
        true,
        44_100,
        false,
        None,
        false,
    )
    .expect("render");
    assert_eq!(summary.format, AudioExportFormat::Flac16);
    assert!(summary.frame_count > 0);
    assert!(summary.diagnostics.is_safe());
    assert!(!summary.silent);
    assert_eq!(fs::read(&output).expect("FLAC")[..4], *b"fLaC");
    let provenance =
        fs::read_to_string(output.with_extension("flac.kamafeu.json")).expect("render provenance");
    assert!(provenance.contains("P86g-4Xcustom"));
    assert!(provenance.contains("voicebank_wav"));
    assert!(provenance.contains("timing_diagnostics"));
    let provenance_json: serde_json::Value =
        serde_json::from_str(&provenance).expect("provenance JSON");
    let commands = provenance_json["commands"].as_array().expect("commands");
    assert!(commands.len() >= 2);
    assert!(commands.iter().all(serde_json::Value::is_string));
    assert!(
        provenance_json["cache_hits"].as_u64().unwrap_or(0)
            + provenance_json["cache_misses"].as_u64().unwrap_or(0)
            >= 1
    );
    assert!(provenance.contains("oto_preutter_ms"));
    assert!(provenance.contains("extension_details"));
    assert!(provenance.contains("org.example.render-engine"));
    assert!(provenance.contains("manifest-test"));
}
