use clap::{Parser, Subcommand};
use kamafeu::renderer::AudioExportFormat;
use serde_json::Value;
use std::path::PathBuf;

fn default_pitch_shape() -> String {
    "s".to_string()
}

#[derive(Parser)]
#[command(name = "kamafeu")]
#[command(about = "OpenUTAU-style voice synthesizer core and Piano Roll GUI in Rust", long_about = None)]
pub(super) struct Cli {
    /// Emit machine-readable JSON for inspection and validation commands
    #[arg(long, global = true)]
    pub(super) json: bool,
    #[command(subcommand)]
    pub(super) command: Option<Commands>,
}

#[derive(Subcommand)]
pub(super) enum Commands {
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

    /// List UTAU and OpenUTAU voicebanks found in singer folders
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
    },

    /// Convert an oto.ini voicebank into a separate, reviewable KFV 2 draft
    MigrateVoicebank {
        /// Path to the classic voicebank root folder
        path: PathBuf,

        /// Destination directory for the new KFV 2 bank; it must not exist
        destination: PathBuf,

        /// KFV 2 phonetic profile JSON
        #[arg(long)]
        profile: PathBuf,

        /// Optional JSON object mapping exact oto.ini aliases to phone arrays
        #[arg(long)]
        mapping: Option<PathBuf>,

        /// MIDI note number assigned to the single imported pitch layer
        #[arg(long, default_value_t = 60)]
        midi: u8,
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

        /// Use an external classic resampler executable (for example straycat-rs)
        #[arg(long)]
        resampler: Option<PathBuf>,

        /// Ignore instrumental wave parts while diagnosing vocal rendering
        #[arg(long)]
        vocal_only: bool,
    },
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenderOutputFormat {
    Auto,
    Wav16,
    Wav24,
    Wav32Float,
    Flac16,
    Flac24,
    RawF32,
}

impl RenderOutputFormat {
    pub(super) fn resolve(self, output: &std::path::Path) -> Result<AudioExportFormat, String> {
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
pub(super) enum AutomationRequest {
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
pub(super) struct AutomationPitchPoint {
    pub(super) time_offset_ms: f64,
    pub(super) pitch_offset_cents: f64,
    #[serde(default = "default_pitch_shape")]
    pub(super) shape: String,
}
