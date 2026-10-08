use clap::{Parser, Subcommand};
use eframe::NativeOptions;
use serde_json::{json, Value};
use std::fs;
use std::io::BufRead;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use kamafeu::{
    drivers::{
        NativeResamplerDriver, NativeWavtoolDriver, NativeWorldResamplerDriver, ResamplerDriver,
        WavtoolDriver,
    },
    extensions::{
        built_in_capabilities, default_project_format_registry, diagnose_project_extensions,
        diagnose_project_extensions_structured, discover, platform_capability_matrix,
        verify_wasm_extension_with_policy, DiscoveredExtension, ExtensionPermissionPolicy,
    },
    gui::KamafeuStudioApp,
    oto::{SingerScanner, Voicebank},
    project::model::{UNote, UProject},
    renderer::{
        AudioDiagnostics, AudioExportFormat, AudioExporter, DitherMode, ProjectRenderer,
        RenderOptions, RenderProvenance,
    },
};

#[derive(Parser)]
#[command(name = "kamafeu")]
#[command(about = "OpenUTAU-style voice synthesizer core and Piano Roll GUI in Rust", long_about = None)]
struct Cli {
    /// Emit machine-readable JSON for inspection and validation commands
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the OpenUTAU-style Piano Roll GUI Studio
    Gui,

    /// Inspect information about an UTAU voicebank directory
    VoicebankInfo {
        /// Path to the UTAU voicebank root folder
        path: PathBuf,

        /// Resolve one alias through prefix/suffix mapping and report its oto.ini entry
        #[arg(long)]
        alias: Option<String>,

        /// Pitch used when resolving `--alias`
        #[arg(long, default_value = "C4")]
        pitch: String,
    },

    /// List UTAU, OpenUTAU and DiffSinger voicebanks found in singer folders
    Voicebanks {
        /// Singer folder to scan. May be specified more than once.
        #[arg(short = 'p', long = "path")]
        paths: Vec<PathBuf>,

        /// Also scan the standard OpenUTAU singer folders for this system
        #[arg(long)]
        include_default: bool,
    },

    /// Validate WAV files and oto.ini timing in an UTAU voicebank
    ValidateVoicebank {
        /// Path to the UTAU voicebank root folder
        path: PathBuf,

        /// For DiffSinger, load ONNX sessions and validate the model contract
        #[arg(long)]
        verify_runtime: bool,
    },

    /// Print project metadata, tracks and note counts without opening the GUI
    ProjectInfo {
        /// Project or score file (.aps, .ustx, .ust, .mid, .midi, .vsqx, .svp, .ufdata, .json)
        path: PathBuf,
    },

    /// Compare two project revisions without modifying either input
    Diff {
        /// Earlier project revision
        before: PathBuf,
        /// Later project revision
        after: PathBuf,
    },

    /// Inspect a render provenance sidecar without opening the project
    ProvenanceInfo {
        /// Path to a *.kamafeu.json sidecar
        path: PathBuf,
    },

    /// Validate project structure before rendering or committing an import
    ValidateProject {
        /// Project or score file to validate
        path: PathBuf,

        /// Also return a failure status for warnings, useful in CI before rendering
        #[arg(long)]
        fail_on_warning: bool,

        /// Optional extension catalog used to validate plugins persisted in APS
        #[arg(long)]
        extensions_path: Option<PathBuf>,
    },

    /// Convert a project between supported score formats without opening the GUI
    Convert {
        /// Source project or score file
        input: PathBuf,

        /// Destination file (.aps, .ustx, .ust, .mid, .midi, .vsqx, .svp, .ufdata, .json)
        output: PathBuf,

        /// Return a failure status when the post-conversion audit detects changed musical data
        #[arg(long)]
        fail_on_loss: bool,
    },

    /// List and validate extension manifests without executing their code
    Extensions {
        /// Folder containing extension directories
        #[arg(default_value = "extensions")]
        path: PathBuf,

        /// Start each declared WASM module inside the isolated compatibility runtime
        #[arg(long)]
        verify_wasm: bool,

        /// Explicitly grant a manifest permission for this verification run.
        #[arg(long = "grant-permission")]
        grant_permissions: Vec<String>,

        /// Show only extensions declaring this format
        #[arg(long)]
        format_filter: Option<String>,

        /// Show only extensions declaring this phonemizer
        #[arg(long)]
        phonemizer_filter: Option<String>,

        /// Explicitly execute one discovered WASM extension by manifest ID
        #[arg(long)]
        invoke_id: Option<String>,

        /// Export name used with --invoke-id
        #[arg(long, default_value = "analyze")]
        invoke_export: String,

        /// Integer argument passed to the selected export
        #[arg(long, default_value_t = 0)]
        invoke_argument: i32,

        /// Read the selected export as the bounded pointer/length bytes ABI
        #[arg(long)]
        invoke_bytes: bool,

        /// JSON parameter passed to the plugin (repeatable: key=value)
        #[arg(long = "invoke-parameter")]
        invoke_parameters: Vec<String>,
    },

    /// Report the checked-in feature matrix for desktop, Android and WASM
    PlatformInfo,

    /// Show render-cache location, size and hit statistics
    CacheInfo,

    /// Remove persistent render-cache entries older than the chosen age
    CachePrune {
        /// Age threshold in days
        #[arg(long, default_value_t = 30)]
        days: u32,
    },

    /// Run a local JSON Lines automation protocol on standard input/output
    Automation,

    /// Run a local Model Context Protocol server on standard input/output
    Mcp,

    /// Generate a test UTAU voicebank with sample WAVs and oto.ini for quick testing
    GenSample {
        /// Destination directory to create the sample voicebank
        path: PathBuf,
    },

    /// Render an APS, UST, USTX, MIDI, VSQX, SVP, UFData, or JSON project to audio
    Render {
        /// Path to UTAU voicebank directory
        #[arg(short, long)]
        voicebank: PathBuf,

        /// Path to project or score file (.aps, .ustx, .ust, .mid, .midi, .vsqx, .svp, .ufdata, .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Path to output audio file (.wav, .flac, .raw)
        #[arg(short, long, default_value = "output.wav")]
        output: PathBuf,

        /// Output encoding. `auto` selects from the output extension.
        #[arg(long, value_enum, default_value_t = RenderOutputFormat::Auto)]
        format: RenderOutputFormat,

        /// Apply deterministic TPDF dither when writing integer PCM
        #[arg(long)]
        dither: bool,

        /// Sample rate (Hz)
        #[arg(short, long, default_value_t = 44100)]
        sample_rate: u32,

        /// Use the native WORLD resampler instead of the classic engine
        #[arg(long)]
        venus: bool,

        /// Ignore instrumental wave parts while diagnosing vocal rendering
        #[arg(long)]
        vocal_only: bool,
    },
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum RenderOutputFormat {
    Auto,
    Wav16,
    Wav24,
    Wav32Float,
    Flac16,
    Flac24,
    RawF32,
}

impl RenderOutputFormat {
    fn resolve(self, output: &std::path::Path) -> Result<AudioExportFormat, String> {
        let format = match self {
            Self::Auto => AudioExportFormat::from_extension(
                output
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("wav"),
            ),
            Self::Wav16 => AudioExportFormat::Wav16,
            Self::Wav24 => AudioExportFormat::Wav24,
            Self::Wav32Float => AudioExportFormat::Wav32Float,
            Self::Flac16 => AudioExportFormat::Flac16,
            Self::Flac24 => AudioExportFormat::Flac24,
            Self::RawF32 => AudioExportFormat::RawF32,
        };
        if self != Self::Auto {
            let extension = output
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !extension.is_empty() && extension != format.extension() {
                return Err(format!(
                    "a extensão .{extension} não corresponde ao formato {}; use .{} ou --format auto",
                    format.display_name(),
                    format.extension()
                ));
            }
        }
        Ok(format)
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum AutomationRequest {
    CreateProject {
        output: PathBuf,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        bpm: Option<f64>,
    },
    ProjectInfo {
        path: PathBuf,
    },
    Diff {
        before: PathBuf,
        after: PathBuf,
    },
    ListVoicebanks {
        #[serde(default)]
        paths: Option<Vec<PathBuf>>,
        #[serde(default)]
        include_default: bool,
    },
    ValidateProject {
        path: PathBuf,
        #[serde(default)]
        extensions_path: Option<PathBuf>,
    },
    Convert {
        input: PathBuf,
        output: PathBuf,
    },
    AddNote {
        input: PathBuf,
        output: PathBuf,
        lyric: String,
        pitch: String,
        position_ms: f64,
        duration_ms: f64,
        #[serde(default)]
        track_index: Option<usize>,
    },
    AddAudioPart {
        input: PathBuf,
        output: PathBuf,
        file_path: PathBuf,
        position_ms: f64,
        #[serde(default)]
        track_index: Option<usize>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        volume_db: Option<f64>,
    },
    SetTempo {
        input: PathBuf,
        output: PathBuf,
        bpm: f64,
    },
    SetNote {
        input: PathBuf,
        output: PathBuf,
        part_index: usize,
        note_index: usize,
        #[serde(default)]
        lyric: Option<String>,
        #[serde(default)]
        pitch: Option<String>,
    },
    RemoveNote {
        input: PathBuf,
        output: PathBuf,
        part_index: usize,
        note_index: usize,
    },
    ApplyLyrics {
        input: PathBuf,
        output: PathBuf,
        part_index: usize,
        text: String,
        /// spaces, hyphens_and_spaces, japanese_syllables or portuguese_syllables
        #[serde(default)]
        mode: Option<String>,
        /// Note index that receives the first lyric token. Subsequent notes follow time order.
        #[serde(default)]
        start_note_index: Option<usize>,
    },
    AddMarker {
        input: PathBuf,
        output: PathBuf,
        name: String,
        position_ms: f64,
        #[serde(default)]
        color: Option<String>,
    },
    RemoveMarker {
        input: PathBuf,
        output: PathBuf,
        marker_index: usize,
    },
    AddSection {
        input: PathBuf,
        output: PathBuf,
        name: String,
        start_ms: f64,
        end_ms: f64,
        #[serde(default)]
        color: Option<String>,
    },
    RemoveSection {
        input: PathBuf,
        output: PathBuf,
        section_index: usize,
    },
    SetPitchBend {
        input: PathBuf,
        output: PathBuf,
        part_index: usize,
        note_index: usize,
        points: Vec<AutomationPitchPoint>,
        #[serde(default)]
        snap_first: Option<bool>,
    },
    SetExpression {
        input: PathBuf,
        output: PathBuf,
        part_index: usize,
        note_index: usize,
        #[serde(default)]
        dynamics: Option<f64>,
        #[serde(default)]
        pitch_delta: Option<f64>,
        #[serde(default)]
        gender: Option<f64>,
        #[serde(default)]
        breathiness: Option<f64>,
        #[serde(default)]
        velocity: Option<f64>,
        #[serde(default)]
        modulation: Option<f64>,
        #[serde(default)]
        volume: Option<f64>,
        #[serde(default)]
        attack: Option<f64>,
        #[serde(default)]
        decay: Option<f64>,
    },
    Render {
        voicebank: PathBuf,
        input: PathBuf,
        output: PathBuf,
        #[serde(default)]
        sample_rate: Option<u32>,
        #[serde(default)]
        venus: bool,
        #[serde(default)]
        vocal_only: bool,
        #[serde(default)]
        dither: bool,
    },
    ValidateVoicebank {
        path: PathBuf,
        #[serde(default)]
        verify_runtime: bool,
    },
    Extensions {
        path: Option<PathBuf>,
        #[serde(default)]
        verify_wasm: bool,
        #[serde(default)]
        grant_permissions: Vec<String>,
        #[serde(default)]
        format_filter: Option<String>,
        #[serde(default)]
        phonemizer_filter: Option<String>,
        #[serde(default)]
        invoke_id: Option<String>,
        #[serde(default)]
        invoke_export: Option<String>,
        #[serde(default)]
        invoke_argument: Option<i32>,
        #[serde(default)]
        invoke_bytes: bool,
        #[serde(default)]
        invoke_parameters: serde_json::Map<String, Value>,
    },
    CacheInfo,
    CachePrune {
        days: Option<u32>,
    },
}

#[derive(serde::Deserialize)]
struct AutomationPitchPoint {
    time_offset_ms: f64,
    pitch_offset_cents: f64,
    #[serde(default = "default_pitch_shape")]
    shape: String,
}

fn default_pitch_shape() -> String {
    "s".to_string()
}

fn automation_lyrics_split_mode(
    mode: Option<&str>,
) -> Result<kamafeu::gui::lyrics_dialog::SplitMode, Box<dyn std::error::Error>> {
    use kamafeu::gui::lyrics_dialog::SplitMode;

    match mode.unwrap_or("hyphens_and_spaces").trim().to_ascii_lowercase().as_str() {
        "spaces" | "space" | "words" => Ok(SplitMode::Spaces),
        "hyphens_and_spaces" | "hyphens" | "hyphen" | "default" => {
            Ok(SplitMode::HyphensAndSpaces)
        }
        "japanese_syllables" | "japanese" | "ja" => Ok(SplitMode::JapaneseSyllables),
        "portuguese_syllables" | "portuguese" | "pt" => Ok(SplitMode::PortugueseSyllables),
        value => Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "mode inválido '{value}'; use spaces, hyphens_and_spaces, japanese_syllables ou portuguese_syllables"
            ),
        )
        .into()),
    }
}

fn main() {
    if let Err(error) = real_main() {
        #[cfg(target_os = "windows")]
        {
            let _ = rfd::MessageDialog::new()
                .set_title("Kamafeu Studio")
                .set_description(format!("O Kamafeu não pôde iniciar:\n\n{error}"))
                .show();
        }
        eprintln!("Kamafeu: {error}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli {
        command,
        json: json_output,
    } = Cli::parse();

    match command {
        None | Some(Commands::Gui) => {
            println!(
                "Starting Kamafeu Studio v{} (Bariloche)...",
                kamafeu::APP_VERSION
            );
            kamafeu::drivers::process::prewarm_wine_in_background();
            let icon_data = kamafeu::gui::window_icon::load_window_icon().ok();
            let mut viewport = eframe::egui::ViewportBuilder::default()
                .with_title(format!(
                    "Kamafeu Studio v{} (Bariloche)",
                    kamafeu::APP_VERSION
                ))
                .with_inner_size([1280.0, 750.0])
                .with_min_inner_size([800.0, 500.0])
                .with_maximized(true);

            if let Some(icon) = icon_data {
                viewport = viewport.with_icon(icon);
            }

            let options = NativeOptions {
                viewport,
                ..Default::default()
            };

            eframe::run_native(
                &format!("Kamafeu Studio v{} (Bariloche)", kamafeu::APP_VERSION),
                options,
                Box::new(|cc| Ok(Box::new(KamafeuStudioApp::new(cc)))),
            )?;
        }

        Some(Commands::VoicebankInfo { path, alias, pitch }) => {
            let vb = Voicebank::new(&path)?;
            if vb.is_diffsinger() {
                let config = vb.diffsinger_config().map_err(Error::other)?;
                if json_output {
                    print_json(&diffsinger_info_json(&path, &vb, &config))?;
                } else {
                    println!("=== DiffSinger voicebank ===");
                    println!("Name:          {}", vb.name);
                    println!("Root:          {}", config.root.display());
                    println!("Acoustic:      {}", config.acoustic.display());
                    println!("Vocoder:       {}", config.vocoder.display());
                    println!("Sample rate:   {} Hz", config.sample_rate);
                    println!("Hop size:      {}", config.hop_size);
                    println!("Mel bins:      {}", config.num_mel_bins);
                    println!("Speakers:      {}", config.speakers.join(", "));
                }
                return Ok(());
            }
            if json_output {
                let aliases = vb
                    .entries
                    .iter()
                    .take(10)
                    .map(|(alias, entry)| {
                        json!({
                            "alias": alias,
                            "wav": entry.wav_filename,
                            "offset_ms": entry.offset,
                            "preutterance_ms": entry.preutterance,
                        })
                    })
                    .collect::<Vec<_>>();
                let resolution = alias.as_deref().map(|alias| {
                    serde_json::to_value(vb.alias_resolution_report(alias, &pitch))
                        .expect("alias resolution report is serializable")
                });
                print_json(&json!({
                    "path": path,
                    "name": vb.name,
                    "author": vb.author,
                    "entry_count": vb.entries.len(),
                    "aliases": aliases,
                    "resolution": resolution,
                }))?;
            } else {
                println!("Loading voicebank from: {:?}", path);
                println!("=== Voicebank Info ===");
                println!("Name:        {}", vb.name);
                println!("Author:      {}", vb.author);
                println!("Total entries: {}", vb.entries.len());
                println!("\nSample Aliases (up to 10):");
                for (alias, entry) in vb.entries.iter().take(10) {
                    println!(
                        "  - Alias: {:<10} -> WAV: {:<12} (offset: {}ms, preutter: {}ms)",
                        alias, entry.wav_filename, entry.offset, entry.preutterance
                    );
                }
                if let Some(alias) = alias {
                    let report = vb.alias_resolution_report(alias, &pitch);
                    match report.resolved_alias {
                        Some(resolved) => println!(
                            "\nResolution: '{}' @ {} -> '{}' / {} (preutter: {}ms, overlap: {}ms)",
                            report.requested_alias,
                            report.pitch,
                            resolved,
                            report.wav_filename.unwrap_or_default(),
                            report.preutterance_ms.unwrap_or_default(),
                            report.overlap_ms.unwrap_or_default()
                        ),
                        None => println!(
                            "\nResolution: '{}' @ {} -> {}",
                            report.requested_alias,
                            report.pitch,
                            report.reason.unwrap_or_else(|| "não resolvido".to_string())
                        ),
                    }
                }
            }
        }

        Some(Commands::Voicebanks {
            paths,
            include_default,
        }) => {
            let catalog = voicebank_catalog_json(paths, include_default);
            if json_output {
                print_json(&catalog)?;
            } else {
                let directories = catalog["directories"].as_array();
                let singers = catalog["voicebanks"].as_array();
                println!("=== Voicebank catalog ===");
                println!("Directories scanned: {}", directories.map_or(0, Vec::len));
                if let Some(directories) = directories {
                    for directory in directories {
                        if let Some(path) = directory.as_str() {
                            println!("  - {path}");
                        }
                    }
                }
                println!("Voicebanks found: {}", singers.map_or(0, Vec::len));
                if let Some(singers) = singers {
                    for singer in singers {
                        println!(
                            "  - {} ({}, {})\n    {}",
                            singer["name"].as_str().unwrap_or("Unknown"),
                            singer["voice_type"].as_str().unwrap_or("Unknown"),
                            singer["author"].as_str().unwrap_or("Unknown"),
                            singer["path"].as_str().unwrap_or(""),
                        );
                    }
                }
            }
        }

        Some(Commands::ValidateVoicebank {
            path,
            verify_runtime,
        }) => {
            let vb = Voicebank::new(&path)?;
            if vb.is_diffsinger() {
                let config = vb.diffsinger_config().map_err(Error::other)?;
                let report = diffsinger_validation_json(&path, &vb, &config, verify_runtime)?;
                if json_output {
                    print_json(&report)?;
                } else {
                    println!("=== DiffSinger validation ===");
                    println!("Name:        {}", vb.name);
                    println!("Acoustic:    {}", config.acoustic.display());
                    println!("Vocoder:     {}", config.vocoder.display());
                    println!(
                        "Result: {}",
                        if verify_runtime {
                            "valid configuration and ONNX contract"
                        } else {
                            "valid configuration (use --verify-runtime to load ONNX models)"
                        }
                    );
                }
                return Ok(());
            }
            let report = vb.diagnostic_report();
            if json_output {
                let issues = report
                    .issues
                    .iter()
                    .map(|issue| {
                        json!({
                            "alias": issue.alias,
                            "wav": issue.wav_filename,
                            "detail": issue.detail,
                            "kind": format!("{:?}", issue.kind),
                            "suggestion": issue.suggestion(),
                        })
                    })
                    .collect::<Vec<_>>();
                print_json(&json!({
                    "path": path,
                    "name": vb.name,
                    "healthy": report.is_healthy(),
                    "entries": report.entry_count,
                    "unique_wavs": report.unique_wav_count,
                    "readable_wavs": report.readable_wav_count,
                    "missing_wavs": report.missing_wav_count,
                    "invalid_wavs": report.invalid_wav_count,
                    "empty_wavs": report.empty_wav_count,
                    "invalid_oto_timing": report.invalid_oto_timing_count,
                    "sample_rates": report.sample_rates,
                    "issues": issues,
                }))?;
            } else {
                println!("=== Voicebank validation ===");
                println!("Name:              {}", vb.name);
                println!("Entries:           {}", report.entry_count);
                println!("Unique WAV files:  {}", report.unique_wav_count);
                println!("Readable WAV files:{}", report.readable_wav_count);
                println!("Missing WAV files: {}", report.missing_wav_count);
                println!("Invalid WAV files: {}", report.invalid_wav_count);
                println!("Empty WAV files:   {}", report.empty_wav_count);
                println!("Invalid oto timing:{}", report.invalid_oto_timing_count);
                if report.sample_rates.is_empty() {
                    println!("Sample rates:      none");
                } else {
                    let sample_rates = report
                        .sample_rates
                        .iter()
                        .map(|(rate, count)| format!("{rate} Hz ({count})"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("Sample rates:      {sample_rates}");
                }
                if report.is_healthy() {
                    println!("Result: healthy");
                } else {
                    println!("Result: {} issue(s)", report.issues.len());
                    for issue in &report.issues {
                        println!(
                            "  - [{}] {}: {}",
                            issue.alias, issue.wav_filename, issue.detail
                        );
                        println!("      sugestão: {}", issue.suggestion());
                    }
                }
            }
            if !report.is_healthy() {
                return Err(
                    Error::new(ErrorKind::InvalidData, "voicebank validation failed").into(),
                );
            }
        }

        Some(Commands::ProjectInfo { path }) => {
            let project = load_project(&path)?;
            let note_count: usize = project.parts.iter().map(|part| part.notes.len()).sum();
            let duration_ms = project
                .parts
                .iter()
                .flat_map(|part| {
                    part.notes.iter().map(move |note| {
                        part.position_ms + note.position_ms + note.duration_ms.max(0.0)
                    })
                })
                .chain(
                    project
                        .wave_parts
                        .iter()
                        .map(|wave| wave.position_ms + wave.duration_ms.max(0.0)),
                )
                .fold(0.0_f64, f64::max);
            let tracks = project
                .tracks
                .iter()
                .enumerate()
                .map(|(index, track)| {
                    let track_notes: usize = project
                        .parts
                        .iter()
                        .filter(|part| part.track_index == index)
                        .map(|part| part.notes.len())
                        .sum();
                    (index, &track.name, track_notes)
                })
                .collect::<Vec<_>>();
            if json_output {
                print_json(&json!({
                    "path": path,
                    "name": project.name,
                    "bpm": project.bpm,
                    "time_signature": format!("{}/{}", project.time_signature_numerator, project.time_signature_denominator),
                    "track_count": project.tracks.len(),
                    "voice_part_count": project.parts.len(),
                    "wave_part_count": project.wave_parts.len(),
                    "section_count": project.sections.len(),
                    "note_count": note_count,
                    "duration_ms": duration_ms,
                    "tracks": tracks.iter().map(|(index, name, notes)| json!({
                        "index": index,
                        "name": name,
                        "note_count": notes,
                    })).collect::<Vec<_>>(),
                }))?;
            } else {
                println!("=== Project info ===");
                println!("Name:        {}", project.name);
                println!("Tempo:       {:.2} BPM", project.bpm);
                println!(
                    "Compasso:    {}/{}",
                    project.time_signature_numerator, project.time_signature_denominator
                );
                println!("Tracks:      {}", project.tracks.len());
                println!("Voice parts: {}", project.parts.len());
                println!("Wave parts:  {}", project.wave_parts.len());
                println!("Sections:    {}", project.sections.len());
                println!("Notes:       {note_count}");
                println!("Duration:    {:.2}s", duration_ms / 1000.0);
                for (index, name, track_notes) in tracks {
                    println!("  {}. {} ({} notes)", index + 1, name, track_notes);
                }
            }
        }

        Some(Commands::Diff { before, after }) => {
            let before_project = load_project(&before)?;
            let after_project = load_project(&after)?;
            let diff = kamafeu::project::ProjectDiff::between(&before_project, &after_project);
            if json_output {
                print_json(&serde_json::to_value(&diff)?)?;
            } else {
                println!("=== Project diff ===");
                println!("Before: {}", before.display());
                println!("After:  {}", after.display());
                println!(
                    "Project metadata changed: {}",
                    diff.changed_project_metadata
                );
                println!("Arrangement changed:       {}", diff.changed_arrangement);
                println!(
                    "Render settings changed:   {}",
                    diff.changed_render_settings
                );
                println!("Notes added:              {}", diff.added_notes);
                println!("Notes removed:            {}", diff.removed_notes);
                println!("Notes changed:            {}", diff.changed_notes);
                println!(
                    "Phoneme overrides changed:{}",
                    diff.changed_phoneme_overrides
                );
                println!("Wave parts added:         {}", diff.added_wave_parts);
                println!("Wave parts removed:       {}", diff.removed_wave_parts);
                println!("Wave parts changed:       {}", diff.changed_wave_parts);
                println!(
                    "Result: {}",
                    if diff.is_empty() {
                        "identical"
                    } else {
                        "changed"
                    }
                );
            }
        }

        Some(Commands::ProvenanceInfo { path }) => {
            let contents = fs::read_to_string(&path)?;
            let provenance: RenderProvenance = serde_json::from_str(&contents)
                .map_err(|error| Error::new(ErrorKind::InvalidData, error))?;
            if json_output {
                print_json(&serde_json::to_value(&provenance)?)?;
            } else {
                println!("=== Render provenance ===");
                println!("Pipeline:       {}", provenance.pipeline_version);
                println!("Schema:         {}", provenance.schema);
                println!("Resampler:      {}", provenance.resampler);
                println!("Wavtool:        {}", provenance.wavtool);
                println!("Cache hit:      {}", provenance.cache_hit);
                println!("Artifacts:      {}", provenance.artifacts.len());
                println!("Timing records: {}", provenance.timing_diagnostics.len());
                println!(
                    "Timing boundaries: {}",
                    provenance.timing_boundary_diagnostics.len()
                );
                println!("Warnings:       {}", provenance.warnings.len());
            }
        }

        Some(Commands::ValidateProject {
            path,
            fail_on_warning,
            extensions_path,
        }) => {
            let report = project_validation_json(&path, extensions_path.as_ref())?;
            if json_output {
                print_json(&report)?;
            } else {
                println!("=== Project validation ===");
                println!("Path: {}", path.display());
                println!("Tracks:      {}", report["track_count"]);
                println!("Voice parts: {}", report["voice_part_count"]);
                println!("Wave parts:  {}", report["wave_part_count"]);
                println!("Notes:       {}", report["note_count"]);
                if report["valid"].as_bool() == Some(true) {
                    println!("Result: valid");
                } else {
                    println!("Result: invalid");
                    for issue in report["issues"].as_array().into_iter().flatten() {
                        println!(
                            "  - [{}] {}: {}",
                            issue["severity"].as_str().unwrap_or("unknown"),
                            issue["location"].as_str().unwrap_or("project"),
                            issue["detail"].as_str().unwrap_or("unknown issue")
                        );
                        if let Some(suggestion) = issue["suggestion"].as_str() {
                            println!("      sugestão: {suggestion}");
                        }
                    }
                }
                for diagnostic in report["extension_diagnostics"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    println!("  - [warning] extension: {}", diagnostic);
                }
            }
            let has_warnings = report["issues"].as_array().is_some_and(|issues| {
                issues
                    .iter()
                    .any(|issue| issue["severity"].as_str() == Some("warning"))
            }) || report["extension_diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty());
            if report["valid"].as_bool() != Some(true) || (fail_on_warning && has_warnings) {
                return Err(Error::new(ErrorKind::InvalidData, "project validation failed").into());
            }
        }

        Some(Commands::Convert {
            input,
            output,
            fail_on_loss,
        }) => {
            let report = convert_project(&input, &output)?;
            if json_output {
                print_json(&report)?;
            } else {
                println!("Converted {} to {}", input.display(), output.display());
                let warnings = report["warnings"].as_array().cloned().unwrap_or_default();
                if warnings.is_empty() {
                    println!("Tracked musical data preserved after reopening the destination.");
                } else {
                    println!("Conversion warnings:");
                    for warning in warnings {
                        if let Some(warning) = warning.as_str() {
                            println!("  - {warning}");
                        }
                    }
                }
            }
            if fail_on_loss && report["tracked_features_preserved"].as_bool() != Some(true) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "a auditoria detectou dados musicais alterados; o arquivo de destino foi criado, mas a conversão falhou em modo --fail-on-loss",
                )
                .into());
            }
        }

        Some(Commands::Extensions {
            path,
            verify_wasm,
            grant_permissions,
            format_filter,
            phonemizer_filter,
            invoke_id,
            invoke_export,
            invoke_argument,
            invoke_bytes,
            invoke_parameters,
        }) => {
            let discovery = discover(&path);
            let built_ins = built_in_capabilities();
            let policy = permission_policy(grant_permissions);
            if let Some(invoke_id) = invoke_id {
                let extension = discovery
                    .extensions
                    .iter()
                    .find(|extension| extension.manifest.id == invoke_id)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::NotFound,
                            format!("extensão não encontrada: {invoke_id}"),
                        )
                    })?;
                let parameters = parse_extension_parameters(&invoke_parameters)?;
                if invoke_bytes {
                    let output = kamafeu::extensions::invoke_wasm_bytes(
                        extension,
                        &policy,
                        &invoke_export,
                        invoke_argument,
                        &parameters,
                    )
                    .map_err(Error::other)?;
                    print_json(&json!({
                        "id": invoke_id,
                        "export": invoke_export,
                        "argument": invoke_argument,
                        "bytes_length": output.len(),
                        "utf8": String::from_utf8_lossy(&output),
                        "hex": output.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
                    }))?;
                } else {
                    let output = kamafeu::extensions::invoke_wasm_i32_with_parameters(
                        extension,
                        &policy,
                        &invoke_export,
                        invoke_argument,
                        &parameters,
                    )
                    .map_err(Error::other)?;
                    print_json(&json!({
                        "id": invoke_id,
                        "export": invoke_export,
                        "argument": invoke_argument,
                        "result": output,
                    }))?;
                }
                return Ok(());
            }
            let matches_filter = |extension: &&DiscoveredExtension| {
                format_filter
                    .as_deref()
                    .is_none_or(|value| extension.manifest.formats.contains(value))
                    && phonemizer_filter
                        .as_deref()
                        .is_none_or(|value| extension.manifest.phonemizers.contains(value))
            };
            let wasm_verification =
                verify_wasm.then(|| wasm_verification_json(&discovery.extensions, &policy));
            if json_output {
                print_json(&json!({
                    "path": path,
                    "built_in_capabilities": built_ins.iter().map(|capability| json!({
                        "id": capability.id,
                        "name": capability.name,
                        "kind": capability.kind,
                        "description": capability.description,
                    })).collect::<Vec<_>>(),
                    "format_filter": format_filter,
                    "phonemizer_filter": phonemizer_filter,
                    "extensions": discovery.extensions.iter().filter(matches_filter).map(|extension| json!({
                        "manifest_path": extension.manifest_path,
                        "manifest_fingerprint": extension
                            .render_identity()
                            .ok()
                            .map(|identity| identity.manifest_fingerprint),
                        "id": extension.manifest.id,
                        "name": extension.manifest.name,
                        "version": extension.manifest.version,
                        "api_version": extension.manifest.api_version,
                        "kind": extension.manifest.kind,
                        "capabilities": extension.manifest.capabilities,
                        "formats": extension.manifest.formats,
                        "phonemizers": extension.manifest.phonemizers,
                        "permissions": extension.manifest.permissions,
                        "platforms": extension.manifest.platforms,
                        "limitations": extension.manifest.limitations,
                        "parameters": extension.manifest.parameters,
                        "entrypoint": extension.manifest.entrypoint,
                        "entrypoint_status": extension_entrypoint_status(extension),
                        "platform_compatible": extension
                            .manifest
                            .supports_platform(kamafeu::extensions::current_platform()),
                        "description": extension.manifest.description,
                    })).collect::<Vec<_>>(),
                    "diagnostics": discovery.diagnostics,
                    "diagnostic_records": discovery.diagnostic_records,
                    "wasm_verification": wasm_verification,
                }))?;
            } else {
                println!("=== Kamafeu extensions ===");
                println!("Directory: {}", path.display());
                println!("Built-in capabilities:");
                for capability in built_ins {
                    println!(
                        "- {} ({:?}) [{}]",
                        capability.name, capability.kind, capability.id
                    );
                }
                if discovery.extensions.is_empty() {
                    println!("No compatible extensions found.");
                }
                for extension in discovery.extensions.iter().filter(matches_filter) {
                    println!(
                        "- {} {} ({:?}) [{}]",
                        extension.manifest.name,
                        extension.manifest.version,
                        extension.manifest.kind,
                        extension.manifest.id
                    );
                    println!(
                        "  entrypoint: {} ({})",
                        extension
                            .manifest
                            .entrypoint
                            .as_deref()
                            .unwrap_or("não declarado"),
                        extension_entrypoint_status(extension)
                    );
                    if !extension.manifest.limitations.is_empty() {
                        println!(
                            "  limitations: {}",
                            extension.manifest.limitations.join("; ")
                        );
                    }
                }
                if !discovery.diagnostics.is_empty() {
                    println!("Diagnostics:");
                    for diagnostic in discovery.diagnostics {
                        println!("  - {diagnostic}");
                    }
                }
                if let Some(results) = wasm_verification {
                    println!("WASM verification:");
                    for result in results {
                        let id = result["id"].as_str().unwrap_or("unknown");
                        if result["ok"].as_bool() == Some(true) {
                            println!("  - [OK] {id}");
                        } else {
                            println!(
                                "  - [Error] {id}: {}",
                                result["error"].as_str().unwrap_or("unknown error")
                            );
                        }
                    }
                }
            }
        }

        Some(Commands::PlatformInfo) => {
            let matrix = platform_capability_matrix();
            if json_output {
                print_json(&json!({
                    "current_platform": kamafeu::extensions::current_platform(),
                    "capabilities": matrix,
                }))?;
            } else {
                println!(
                    "Current platform: {}",
                    kamafeu::extensions::current_platform()
                );
                for capability in matrix {
                    println!(
                        "- {}: desktop={} android={} wasm={} — {}",
                        capability.id,
                        capability.desktop,
                        capability.android,
                        capability.wasm,
                        capability.note
                    );
                }
            }
        }

        Some(Commands::CacheInfo) => {
            let report = cache_info_json();
            if json_output {
                print_json(&report)?;
            } else {
                println!("=== Render cache ===");
                println!("Directory: {}", report["directory"].as_str().unwrap_or(""));
                println!("Disk files: {}", report["disk_file_count"]);
                println!("Disk size:  {} bytes", report["disk_bytes"]);
                println!(
                    "Memory:     {} entries / {} samples",
                    report["memory_entries"], report["memory_samples"]
                );
                println!(
                    "Stats:      {} hit(s), {} miss(es), {} render(s)",
                    report["hits"], report["misses"], report["renders"]
                );
            }
        }

        Some(Commands::CachePrune { days }) => {
            let removed = kamafeu::renderer::resampler_cache::cleanup_older_than(days)?;
            if json_output {
                print_json(&json!({ "days": days, "removed": removed }))?;
            } else {
                println!("Removed {removed} cache file(s) older than {days} day(s).");
            }
        }

        Some(Commands::Automation) => run_automation()?,

        Some(Commands::Mcp) => run_mcp()?,

        Some(Commands::GenSample { path }) => {
            println!("Generating test UTAU voicebank at: {:?}", path);
            fs::create_dir_all(&path)?;

            let char_txt = "name=Kamafeu Sample Synth\nauthor=Kamafeu Team\n";
            fs::write(path.join("character.txt"), char_txt)?;

            let vowels = [
                ("ka", 261.63),
                ("ki", 293.66),
                ("ku", 329.63),
                ("ke", 349.23),
                ("ko", 392.00),
            ];

            let sample_rate = 44100;
            let mut oto_lines = Vec::new();

            for &(name, freq) in &vowels {
                let filename = format!("{}.wav", name);
                let wav_path = path.join(&filename);

                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: sample_rate as u32,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };

                let mut writer = hound::WavWriter::create(&wav_path, spec)?;
                let duration_sec = 1.0;
                let total_samples = (sample_rate as f64 * duration_sec) as usize;

                for i in 0..total_samples {
                    let t = i as f64 / sample_rate as f64;
                    let sample_val = 0.6 * (2.0 * std::f64::consts::PI * freq * t).sin()
                        + 0.3 * (2.0 * std::f64::consts::PI * freq * 2.0 * t).sin()
                        + 0.1 * (2.0 * std::f64::consts::PI * freq * 3.0 * t).sin();

                    let sample_i16 = (sample_val * i16::MAX as f64) as i16;
                    writer.write_sample(sample_i16)?;
                }
                writer.finalize()?;

                oto_lines.push(format!("{}=,10,50,-100,30,15", filename));
            }

            fs::write(path.join("oto.ini"), oto_lines.join("\n"))?;

            println!("Sample voicebank created successfully!");
            println!("Contains: character.txt, oto.ini, and ka.wav..ko.wav samples.");
        }

        Some(Commands::Render {
            voicebank,
            input,
            output,
            format,
            dither,
            sample_rate,
            venus,
            vocal_only,
        }) => {
            let summary = render_project_to_file(
                &voicebank,
                &input,
                &output,
                format,
                dither,
                sample_rate,
                venus,
                vocal_only,
            )?;
            if json_output {
                print_json(&render_summary_json(&summary))?;
            } else {
                println!(
                    "Rendered {} notes at {}Hz.",
                    summary.note_count, summary.sample_rate
                );
                println!(
                    "{} written to {} ({} frames, {} channel(s)).",
                    summary.format.display_name(),
                    summary.output.display(),
                    summary.frame_count,
                    summary.channels
                );
                println!(
                    "Audio: peak {:.4}, RMS {:.4}, {} clipped sample(s), {}.",
                    summary.diagnostics.peak,
                    summary.diagnostics.rms,
                    summary.diagnostics.clipped_samples,
                    if summary.silent { "silent" } else { "audible" }
                );
            }
        }
    }

    Ok(())
}

fn print_json(value: &serde_json::Value) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Serves one JSON object per input line and writes exactly one JSON response
/// per request. Keeping this protocol on stdio makes it usable from scripts,
/// editor integrations and an eventual MCP adapter without opening a network
/// port or granting ambient filesystem access.
fn run_automation() -> Result<(), Box<dyn std::error::Error>> {
    for (line_number, line) in std::io::stdin().lock().lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<AutomationRequest>(&line) {
            Ok(request) => match handle_automation_request(request) {
                Ok(result) => json!({ "ok": true, "result": result }),
                Err(error) => json!({ "ok": false, "error": error.to_string() }),
            },
            Err(error) => json!({
                "ok": false,
                "error": format!("invalid request on line {}: {error}", line_number + 1),
            }),
        };
        println!("{}", serde_json::to_string(&response)?);
    }
    Ok(())
}

/// Minimal MCP server backed by the same safe project operations used by the
/// automation protocol. It intentionally has no network listener: the caller
/// owns process creation and filesystem permissions.
fn run_mcp() -> Result<(), Box<dyn std::error::Error>> {
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                println!(
                    "{}",
                    serde_json::to_string(&mcp_error(
                        Value::Null,
                        -32700,
                        &format!("parse error: {error}")
                    ))?
                );
                continue;
            }
        };
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str);
        let response = match method {
            Some("initialize") => Some(mcp_result(
                id.unwrap_or(Value::Null),
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "kamafeu-studio", "version": kamafeu::APP_VERSION },
                }),
            )),
            Some("notifications/initialized") => None,
            Some("tools/list") => Some(mcp_result(
                id.unwrap_or(Value::Null),
                json!({ "tools": mcp_tools() }),
            )),
            Some("tools/call") => Some(mcp_tool_call(
                id.unwrap_or(Value::Null),
                message.get("params").cloned().unwrap_or(Value::Null),
            )),
            Some(method) => {
                id.map(|id| mcp_error(id, -32601, &format!("method not found: {method}")))
            }
            None => id.map(|id| mcp_error(id, -32600, "invalid JSON-RPC request")),
        };
        if let Some(response) = response {
            println!("{}", serde_json::to_string(&response)?);
        }
    }
    Ok(())
}

fn mcp_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn mcp_error(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn mcp_tool_call(id: Value, params: Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return mcp_error(id, -32602, "tools/call requires params.name");
    };
    let Some(command) = name.strip_prefix("kamafeu_") else {
        return mcp_tool_result(id, Err(format!("unknown Kamafeu tool: {name}")));
    };
    let mut arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    arguments.insert("command".to_string(), Value::String(command.to_string()));
    let request = serde_json::from_value::<AutomationRequest>(Value::Object(arguments))
        .map_err(|error| format!("invalid arguments for {name}: {error}"));
    let result = request
        .and_then(|request| handle_automation_request(request).map_err(|error| error.to_string()));
    mcp_tool_result(id, result)
}

fn mcp_tool_result(id: Value, result: Result<Value, String>) -> Value {
    match result {
        Ok(result) => mcp_result(
            id,
            json!({
                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default() }],
                "structuredContent": result,
                "isError": false,
            }),
        ),
        Err(error) => mcp_result(
            id,
            json!({
                "content": [{ "type": "text", "text": error }],
                "isError": true,
            }),
        ),
    }
}

fn mcp_tools() -> Vec<Value> {
    [
        (
            "project_info",
            "Read project metadata, tracks, notes and duration.",
        ),
        (
            "diff",
            "Compare two project revisions without modifying either input.",
        ),
        (
            "list_voicebanks",
            "Scan singer folders and list UTAU, OpenUTAU and DiffSinger voicebanks.",
        ),
        (
            "validate_project",
            "Check a project for invalid tempo, tracks, notes and audio parts.",
        ),
        (
            "create_project",
            "Create a new APS, USTX, UST, MIDI, VSQX, SVP, UFData or JSON project.",
        ),
        (
            "add_note",
            "Add a vocal note to a project and save to an explicit output path.",
        ),
        ("set_note", "Change lyric and/or pitch of a specific note."),
        (
            "remove_note",
            "Remove one note and save the resulting project to an explicit output path.",
        ),
        (
            "apply_lyrics",
            "Split and distribute lyrics across a vocal part in musical time order.",
        ),
        (
            "add_marker",
            "Add a named arrangement marker at an explicit timeline position.",
        ),
        (
            "remove_marker",
            "Remove a project marker and save the resulting project to an explicit output path.",
        ),
        (
            "add_section",
            "Add a named arrangement section with explicit start and end positions.",
        ),
        (
            "remove_section",
            "Remove a project section and save the resulting project to an explicit output path.",
        ),
        (
            "set_pitch_bend",
            "Replace a note pitch-bend curve with time/cents points.",
        ),
        (
            "set_expression",
            "Set vocal expressions such as dynamics, breathiness and gender.",
        ),
        (
            "add_audio_part",
            "Add an existing audio file to the arrangement.",
        ),
        (
            "render",
            "Render a project with a voicebank to WAV, FLAC or RAW.",
        ),
        (
            "validate_voicebank",
            "Validate UTAU assets or DiffSinger configuration/runtime.",
        ),
        (
            "extensions",
            "List extensions and optionally verify their WASM modules.",
        ),
        ("cache_info", "Inspect renderer cache location and usage."),
        ("cache_prune", "Remove stale renderer-cache entries."),
    ]
    .into_iter()
    .map(|(name, description)| {
        json!({
            "name": format!("kamafeu_{name}"),
            "description": description,
            "inputSchema": { "type": "object", "additionalProperties": true },
        })
    })
    .collect()
}

fn handle_automation_request(
    request: AutomationRequest,
) -> Result<Value, Box<dyn std::error::Error>> {
    match request {
        AutomationRequest::CreateProject { output, name, bpm } => {
            let mut project = UProject::default();
            if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
                project.name = name;
            }
            if let Some(bpm) = bpm {
                project.set_bpm_preserving_beats(bpm).ok_or_else(|| {
                    Error::new(ErrorKind::InvalidInput, "BPM deve ser um número positivo")
                })?;
            }
            save_project(&project, &output)?;
            Ok(json!({ "output": output, "name": project.name, "bpm": project.bpm }))
        }
        AutomationRequest::ProjectInfo { path } => project_info_json(&path),
        AutomationRequest::Diff { before, after } => {
            let before_project = load_project(&before)?;
            let after_project = load_project(&after)?;
            Ok(serde_json::to_value(
                kamafeu::project::ProjectDiff::between(&before_project, &after_project),
            )?)
        }
        AutomationRequest::ListVoicebanks {
            paths,
            include_default,
        } => Ok(voicebank_catalog_json(
            paths.unwrap_or_default(),
            include_default,
        )),
        AutomationRequest::ValidateProject {
            path,
            extensions_path,
        } => project_validation_json(&path, extensions_path.as_ref()),
        AutomationRequest::Convert { input, output } => convert_project(&input, &output),
        AutomationRequest::AddNote {
            input,
            output,
            lyric,
            pitch,
            position_ms,
            duration_ms,
            track_index,
        } => {
            if !position_ms.is_finite() || position_ms < 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "position_ms deve ser um número finito maior ou igual a zero",
                )
                .into());
            }
            if !duration_ms.is_finite() || duration_ms <= 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "duration_ms deve ser um número finito maior que zero",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            project.normalize();
            let track_index = track_index.unwrap_or(0);
            while project.tracks.len() <= track_index {
                project.tracks.push(kamafeu::project::UTrack {
                    name: format!("Track {}", project.tracks.len() + 1),
                    ..kamafeu::project::UTrack::default()
                });
            }
            let part_index = project
                .parts
                .iter()
                .position(|part| part.track_index == track_index)
                .unwrap_or_else(|| {
                    project.parts.push(kamafeu::project::UVoicePart::new(
                        format!("Parte Vocal {}", track_index + 1),
                        track_index,
                    ));
                    project.parts.len() - 1
                });
            project.parts[part_index].notes.push(UNote::new(
                lyric,
                pitch,
                position_ms,
                duration_ms,
            ));
            project.parts[part_index].notes.sort_by(|left, right| {
                left.position_ms
                    .partial_cmp(&right.position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "track_index": track_index,
                "part_index": part_index,
                "note_count": project.parts[part_index].notes.len(),
            }))
        }
        AutomationRequest::AddAudioPart {
            input,
            output,
            file_path,
            position_ms,
            track_index,
            name,
            volume_db,
        } => {
            if !file_path.is_file() {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("arquivo de áudio não encontrado: {}", file_path.display()),
                )
                .into());
            }
            if !position_ms.is_finite() || position_ms < 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "position_ms deve ser um número finito maior ou igual a zero",
                )
                .into());
            }
            if volume_db.is_some_and(|value| !value.is_finite()) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "volume_db deve ser um número finito",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            project.normalize();
            let track_index = track_index.unwrap_or(0);
            while project.tracks.len() <= track_index {
                project.tracks.push(kamafeu::project::UTrack {
                    name: format!("Track {}", project.tracks.len() + 1),
                    ..kamafeu::project::UTrack::default()
                });
            }
            let default_name = file_path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Audio")
                .to_string();
            let mut wave = kamafeu::project::UWavePart::new(
                name.filter(|name| !name.trim().is_empty())
                    .unwrap_or(default_name),
                file_path.to_string_lossy(),
                track_index,
            );
            wave.position_ms = position_ms;
            if let Some(volume_db) = volume_db {
                wave.volume_db = volume_db;
            }
            let duration_ms = wave.duration_ms;
            project.wave_parts.push(wave);
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "track_index": track_index,
                "wave_part_count": project.wave_parts.len(),
                "duration_ms": duration_ms,
            }))
        }
        AutomationRequest::SetTempo { input, output, bpm } => {
            let mut project = load_project(&input)?;
            let scale = project.set_bpm_preserving_beats(bpm).ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, "BPM deve ser um número positivo")
            })?;
            save_project(&project, &output)?;
            Ok(json!({ "input": input, "output": output, "bpm": project.bpm, "time_scale": scale }))
        }
        AutomationRequest::SetNote {
            input,
            output,
            part_index,
            note_index,
            lyric,
            pitch,
        } => {
            let mut project = load_project(&input)?;
            let note = automation_note_mut(&mut project, part_index, note_index)?;
            if let Some(lyric) = lyric {
                note.lyric = lyric;
            }
            if let Some(pitch) = pitch {
                let valid = kamafeu::dsp::pitch::note_name_to_midi(&pitch)
                    .or_else(|| pitch.parse::<u8>().ok())
                    .is_some();
                if !valid {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "pitch deve ser uma nota como C4 ou uma chave MIDI entre 0 e 127",
                    )
                    .into());
                }
                note.pitch = pitch;
            }
            save_project(&project, &output)?;
            Ok(
                json!({ "input": input, "output": output, "part_index": part_index, "note_index": note_index }),
            )
        }
        AutomationRequest::RemoveNote {
            input,
            output,
            part_index,
            note_index,
        } => {
            let mut project = load_project(&input)?;
            project.normalize();
            let part = project.parts.get_mut(part_index).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("parte vocal {part_index} não existe"),
                )
            })?;
            if note_index >= part.notes.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("nota {note_index} não existe na parte vocal {part_index}"),
                )
                .into());
            }
            let removed = part.notes.remove(note_index);
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "note_index": note_index,
                "removed": {
                    "lyric": removed.lyric,
                    "pitch": removed.pitch,
                    "position_ms": removed.position_ms,
                    "duration_ms": removed.duration_ms,
                },
            }))
        }
        AutomationRequest::ApplyLyrics {
            input,
            output,
            part_index,
            text,
            mode,
            start_note_index,
        } => {
            let split_mode = automation_lyrics_split_mode(mode.as_deref())?;
            let tokens = kamafeu::gui::lyrics_dialog::split_lyrics(&text, split_mode);
            if tokens.is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "text deve conter ao menos uma palavra ou sílaba",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            project.normalize();
            let part = project.parts.get_mut(part_index).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("parte vocal {part_index} não existe"),
                )
            })?;
            let start_note_index = start_note_index.unwrap_or(0);
            if start_note_index >= part.notes.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "nota inicial {start_note_index} não existe na parte vocal {part_index}"
                    ),
                )
                .into());
            }
            let mut target_indices = (0..part.notes.len()).collect::<Vec<_>>();
            target_indices.sort_by(|left, right| {
                part.notes[*left]
                    .position_ms
                    .partial_cmp(&part.notes[*right].position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let start_at = target_indices
                .iter()
                .position(|index| *index == start_note_index)
                .expect("the requested note index is already validated");
            let target_indices = &target_indices[start_at..];
            let applied = tokens.len().min(target_indices.len());
            for (token, note_index) in tokens.iter().zip(target_indices.iter()) {
                part.notes[*note_index].lyric = token.clone();
            }
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "start_note_index": start_note_index,
                "mode": mode.unwrap_or_else(|| "hyphens_and_spaces".to_string()),
                "token_count": tokens.len(),
                "applied_count": applied,
                "unassigned_tokens": tokens.len().saturating_sub(applied),
                "unchanged_notes": target_indices.len().saturating_sub(applied),
            }))
        }
        AutomationRequest::AddMarker {
            input,
            output,
            name,
            position_ms,
            color,
        } => {
            if !position_ms.is_finite() || position_ms < 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "position_ms deve ser um número finito maior ou igual a zero",
                )
                .into());
            }
            if name.trim().is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "name do marcador não pode estar vazio",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let mut marker = kamafeu::project::UProjectMarker::new(name, position_ms);
            marker.color = color;
            let marker_name = marker.name.trim().to_string();
            project.markers.push(marker);
            project.normalize();
            let marker_index = project
                .markers
                .iter()
                .rposition(|marker| {
                    (marker.position_ms - position_ms).abs() < f64::EPSILON
                        && marker.name == marker_name
                })
                .unwrap_or_else(|| project.markers.len().saturating_sub(1));
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "marker_index": marker_index,
                "marker": project.markers.get(marker_index).map(|marker| json!({
                    "name": marker.name,
                    "position_ms": marker.position_ms,
                    "color": marker.color,
                })),
            }))
        }
        AutomationRequest::RemoveMarker {
            input,
            output,
            marker_index,
        } => {
            let mut project = load_project(&input)?;
            if marker_index >= project.markers.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("marcador {marker_index} não existe"),
                )
                .into());
            }
            let marker = project.markers.remove(marker_index);
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "marker_index": marker_index,
                "removed": {
                    "name": marker.name,
                    "position_ms": marker.position_ms,
                    "color": marker.color,
                },
            }))
        }
        AutomationRequest::AddSection {
            input,
            output,
            name,
            start_ms,
            end_ms,
            color,
        } => {
            if !start_ms.is_finite() || !end_ms.is_finite() || start_ms < 0.0 || end_ms < start_ms {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "start_ms e end_ms devem ser finitos, com end_ms maior ou igual a start_ms",
                )
                .into());
            }
            if name.trim().is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "name da seção não pode estar vazio",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let mut section = kamafeu::project::UProjectSection::new(name, start_ms, end_ms);
            section.color = color;
            let section_name = section.name.trim().to_string();
            project.sections.push(section);
            project.normalize();
            let section_index = project
                .sections
                .iter()
                .rposition(|section| {
                    section.name == section_name
                        && (section.start_ms - start_ms).abs() < f64::EPSILON
                        && (section.end_ms - end_ms).abs() < f64::EPSILON
                })
                .unwrap_or_else(|| project.sections.len().saturating_sub(1));
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "section_index": section_index,
                "section": project.sections.get(section_index).map(|section| json!({
                    "name": section.name,
                    "start_ms": section.start_ms,
                    "end_ms": section.end_ms,
                    "color": section.color,
                })),
            }))
        }
        AutomationRequest::RemoveSection {
            input,
            output,
            section_index,
        } => {
            let mut project = load_project(&input)?;
            if section_index >= project.sections.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("seção {section_index} não existe"),
                )
                .into());
            }
            let section = project.sections.remove(section_index);
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "section_index": section_index,
                "removed": {
                    "name": section.name,
                    "start_ms": section.start_ms,
                    "end_ms": section.end_ms,
                    "color": section.color,
                },
            }))
        }
        AutomationRequest::SetPitchBend {
            input,
            output,
            part_index,
            note_index,
            points,
            snap_first,
        } => {
            if points.iter().any(|point| {
                !point.time_offset_ms.is_finite() || !point.pitch_offset_cents.is_finite()
            }) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "pontos de pitch bend devem conter valores finitos",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let note = automation_note_mut(&mut project, part_index, note_index)?;
            note.pitch_bend.points = points
                .into_iter()
                .map(|point| kamafeu::project::UPitchBendPoint {
                    time_offset_ms: point.time_offset_ms,
                    pitch_offset_cents: point.pitch_offset_cents,
                    shape: point.shape,
                })
                .collect();
            if let Some(snap_first) = snap_first {
                note.pitch_bend.snap_first = snap_first;
            }
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "note_index": note_index,
                "point_count": project.parts[part_index].notes[note_index].pitch_bend.points.len(),
            }))
        }
        AutomationRequest::SetExpression {
            input,
            output,
            part_index,
            note_index,
            dynamics,
            pitch_delta,
            gender,
            breathiness,
            velocity,
            modulation,
            volume,
            attack,
            decay,
        } => {
            let values = [
                dynamics,
                pitch_delta,
                gender,
                breathiness,
                velocity,
                modulation,
                volume,
                attack,
                decay,
            ];
            if values.iter().flatten().any(|value| !value.is_finite()) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "expressões devem conter apenas valores finitos",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let note = automation_note_mut(&mut project, part_index, note_index)?;
            if let Some(value) = dynamics {
                note.expressions.dynamics = value;
            }
            if let Some(value) = pitch_delta {
                note.expressions.pitch_delta = value;
            }
            if let Some(value) = gender {
                note.expressions.gender = value;
            }
            if let Some(value) = breathiness {
                note.expressions.breathiness = value;
            }
            if let Some(value) = velocity {
                note.expressions.velocity = value;
            }
            if let Some(value) = modulation {
                note.expressions.modulation = value;
            }
            if let Some(value) = volume {
                note.expressions.volume = value;
            }
            if let Some(value) = attack {
                note.expressions.attack = value;
            }
            if let Some(value) = decay {
                note.expressions.decay = value;
            }
            project.normalize();
            let expressions = &project.parts[part_index].notes[note_index].expressions;
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "note_index": note_index,
                "expressions": {
                    "dynamics": expressions.dynamics,
                    "pitch_delta": expressions.pitch_delta,
                    "gender": expressions.gender,
                    "breathiness": expressions.breathiness,
                    "velocity": expressions.velocity,
                    "modulation": expressions.modulation,
                    "volume": expressions.volume,
                    "attack": expressions.attack,
                    "decay": expressions.decay,
                },
            }))
        }
        AutomationRequest::Render {
            voicebank,
            input,
            output,
            sample_rate,
            venus,
            vocal_only,
            dither,
        } => {
            let summary = render_project_to_file(
                &voicebank,
                &input,
                &output,
                RenderOutputFormat::Auto,
                dither,
                sample_rate.unwrap_or(44_100),
                venus,
                vocal_only,
            )?;
            Ok(render_summary_json(&summary))
        }
        AutomationRequest::ValidateVoicebank {
            path,
            verify_runtime,
        } => voicebank_validation_json(&path, verify_runtime),
        AutomationRequest::Extensions {
            path,
            verify_wasm,
            grant_permissions,
            format_filter,
            phonemizer_filter,
            invoke_id,
            invoke_export,
            invoke_argument,
            invoke_bytes,
            invoke_parameters,
        } => {
            let path = path.unwrap_or_else(|| PathBuf::from("extensions"));
            if let Some(id) = invoke_id {
                invoke_extension_json(
                    &path,
                    &id,
                    invoke_export.as_deref().unwrap_or("analyze"),
                    invoke_argument.unwrap_or(0),
                    invoke_bytes,
                    &grant_permissions,
                    &invoke_parameters,
                )
            } else {
                Ok(extension_catalog_json(
                    &path,
                    verify_wasm,
                    &grant_permissions,
                    format_filter.as_deref(),
                    phonemizer_filter.as_deref(),
                ))
            }
        }
        AutomationRequest::CacheInfo => Ok(cache_info_json()),
        AutomationRequest::CachePrune { days } => {
            let days = days.unwrap_or(30);
            let removed = kamafeu::renderer::resampler_cache::cleanup_older_than(days)?;
            Ok(json!({ "days": days, "removed": removed }))
        }
    }
}

fn voicebank_catalog_json(mut directories: Vec<PathBuf>, include_default: bool) -> Value {
    if directories.is_empty() || include_default {
        for directory in SingerScanner::default_singers_directories() {
            if !directories.contains(&directory) {
                directories.push(directory);
            }
        }
    }

    let voicebanks = SingerScanner::scan_directories(&directories)
        .into_iter()
        .map(|singer| {
            json!({
                "name": singer.name,
                "author": singer.author,
                "voice_type": singer.voice_type,
                "path": singer.path,
                "website": singer.web,
                "image_path": singer.image_path,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "directories": directories,
        "count": voicebanks.len(),
        "voicebanks": voicebanks,
    })
}

fn cache_info_json() -> Value {
    let directory = kamafeu::renderer::resampler_cache::persistent_cache_dir();
    let (disk_file_count, disk_bytes) = kamafeu::renderer::resampler_cache::get_disk_cache_stats();
    let (memory_entries, memory_samples) =
        kamafeu::renderer::resampler_cache::get_memory_cache_stats();
    let stats = kamafeu::renderer::resampler_cache::cache_stats();
    json!({
        "directory": directory,
        "disk_file_count": disk_file_count,
        "disk_bytes": disk_bytes,
        "memory_entries": memory_entries,
        "memory_samples": memory_samples,
        "hits": stats.hits,
        "misses": stats.misses,
        "renders": stats.renders,
    })
}

fn automation_note_mut(
    project: &mut UProject,
    part_index: usize,
    note_index: usize,
) -> Result<&mut UNote, Box<dyn std::error::Error>> {
    let part = project.parts.get_mut(part_index).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("part_index inválido: {part_index}"),
        )
    })?;
    part.notes.get_mut(note_index).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("note_index inválido para a parte {part_index}: {note_index}"),
        )
        .into()
    })
}

#[derive(Debug, Clone, PartialEq)]
struct ConversionFingerprint {
    bpm: f64,
    time_signature_numerator: u8,
    time_signature_denominator: u8,
    track_count: usize,
    voice_part_count: usize,
    wave_part_count: usize,
    marker_count: usize,
    section_count: usize,
    extension_count: usize,
    note_count: usize,
    duration_ms: f64,
    pitch_bend_point_count: usize,
    dynamics_curve_point_count: usize,
    phoneme_override_count: usize,
    non_default_expression_count: usize,
    note_content_signature: u64,
}

impl ConversionFingerprint {
    fn from_project(project: &UProject) -> Self {
        let mut fingerprint = Self {
            bpm: project.bpm,
            time_signature_numerator: project.time_signature_numerator,
            time_signature_denominator: project.time_signature_denominator,
            track_count: project.tracks.len(),
            voice_part_count: project.parts.len(),
            wave_part_count: project.wave_parts.len(),
            marker_count: project.markers.len(),
            section_count: project.sections.len(),
            extension_count: project.extensions.len(),
            note_count: 0,
            duration_ms: 0.0,
            pitch_bend_point_count: 0,
            dynamics_curve_point_count: 0,
            phoneme_override_count: 0,
            non_default_expression_count: 0,
            note_content_signature: 0xcbf2_9ce4_8422_2325,
        };
        for part in &project.parts {
            fingerprint.hash_u64(part.track_index as u64);
            fingerprint.hash_f64(part.position_ms);
            for note in &part.notes {
                fingerprint.note_count += 1;
                fingerprint.duration_ms = fingerprint
                    .duration_ms
                    .max(part.position_ms + note.position_ms + note.duration_ms.max(0.0));
                fingerprint.pitch_bend_point_count += note.pitch_bend.points.len();
                fingerprint.dynamics_curve_point_count += note.expressions.dynamics_curve.len();
                fingerprint.phoneme_override_count += note.phoneme_overrides.len();
                if note.expressions != kamafeu::project::UExpressions::default() {
                    fingerprint.non_default_expression_count += 1;
                }
                fingerprint.hash_text(&note.lyric);
                fingerprint.hash_text(&note.pitch);
                fingerprint.hash_f64(note.position_ms);
                fingerprint.hash_f64(note.duration_ms);
                for phoneme_override in &note.phoneme_overrides {
                    fingerprint
                        .hash_text(&serde_json::to_string(phoneme_override).unwrap_or_default());
                }
            }
        }
        for wave in &project.wave_parts {
            fingerprint.duration_ms = fingerprint
                .duration_ms
                .max(wave.position_ms + wave.duration_ms.max(0.0));
        }
        fingerprint
    }

    fn hash_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.note_content_signature ^= u64::from(byte);
            self.note_content_signature =
                self.note_content_signature.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn hash_f64(&mut self, value: f64) {
        self.hash_u64(value.to_bits());
    }

    fn hash_text(&mut self, value: &str) {
        self.hash_u64(value.len() as u64);
        for byte in value.as_bytes() {
            self.note_content_signature ^= u64::from(*byte);
            self.note_content_signature =
                self.note_content_signature.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn json(&self) -> Value {
        json!({
            "bpm": self.bpm,
            "time_signature": format!("{}/{}", self.time_signature_numerator, self.time_signature_denominator),
            "track_count": self.track_count,
            "voice_part_count": self.voice_part_count,
            "wave_part_count": self.wave_part_count,
            "marker_count": self.marker_count,
            "section_count": self.section_count,
            "extension_count": self.extension_count,
            "note_count": self.note_count,
            "duration_ms": self.duration_ms,
            "pitch_bend_point_count": self.pitch_bend_point_count,
            "dynamics_curve_point_count": self.dynamics_curve_point_count,
            "phoneme_override_count": self.phoneme_override_count,
            "non_default_expression_count": self.non_default_expression_count,
            "note_content_signature": format!("{:016x}", self.note_content_signature),
        })
    }
}

fn convert_project(input: &PathBuf, output: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let mut source = load_project(input)?;
    source.normalize();
    let before = ConversionFingerprint::from_project(&source);
    save_project(&source, output)?;

    // Reopen through the selected adapter. A write succeeding is not enough:
    // interchange formats can intentionally omit data the native project has.
    let destination = load_project(output)?;
    let after = ConversionFingerprint::from_project(&destination);
    let mut warnings = Vec::new();
    compare_conversion_feature(
        &mut warnings,
        "tempo (BPM)",
        before.bpm,
        after.bpm,
        |left, right| (left - right).abs() > 0.001,
    );
    let before_meter = format!(
        "{}/{}",
        before.time_signature_numerator, before.time_signature_denominator
    );
    let after_meter = format!(
        "{}/{}",
        after.time_signature_numerator, after.time_signature_denominator
    );
    compare_conversion_feature(
        &mut warnings,
        "compasso",
        before_meter.as_str(),
        after_meter.as_str(),
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "faixas",
        before.track_count,
        after.track_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "partes vocais",
        before.voice_part_count,
        after.voice_part_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "partes de áudio",
        before.wave_part_count,
        after.wave_part_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "marcadores de projeto",
        before.marker_count,
        after.marker_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "seções de projeto",
        before.section_count,
        after.section_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "notas",
        before.note_count,
        after.note_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "conteúdo de letra, altura ou tempo das notas (assinatura)",
        before.note_content_signature,
        after.note_content_signature,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "duração do projeto (ms)",
        before.duration_ms,
        after.duration_ms,
        |left, right| (left - right).abs() > 1.0,
    );
    compare_conversion_feature(
        &mut warnings,
        "pontos de pitch bend",
        before.pitch_bend_point_count,
        after.pitch_bend_point_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "pontos de curva de dinâmica",
        before.dynamics_curve_point_count,
        after.dynamics_curve_point_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "overrides persistidos por fonema",
        before.phoneme_override_count,
        after.phoneme_override_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "notas com expressões não padrão",
        before.non_default_expression_count,
        after.non_default_expression_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "extensões associadas ao projeto",
        before.extension_count,
        after.extension_count,
        |left, right| left != right,
    );

    Ok(json!({
        "input": input,
        "output": output,
        "source": before.json(),
        "destination": after.json(),
        "warnings": warnings,
        "tracked_features_preserved": warnings.is_empty(),
    }))
}

fn compare_conversion_feature<T: std::fmt::Display + Copy>(
    warnings: &mut Vec<String>,
    label: &str,
    source: T,
    destination: T,
    differs: impl FnOnce(T, T) -> bool,
) {
    if differs(source, destination) {
        warnings.push(format!(
            "{label}: origem {source}, destino {destination}; revise este recurso após a conversão"
        ));
    }
}

fn project_validation_json(
    path: &PathBuf,
    extensions_path: Option<&PathBuf>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let project = load_project(path)?;
    let report = project.diagnostic_report();
    let extension_diagnostics = extensions_path
        .map(|catalog_path| {
            let discovery = discover(catalog_path);
            let mut diagnostics = diagnose_project_extensions(&project.extensions, &discovery);
            diagnostics.extend(discovery.diagnostics);
            diagnostics
        })
        .unwrap_or_default();
    Ok(json!({
        "path": path,
        "valid": report.is_valid(),
        "track_count": report.track_count,
        "voice_part_count": report.voice_part_count,
        "wave_part_count": report.wave_part_count,
        "note_count": report.note_count,
        "error_count": report.error_count(),
        "warning_count": report.warning_count(),
        "vocal_health_score": report.vocal_health_score(),
        "extension_diagnostics": extension_diagnostics,
        "extension_diagnostic_records": extensions_path.map(|catalog_path| {
            let discovery = discover(catalog_path);
            let mut records = diagnose_project_extensions_structured(&project.extensions, &discovery)
                .into_iter()
                .map(|record| serde_json::to_value(record).unwrap_or_else(|_| serde_json::json!({})))
                .collect::<Vec<_>>();
            records.extend(discovery.diagnostic_records.into_iter().map(|diagnostic| {
                serde_json::json!({
                    "extension_id": serde_json::Value::Null,
                    "kind": "discovery",
                    "detail": diagnostic.reason,
                    "manifest_path": diagnostic.manifest_path,
                })
            }));
            serde_json::to_value(records).unwrap_or_else(|_| serde_json::json!([]))
        }).unwrap_or_else(|| serde_json::json!([])),
        "issues": report.issues.iter().map(|issue| json!({
            "severity": format!("{:?}", issue.severity).to_ascii_lowercase(),
            "kind": format!("{:?}", issue.kind),
            "location": issue.location,
            "detail": issue.detail,
            "suggestion": issue.suggestion(),
        })).collect::<Vec<_>>(),
    }))
}

struct RenderSummary {
    output: PathBuf,
    format: AudioExportFormat,
    sample_rate: u32,
    channels: u16,
    frame_count: usize,
    note_count: usize,
    diagnostics: AudioDiagnostics,
    silent: bool,
}

#[allow(clippy::too_many_arguments)]
fn render_project_to_file(
    voicebank_path: &PathBuf,
    input: &PathBuf,
    output: &PathBuf,
    requested_format: RenderOutputFormat,
    dither: bool,
    sample_rate: u32,
    venus: bool,
    vocal_only: bool,
) -> Result<RenderSummary, Box<dyn std::error::Error>> {
    let voicebank = Voicebank::new(voicebank_path)?;
    let mut project = load_project(input)?;
    project.normalize();
    if vocal_only {
        project.wave_parts.clear();
    }
    let note_count: usize = project.parts.iter().map(|part| part.notes.len()).sum();
    if note_count == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "o projeto não contém notas renderizáveis",
        )
        .into());
    }

    let classic_resampler = NativeResamplerDriver;
    let world_resampler = NativeWorldResamplerDriver;
    let resampler: &dyn ResamplerDriver = if venus {
        &world_resampler
    } else {
        &classic_resampler
    };
    let native_wavtool = NativeWavtoolDriver;
    let render_options = RenderOptions {
        flags: project.flags.clone(),
        phonemizer_mode: project.phonemizer.unwrap_or_default(),
        ..RenderOptions::default()
    };
    let timing_diagnostics = std::sync::Arc::new(std::sync::Mutex::new(Vec::<
        kamafeu::renderer::timing::PhonemeTimingDiagnostic,
    >::new()));
    let timing_boundary_diagnostics = std::sync::Arc::new(std::sync::Mutex::new(Vec::<
        kamafeu::renderer::provenance::TimingBoundaryDiagnostic,
    >::new()));
    let render_commands = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let cache_stats = std::sync::Arc::new(std::sync::Mutex::new((0usize, 0usize)));
    let timing_capture = std::sync::Arc::clone(&timing_diagnostics);
    let timing_boundary_capture = std::sync::Arc::clone(&timing_boundary_diagnostics);
    let commands_capture = std::sync::Arc::clone(&render_commands);
    let cache_capture = std::sync::Arc::clone(&cache_stats);
    let progress_callback = move |_progress: f32, message: &str| {
        if let Some(payload) = message.strip_prefix("[Timing] ") {
            if let Ok(diagnostic) = serde_json::from_str(payload) {
                if let Ok(mut captured) = timing_capture.lock() {
                    captured.push(diagnostic);
                }
            }
        }
        if let Some(diagnostic) = kamafeu::renderer::provenance::parse_timing_boundary_log(message)
        {
            if let Ok(mut captured) = timing_boundary_capture.lock() {
                captured.push(diagnostic);
            }
        }
        if let Some(command) = message
            .strip_prefix("  [Resampler Command] ")
            .or_else(|| message.strip_prefix("  [Wavtool Command] "))
        {
            if let Ok(mut commands) = commands_capture.lock() {
                commands.push(command.to_string());
            }
        }
        if let Ok(mut stats) = cache_capture.lock() {
            if message.contains("[Resampler Cache] hit") {
                stats.0 += 1;
            } else if message.contains("[Resampler Cache] miss") {
                stats.1 += 1;
            }
        }
    };
    let rendered = ProjectRenderer::render_project_with_drivers(
        &project,
        &voicebank,
        sample_rate,
        0.0,
        resampler,
        &native_wavtool,
        &render_options,
        Some(&progress_callback),
    );
    if let Some(error) = rendered.error {
        return Err(std::io::Error::other(error).into());
    }

    let format = requested_format
        .resolve(output)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
    AudioExporter::export_audio_with_dither(
        output,
        &rendered.samples,
        sample_rate,
        rendered.channels,
        format,
        if dither {
            DitherMode::Tpdf
        } else {
            DitherMode::None
        },
    )
    .map_err(Error::other)?;
    let project_bytes = serde_json::to_vec(&project)?;
    let timing_diagnostics = timing_diagnostics
        .lock()
        .map(|diagnostics| diagnostics.clone())
        .unwrap_or_default();
    let timing_boundary_diagnostics = timing_boundary_diagnostics
        .lock()
        .map(|diagnostics| diagnostics.clone())
        .unwrap_or_default();
    let render_commands = render_commands
        .lock()
        .map(|commands| commands.clone())
        .unwrap_or_default();
    let (cache_hits, cache_misses) = cache_stats.lock().map(|stats| *stats).unwrap_or_default();
    let mut provenance = RenderProvenance::new(
        kamafeu::renderer::provenance::content_fingerprint(&project_bytes),
        voicebank.content_fingerprint(),
        resampler.name(),
        native_wavtool.name(),
        sample_rate,
        rendered.channels,
        render_options.clone(),
    )
    .with_timing_diagnostics(timing_diagnostics)
    .with_timing_boundary_diagnostics(timing_boundary_diagnostics)
    .with_commands(render_commands)
    .with_cache_stats(cache_hits, cache_misses)
    .with_extension_details(project.extensions.iter().map(|extension| {
        kamafeu::renderer::RenderExtension {
            id: extension.id.clone(),
            version: extension.version.clone(),
            origin: extension.origin.clone(),
            manifest_fingerprint: extension.manifest_fingerprint.clone(),
        }
    }))
    .with_artifact_bytes("project", input.display().to_string(), &project_bytes);
    provenance
        .warnings
        .extend(provenance.timing_boundary_warnings());
    provenance.warnings.sort();
    provenance.warnings.dedup();
    let oto_path = voicebank.root_path.join("oto.ini");
    if oto_path.is_file() {
        provenance = provenance.with_artifact_file("oto", &oto_path)?;
    }
    let mut wav_paths = std::collections::BTreeSet::new();
    for entry in voicebank.entries.values() {
        let wav_path = voicebank.root_path.join(&entry.wav_filename);
        if wav_path.is_file() {
            wav_paths.insert(wav_path);
        }
    }
    for wav_path in wav_paths {
        provenance = provenance.with_artifact_file("voicebank_wav", wav_path)?;
    }
    AudioExporter::write_provenance_sidecar(output, &provenance).map_err(Error::other)?;
    let diagnostics = AudioDiagnostics::analyze(&rendered.samples, rendered.channels);
    Ok(RenderSummary {
        output: output.clone(),
        format,
        sample_rate,
        channels: rendered.channels,
        frame_count: rendered.frame_count(),
        note_count,
        silent: diagnostics.peak < 1e-5,
        diagnostics,
    })
}

fn render_summary_json(summary: &RenderSummary) -> Value {
    json!({
        "output": summary.output,
        "format": summary.format.display_name(),
        "sample_rate": summary.sample_rate,
        "channels": summary.channels,
        "frame_count": summary.frame_count,
        "note_count": summary.note_count,
        "duration_ms": summary.frame_count as f64 / f64::from(summary.sample_rate.max(1)) * 1000.0,
        "audio": {
            "peak": summary.diagnostics.peak,
            "rms": summary.diagnostics.rms,
            "clipped_samples": summary.diagnostics.clipped_samples,
            "non_finite_samples": summary.diagnostics.non_finite_samples,
            "max_step": summary.diagnostics.max_step,
            "max_step_frame": summary.diagnostics.max_step_frame,
            "safe": summary.diagnostics.is_safe(),
            "silent": summary.silent,
        },
    })
}

fn project_info_json(path: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let project = load_project(path)?;
    let note_count: usize = project.parts.iter().map(|part| part.notes.len()).sum();
    let duration_ms = project
        .parts
        .iter()
        .flat_map(|part| {
            part.notes
                .iter()
                .map(move |note| part.position_ms + note.position_ms + note.duration_ms.max(0.0))
        })
        .chain(
            project
                .wave_parts
                .iter()
                .map(|wave| wave.position_ms + wave.duration_ms.max(0.0)),
        )
        .fold(0.0_f64, f64::max);
    let tracks = project
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let track_notes: usize = project
                .parts
                .iter()
                .filter(|part| part.track_index == index)
                .map(|part| part.notes.len())
                .sum();
            json!({
                "index": index,
                "name": track.name,
                "note_count": track_notes,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "path": path,
        "name": project.name,
        "bpm": project.bpm,
        "time_signature": format!("{}/{}", project.time_signature_numerator, project.time_signature_denominator),
        "track_count": project.tracks.len(),
        "voice_part_count": project.parts.len(),
        "wave_part_count": project.wave_parts.len(),
        "marker_count": project.markers.len(),
        "section_count": project.sections.len(),
        "sections": project.sections.iter().enumerate().map(|(index, section)| json!({
            "index": index,
            "name": section.name,
            "start_ms": section.start_ms,
            "end_ms": section.end_ms,
            "color": section.color,
        })).collect::<Vec<_>>(),
        "markers": project.markers.iter().enumerate().map(|(index, marker)| json!({
            "index": index,
            "name": marker.name,
            "position_ms": marker.position_ms,
            "color": marker.color,
        })).collect::<Vec<_>>(),
        "note_count": note_count,
        "duration_ms": duration_ms,
        "tracks": tracks,
    }))
}

fn voicebank_validation_json(
    path: &PathBuf,
    verify_runtime: bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    let vb = Voicebank::new(path)?;
    if vb.is_diffsinger() {
        let config = vb.diffsinger_config().map_err(Error::other)?;
        return diffsinger_validation_json(path, &vb, &config, verify_runtime);
    }
    let report = vb.diagnostic_report();
    let issues = report
        .issues
        .iter()
        .map(|issue| {
            json!({
                "alias": issue.alias,
                "wav": issue.wav_filename,
                "detail": issue.detail,
                "kind": format!("{:?}", issue.kind),
                "suggestion": issue.suggestion(),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "path": path,
        "name": vb.name,
        "healthy": report.is_healthy(),
        "entries": report.entry_count,
        "unique_wavs": report.unique_wav_count,
        "readable_wavs": report.readable_wav_count,
        "missing_wavs": report.missing_wav_count,
        "invalid_wavs": report.invalid_wav_count,
        "empty_wavs": report.empty_wav_count,
        "invalid_oto_timing": report.invalid_oto_timing_count,
        "sample_rates": report.sample_rates,
        "issues": issues,
    }))
}

fn diffsinger_validation_json(
    path: &PathBuf,
    voicebank: &Voicebank,
    config: &kamafeu::oto::DiffSingerConfig,
    verify_runtime: bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut report = diffsinger_info_json(path, voicebank, config);
    report["runtime_verified"] = Value::Bool(false);
    if verify_runtime {
        let runtime = kamafeu::dsp::diffsinger_runtime::DiffSingerRuntime::load(config)
            .map_err(Error::other)?;
        let contract = runtime.contract();
        report["runtime_verified"] = Value::Bool(true);
        report["model_contract"] = json!({
            "acoustic_inputs": contract.acoustic_inputs,
            "acoustic_outputs": contract.acoustic_outputs,
            "vocoder_inputs": contract.vocoder_inputs,
            "vocoder_outputs": contract.vocoder_outputs,
        });
    }
    Ok(report)
}

fn diffsinger_info_json(
    path: &PathBuf,
    voicebank: &Voicebank,
    config: &kamafeu::oto::DiffSingerConfig,
) -> Value {
    json!({
        "path": path,
        "type": "diffsinger",
        "healthy": true,
        "name": voicebank.name,
        "acoustic": config.acoustic,
        "vocoder": config.vocoder,
        "phonemes": config.phonemes,
        "languages": config.languages,
        "language_ids": config.language_ids,
        "speakers": config.speakers,
        "sample_rate": config.sample_rate,
        "vocoder_sample_rate": config.vocoder_sample_rate,
        "hop_size": config.hop_size,
        "num_mel_bins": config.num_mel_bins,
        "mel_base": config.mel_base,
    })
}

fn extension_catalog_json(
    path: &PathBuf,
    verify_wasm: bool,
    grant_permissions: &[String],
    format_filter: Option<&str>,
    phonemizer_filter: Option<&str>,
) -> Value {
    let discovery = discover(path);
    let built_ins = built_in_capabilities();
    let policy = permission_policy(grant_permissions.to_vec());
    let matches_filter = |extension: &&DiscoveredExtension| {
        format_filter.is_none_or(|value| extension.manifest.formats.contains(value))
            && phonemizer_filter.is_none_or(|value| extension.manifest.phonemizers.contains(value))
    };
    let filtered_extensions = discovery
        .extensions
        .iter()
        .filter(matches_filter)
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "path": path,
        "format_filter": format_filter,
        "phonemizer_filter": phonemizer_filter,
        "built_in_capabilities": built_ins.iter().map(|capability| json!({
            "id": capability.id,
            "name": capability.name,
            "kind": capability.kind,
            "description": capability.description,
        })).collect::<Vec<_>>(),
        "extensions": filtered_extensions.iter().map(|extension| json!({
            "manifest_path": extension.manifest_path,
            "manifest_fingerprint": extension
                .render_identity()
                .ok()
                .map(|identity| identity.manifest_fingerprint),
            "id": extension.manifest.id,
            "name": extension.manifest.name,
            "version": extension.manifest.version,
            "api_version": extension.manifest.api_version,
            "kind": extension.manifest.kind,
            "capabilities": extension.manifest.capabilities,
            "formats": extension.manifest.formats,
            "phonemizers": extension.manifest.phonemizers,
            "permissions": extension.manifest.permissions,
            "platforms": extension.manifest.platforms,
            "limitations": extension.manifest.limitations,
            "parameters": extension.manifest.parameters,
            "entrypoint": extension.manifest.entrypoint,
            "entrypoint_status": extension_entrypoint_status(extension),
            "platform_compatible": extension
                .manifest
                .supports_platform(kamafeu::extensions::current_platform()),
            "description": extension.manifest.description,
        })).collect::<Vec<_>>(),
        "diagnostics": discovery.diagnostics,
        "diagnostic_records": discovery.diagnostic_records,
        "wasm_verification": verify_wasm.then(|| wasm_verification_json(&filtered_extensions, &policy)),
    })
}

fn invoke_extension_json(
    path: &std::path::Path,
    id: &str,
    export_name: &str,
    argument: i32,
    bytes: bool,
    granted_permissions: &[String],
    parameters: &serde_json::Map<String, Value>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let discovery = discover(path);
    let extension = discovery
        .extensions
        .iter()
        .find(|extension| extension.manifest.id == id)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("extensão não encontrada: {id}"),
            )
        })?;
    let policy = permission_policy(granted_permissions.to_vec());
    if bytes {
        let output = kamafeu::extensions::invoke_wasm_bytes(
            extension,
            &policy,
            export_name,
            argument,
            parameters,
        )
        .map_err(Error::other)?;
        Ok(json!({
            "id": id,
            "export": export_name,
            "argument": argument,
            "bytes_length": output.len(),
            "utf8": String::from_utf8_lossy(&output),
            "hex": output.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        }))
    } else {
        let output = kamafeu::extensions::invoke_wasm_i32_with_parameters(
            extension,
            &policy,
            export_name,
            argument,
            parameters,
        )
        .map_err(Error::other)?;
        Ok(json!({
            "id": id,
            "export": export_name,
            "argument": argument,
            "result": output,
        }))
    }
}

fn extension_entrypoint_status(extension: &DiscoveredExtension) -> &'static str {
    let Some(entrypoint) = extension.manifest.entrypoint.as_deref() else {
        return "not_declared";
    };
    if !entrypoint.to_ascii_lowercase().ends_with(".wasm") {
        return "not_wasm";
    }
    let Some(root) = extension.manifest_path.parent() else {
        return "missing";
    };
    if root.join(entrypoint).is_file() {
        "available"
    } else {
        "missing"
    }
}

fn permission_policy(granted: Vec<String>) -> ExtensionPermissionPolicy {
    ExtensionPermissionPolicy {
        granted: granted.into_iter().collect(),
    }
}

fn parse_extension_parameters(
    values: &[String],
) -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let mut parameters = serde_json::Map::new();
    for value in values {
        let (name, raw) = value.split_once('=').ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("parâmetro de extensão inválido '{value}'; use chave=valor JSON"),
            )
        })?;
        if name.trim().is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "nome de parâmetro vazio").into());
        }
        let parsed = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
        if parameters.insert(name.to_string(), parsed).is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("parâmetro de extensão duplicado: '{name}'"),
            )
            .into());
        }
    }
    Ok(parameters)
}

fn wasm_verification_json(
    extensions: &[DiscoveredExtension],
    policy: &ExtensionPermissionPolicy,
) -> Vec<Value> {
    extensions
        .iter()
        .filter(|extension| {
            extension
                .manifest
                .entrypoint
                .as_deref()
                .is_some_and(|entrypoint| entrypoint.to_ascii_lowercase().ends_with(".wasm"))
        })
        .map(
            |extension| match verify_wasm_extension_with_policy(extension, policy) {
                Ok(info) => json!({
                    "id": info.id,
                    "ok": true,
                    "entrypoint": info.entrypoint,
                    "api_version": info.api_version,
                }),
                Err(error) => json!({ "id": extension.manifest.id, "ok": false, "error": error }),
            },
        )
        .collect()
}

fn load_project(path: &PathBuf) -> Result<UProject, Box<dyn std::error::Error>> {
    default_project_format_registry().load_file(path)
}

fn save_project(project: &UProject, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    default_project_format_registry().save_file(project, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kamafeu::extensions::{ExtensionKind, ExtensionManifest, EXTENSION_API_VERSION};

    #[test]
    fn extension_cli_parameters_parse_json_and_reject_duplicates() {
        let parameters = parse_extension_parameters(&[
            "mix=0.75".to_string(),
            "enabled=true".to_string(),
            "mode=fast".to_string(),
        ])
        .expect("parse extension parameters");
        assert_eq!(parameters["mix"], json!(0.75));
        assert_eq!(parameters["enabled"], json!(true));
        assert_eq!(parameters["mode"], json!("fast"));

        let duplicate = parse_extension_parameters(&["mix=1".to_string(), "mix=2".to_string()])
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
        let result = invoke_extension_json(
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
            extension_catalog_json(&directory.path().to_path_buf(), false, &[], None, None);
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
        assert_eq!(extension_entrypoint_status(&extension), "available");
        extension.manifest.entrypoint = Some("missing.wasm".to_string());
        assert_eq!(extension_entrypoint_status(&extension), "missing");
        extension.manifest.entrypoint = Some("plugin.txt".to_string());
        assert_eq!(extension_entrypoint_status(&extension), "not_wasm");
        extension.manifest.entrypoint = None;
        assert_eq!(extension_entrypoint_status(&extension), "not_declared");
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

        let report = convert_project(&input, &output).expect("convert and inspect");
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

        let report = convert_project(&input, &output).expect("convert UST");
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
            ConversionFingerprint::from_project(&source).note_count,
            ConversionFingerprint::from_project(&changed).note_count
        );
        assert_ne!(
            ConversionFingerprint::from_project(&source).note_content_signature,
            ConversionFingerprint::from_project(&changed).note_content_signature
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
    fn diff_cli_accepts_two_project_revisions() {
        let cli = Cli::try_parse_from(["kamafeu", "diff", "before.aps", "after.aps"])
            .expect("diff command");
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
        let result = handle_automation_request(AutomationRequest::Diff { before, after })
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

        let report = project_validation_json(&project_path, Some(&catalog)).expect("report");
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

        let value = handle_automation_request(AutomationRequest::ProjectInfo { path })
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

        let catalog = voicebank_catalog_json(vec![directory.path().to_path_buf()], false);
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

        let response = handle_automation_request(AutomationRequest::ApplyLyrics {
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

        let added = handle_automation_request(AutomationRequest::AddMarker {
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

        let removed = handle_automation_request(AutomationRequest::RemoveMarker {
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

        let added = handle_automation_request(AutomationRequest::AddSection {
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

        let removed = handle_automation_request(AutomationRequest::RemoveSection {
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
        handle_automation_request(AutomationRequest::CreateProject {
            output: created.clone(),
            name: Some("Automation composition".to_string()),
            bpm: Some(120.0),
        })
        .expect("create project");
        handle_automation_request(AutomationRequest::AddNote {
            input: created,
            output: with_note.clone(),
            lyric: "la".to_string(),
            pitch: "C4".to_string(),
            position_ms: 1_000.0,
            duration_ms: 500.0,
            track_index: Some(1),
        })
        .expect("add note");
        let result = handle_automation_request(AutomationRequest::SetTempo {
            input: with_note,
            output: retimed.clone(),
            bpm: 240.0,
        })
        .expect("set tempo");
        assert_eq!(result["time_scale"], 0.5);
        handle_automation_request(AutomationRequest::SetNote {
            input: retimed,
            output: edited.clone(),
            part_index: 1,
            note_index: 0,
            lyric: Some("li".to_string()),
            pitch: Some("D4".to_string()),
        })
        .expect("edit note");
        handle_automation_request(AutomationRequest::SetPitchBend {
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
        let response = handle_automation_request(AutomationRequest::SetExpression {
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

        let removed = handle_automation_request(AutomationRequest::RemoveNote {
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
        kamafeu::renderer::TrackRenderer::save_wav_samples(
            &audio_path,
            &[0.0, 0.1, -0.1, 0.0],
            44_100,
        )
        .expect("backing audio");
        save_project(&UProject::default(), &input).expect("project");

        let result = handle_automation_request(AutomationRequest::AddAudioPart {
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
        assert!(mcp_tools()
            .iter()
            .any(|tool| tool["name"] == "kamafeu_render"));
        let response = mcp_tool_call(
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

        let summary = render_project_to_file(
            &voicebank,
            &input,
            &output,
            RenderOutputFormat::Auto,
            true,
            44_100,
            false,
            false,
        )
        .expect("render");
        assert_eq!(summary.format, AudioExportFormat::Flac16);
        assert!(summary.frame_count > 0);
        assert!(summary.diagnostics.is_safe());
        assert!(!summary.silent);
        assert_eq!(fs::read(&output).expect("FLAC")[..4], *b"fLaC");
        let provenance = fs::read_to_string(output.with_extension("flac.kamafeu.json"))
            .expect("render provenance");
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

    #[test]
    fn diffsinger_validation_reports_neural_assets_not_an_empty_oto() {
        let directory = tempfile::tempdir().expect("temporary DiffSinger directory");
        fs::write(directory.path().join("phonemes.txt"), "a 1\n").expect("phonemes");
        fs::write(directory.path().join("acoustic.onnx"), b"model").expect("acoustic");
        fs::write(directory.path().join("vocoder.onnx"), b"model").expect("vocoder");
        fs::write(
            directory.path().join("dsconfig.yaml"),
            "phonemes: phonemes.txt\nacoustic: acoustic.onnx\nvocoder: vocoder.onnx\n",
        )
        .expect("configuration");

        let report = voicebank_validation_json(&directory.path().to_path_buf(), false)
            .expect("DiffSinger validation");
        assert_eq!(report["type"], "diffsinger");
        assert_eq!(report["healthy"], true);
        assert_eq!(report["hop_size"], 512);
    }
}
