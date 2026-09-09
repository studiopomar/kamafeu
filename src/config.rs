use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

fn default_scale() -> f32 {
    1.0
}

fn default_sample_rate() -> u32 {
    44100
}

fn default_buffer_size() -> u32 {
    512
}

fn default_master_vol() -> f32 {
    1.0
}

fn default_channel_mode() -> String {
    "Estéreo (L+R)".to_string()
}

fn default_buffer_periods() -> u32 {
    2
}

fn default_dither_str() -> String {
    "TPDF (Triangular)".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceConfig {
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "default_buffer_size")]
    pub buffer_size_frames: u32,
    #[serde(default = "default_buffer_periods")]
    pub buffer_periods: u32,
    #[serde(default = "default_channel_mode")]
    pub channel_mode: String,
    #[serde(default)]
    pub exclusive_mode: bool,
    #[serde(default = "default_master_vol")]
    pub master_volume: f32,
    #[serde(default)]
    pub preamp_gain_db: f32,
    #[serde(default)]
    pub latency_compensation_ms: f32,
    #[serde(default = "default_dither_str")]
    pub dither_algorithm: String,
    #[serde(default = "default_true")]
    pub auto_mute_on_device_change: bool,
    #[serde(default = "default_true")]
    pub high_quality_resampling: bool,
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        Self {
            device_name: None,
            sample_rate: 44100,
            buffer_size_frames: 512,
            buffer_periods: 2,
            channel_mode: default_channel_mode(),
            exclusive_mode: false,
            master_volume: 1.0,
            preamp_gain_db: 0.0,
            latency_compensation_ms: 0.0,
            dither_algorithm: default_dither_str(),
            auto_mute_on_device_change: true,
            high_quality_resampling: true,
        }
    }
}

fn default_resampler_str() -> String {
    "Native Hybrid TD-PSOLA".to_string()
}

fn default_wavtool_str() -> String {
    "Andromeda (Nativo)".to_string()
}

fn default_pitch_step() -> f32 {
    4.0
}

fn default_crossfade_ms() -> f32 {
    15.0
}

fn default_lookahead_ms() -> f32 {
    2000.0
}

fn default_interpolation_str() -> String {
    "Cosine".to_string()
}

fn default_crossfade_curve() -> String {
    "Equal Power S-Curve".to_string()
}

fn default_thread_priority() -> String {
    "Normal".to_string()
}

fn default_render_chunk_bars() -> u32 {
    4
}

fn default_f0_method() -> String {
    "YIN (Pitch Fundamental Adaptativo)".to_string()
}

fn default_f0_min() -> f32 {
    40.0
}

fn default_f0_max() -> f32 {
    1400.0
}

fn default_fft_window() -> String {
    "Blackman-Harris (4-term)".to_string()
}

fn default_formant_mode() -> String {
    "LPC Spectral Envelope".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DspEngineConfig {
    #[serde(default)]
    pub render_threads: u32,
    #[serde(default = "default_thread_priority")]
    pub thread_priority: String,
    #[serde(default = "default_resampler_str")]
    pub default_resampler: String,
    #[serde(default = "default_wavtool_str")]
    pub default_wavtool: String,
    #[serde(default = "default_pitch_step")]
    pub pitch_curve_step_ms: f32,
    #[serde(default = "default_interpolation_str")]
    pub pitch_interpolation: String,
    #[serde(default = "default_true")]
    pub anti_aliasing_filter: bool,
    #[serde(default = "default_crossfade_ms")]
    pub crossfade_window_ms: f32,
    #[serde(default = "default_crossfade_curve")]
    pub crossfade_curve: String,
    #[serde(default = "default_lookahead_ms")]
    pub render_lookahead_ms: f32,
    #[serde(default = "default_render_chunk_bars")]
    pub render_chunk_bars: u32,
    #[serde(default = "default_f0_method")]
    pub f0_detection_method: String,
    #[serde(default = "default_f0_min")]
    pub f0_min_hz: f32,
    #[serde(default = "default_f0_max")]
    pub f0_max_hz: f32,
    #[serde(default = "default_fft_window")]
    pub fft_window_type: String,
    #[serde(default)]
    pub oversampling_factor: u32,
    #[serde(default = "default_formant_mode")]
    pub formant_preservation_mode: String,
    #[serde(default = "default_true")]
    pub background_prerender: bool,
    #[serde(default)]
    pub verbose_dsp_logging: bool,
    #[serde(default)]
    pub custom_wine_prefix: Option<PathBuf>,
}

impl Default for DspEngineConfig {
    fn default() -> Self {
        Self {
            render_threads: 0,
            thread_priority: default_thread_priority(),
            default_resampler: default_resampler_str(),
            default_wavtool: default_wavtool_str(),
            pitch_curve_step_ms: 4.0,
            pitch_interpolation: default_interpolation_str(),
            anti_aliasing_filter: true,
            crossfade_window_ms: 15.0,
            crossfade_curve: default_crossfade_curve(),
            render_lookahead_ms: 2000.0,
            render_chunk_bars: 4,
            f0_detection_method: default_f0_method(),
            f0_min_hz: default_f0_min(),
            f0_max_hz: default_f0_max(),
            fft_window_type: default_fft_window(),
            oversampling_factor: 1,
            formant_preservation_mode: default_formant_mode(),
            background_prerender: true,
            verbose_dsp_logging: false,
            custom_wine_prefix: None,
        }
    }
}

fn default_ram_cache_mb() -> usize {
    1024
}

fn default_disk_cache_mb() -> usize {
    4096
}

fn default_waveform_res() -> u32 {
    2
}

fn default_ram_compression() -> String {
    "Nenhuma (Float32)".to_string()
}

fn default_preload_strat() -> String {
    "Lazy On-Demand".to_string()
}

fn default_cleanup_days() -> u32 {
    7
}

fn default_stream_thresh() -> f32 {
    16.0
}

fn default_io_threads() -> u32 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCacheConfig {
    #[serde(default = "default_ram_cache_mb")]
    pub max_ram_cache_mb: usize,
    #[serde(default = "default_disk_cache_mb")]
    pub max_disk_cache_mb: usize,
    #[serde(default = "default_ram_compression")]
    pub ram_cache_compression: String,
    #[serde(default = "default_preload_strat")]
    pub ram_preload_strategy: String,
    #[serde(default = "default_io_threads")]
    pub io_thread_concurrency: u32,
    #[serde(default)]
    pub custom_cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub clear_cache_on_exit: bool,
    #[serde(default = "default_cleanup_days")]
    pub auto_cleanup_days: u32,
    #[serde(default = "default_waveform_res")]
    pub waveform_cache_resolution: u32,
    #[serde(default = "default_stream_thresh")]
    pub stream_threshold_mb: f32,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            max_ram_cache_mb: 1024,
            max_disk_cache_mb: 4096,
            ram_cache_compression: default_ram_compression(),
            ram_preload_strategy: default_preload_strat(),
            io_thread_concurrency: 4,
            custom_cache_dir: None,
            clear_cache_on_exit: false,
            auto_cleanup_days: 7,
            waveform_cache_resolution: 2,
            stream_threshold_mb: 16.0,
        }
    }
}

fn default_export_fmt() -> String {
    "WAV (Lossless PCM)".to_string()
}

fn default_bit_depth() -> u16 {
    16
}

fn default_dither_mode() -> String {
    "TPDF (Triangular)".to_string()
}

fn default_normalize_db() -> f32 {
    -0.5
}

fn default_tail_ms() -> f32 {
    500.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDefaultsConfig {
    #[serde(default = "default_export_fmt")]
    pub format: String,
    #[serde(default = "default_bit_depth")]
    pub bit_depth: u16,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "default_dither_mode")]
    pub dither_mode: String,
    #[serde(default)]
    pub normalize_audio: bool,
    #[serde(default = "default_normalize_db")]
    pub normalize_peak_db: f32,
    #[serde(default = "default_tail_ms")]
    pub tail_silence_ms: f32,
    #[serde(default)]
    pub head_silence_ms: f32,
    #[serde(default)]
    pub export_stems_default: bool,
    #[serde(default = "default_true")]
    pub include_instrumental: bool,
    #[serde(default = "default_true")]
    pub embed_project_metadata: bool,
}

impl Default for ExportDefaultsConfig {
    fn default() -> Self {
        Self {
            format: default_export_fmt(),
            bit_depth: 16,
            sample_rate: 44100,
            dither_mode: default_dither_mode(),
            normalize_audio: false,
            normalize_peak_db: -0.5,
            tail_silence_ms: 500.0,
            head_silence_ms: 0.0,
            export_stems_default: false,
            include_instrumental: true,
            embed_project_metadata: true,
        }
    }
}

fn default_autosave_sec() -> u32 {
    300
}

fn default_max_undo() -> usize {
    100
}

fn default_grid_snap_str() -> String {
    "1/16".to_string()
}

fn default_autoscroll_str() -> String {
    "Seguir Cabeça (Página)".to_string()
}

fn default_note_duration_ms() -> f64 {
    480.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorWorkflowConfig {
    #[serde(default = "default_true")]
    pub auto_save_enabled: bool,
    #[serde(default = "default_autosave_sec")]
    pub auto_save_interval_sec: u32,
    #[serde(default = "default_true")]
    pub backup_on_save: bool,
    #[serde(default = "default_max_undo")]
    pub max_undo_steps: usize,
    #[serde(default = "default_true")]
    pub note_audition_on_click: bool,
    #[serde(default)]
    pub audition_on_lyric_change: bool,
    #[serde(default = "default_note_duration_ms")]
    pub default_note_duration_ms: f64,
    #[serde(default = "default_grid_snap_str")]
    pub default_grid_snap: String,
    #[serde(default = "default_autoscroll_str")]
    pub default_auto_scroll: String,
    #[serde(default = "default_true")]
    pub confirm_on_exit_dirty: bool,
}

impl Default for EditorWorkflowConfig {
    fn default() -> Self {
        Self {
            auto_save_enabled: true,
            auto_save_interval_sec: 300,
            backup_on_save: true,
            max_undo_steps: 100,
            note_audition_on_click: true,
            audition_on_lyric_change: false,
            default_note_duration_ms: 480.0,
            default_grid_snap: default_grid_snap_str(),
            default_auto_scroll: default_autoscroll_str(),
            confirm_on_exit_dirty: true,
        }
    }
}

fn default_alias_strat() -> String {
    "Prioritário (Prefixo/Sufixo -> Exato -> Romaji)".to_string()
}

fn default_consonant_stretch() -> String {
    "Adaptive SOLA".to_string()
}

fn default_one_f32() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicebankTuningConfig {
    #[serde(default = "default_alias_strat")]
    pub alias_search_strategy: String,
    #[serde(default = "default_consonant_stretch")]
    pub consonant_stretch_mode: String,
    #[serde(default = "default_true")]
    pub auto_reload_oto: bool,
    #[serde(default = "default_true")]
    pub recursive_voicebank_scan: bool,
    #[serde(default)]
    pub strict_oto_parsing: bool,
    #[serde(default = "default_true")]
    pub auto_phonetic_g2p: bool,
    #[serde(default = "default_one_f32")]
    pub preutterance_scale: f32,
    #[serde(default = "default_one_f32")]
    pub overlap_scale: f32,
    #[serde(default = "default_one_f32")]
    pub fixed_consonant_scale: f32,
    #[serde(default = "default_one_f32")]
    pub vowel_crossfade_slope: f32,
    #[serde(default)]
    pub breath_noise_reduction_db: f32,
    #[serde(default)]
    pub auto_pitch_correction_strength: f32,
}

impl Default for VoicebankTuningConfig {
    fn default() -> Self {
        Self {
            alias_search_strategy: default_alias_strat(),
            consonant_stretch_mode: default_consonant_stretch(),
            auto_reload_oto: true,
            recursive_voicebank_scan: true,
            strict_oto_parsing: false,
            auto_phonetic_g2p: true,
            preutterance_scale: 1.0,
            overlap_scale: 1.0,
            fixed_consonant_scale: 1.0,
            vowel_crossfade_slope: 1.0,
            breath_noise_reduction_db: 0.0,
            auto_pitch_correction_strength: 0.0,
        }
    }
}

fn default_simd_str() -> String {
    "Auto (AVX2 / NEON / SSE)".to_string()
}

fn default_wine_debug_str() -> String {
    "fixme-all,err-all".to_string()
}

fn default_ipc_timeout() -> u32 {
    4000
}

fn default_pipe_buffer() -> u32 {
    64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalConfig {
    #[serde(default)]
    pub async_ipc_rendering: bool,
    #[serde(default)]
    pub force_opengl_fallback: bool,
    #[serde(default)]
    pub vst3_plugin_support: bool,
    #[serde(default = "default_simd_str")]
    pub simd_mode: String,
    #[serde(default)]
    pub realtime_thread_affinity: bool,
    #[serde(default = "default_ipc_timeout")]
    pub resampler_ipc_timeout_ms: u32,
    #[serde(default = "default_pipe_buffer")]
    pub pipe_buffer_kb: u32,
    #[serde(default)]
    pub dump_chunks_debug: bool,
    #[serde(default)]
    pub trace_dsp_timing: bool,
    #[serde(default = "default_true")]
    pub flush_denormals_to_zero: bool,
    #[serde(default)]
    pub custom_plugin_paths: Vec<PathBuf>,
    #[serde(default = "default_wine_debug_str")]
    pub wine_debug_channel: String,
}

impl Default for ExperimentalConfig {
    fn default() -> Self {
        Self {
            async_ipc_rendering: false,
            force_opengl_fallback: false,
            vst3_plugin_support: false,
            simd_mode: default_simd_str(),
            realtime_thread_affinity: false,
            resampler_ipc_timeout_ms: default_ipc_timeout(),
            pipe_buffer_kb: default_pipe_buffer(),
            dump_chunks_debug: false,
            trace_dsp_timing: false,
            flush_denormals_to_zero: true,
            custom_plugin_paths: Vec::new(),
            wine_debug_channel: default_wine_debug_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppLanguage {
    #[default]
    PtBr, // Brasileiro (Brasil)
    EnUs, // Inglês (Global)
}

impl AppLanguage {
    pub fn display_name(&self) -> &'static str {
        match self {
            AppLanguage::PtBr => "Brasileiro (Brasil)",
            AppLanguage::EnUs => "Inglês (Global)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KamafeuConfig {
    #[serde(default)]
    pub language: AppLanguage,
    pub last_voicebank: Option<PathBuf>,
    #[serde(default)]
    pub recent_voicebanks: Vec<PathBuf>,
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
    #[serde(default)]
    pub singers_paths: Vec<PathBuf>,
    #[serde(default = "default_true")]
    pub discord_rpc_enabled: bool,
    #[serde(default = "default_scale")]
    pub ui_scale_factor: f32,
    #[serde(default)]
    pub theme: crate::gui::theme::ThemeConfig,
    #[serde(default)]
    pub audio: AudioDeviceConfig,
    #[serde(default)]
    pub dsp: DspEngineConfig,
    #[serde(default)]
    pub memory: MemoryCacheConfig,
    #[serde(default)]
    pub export: ExportDefaultsConfig,
    #[serde(default)]
    pub workflow: EditorWorkflowConfig,
    #[serde(default)]
    pub voicebank_tuning: VoicebankTuningConfig,
    #[serde(default)]
    pub experimental: ExperimentalConfig,
}

impl Default for KamafeuConfig {
    fn default() -> Self {
        let mut default_singers = crate::oto::SingerScanner::default_singers_directories();
        default_singers.dedup();
        Self {
            language: AppLanguage::default(),
            last_voicebank: None,
            recent_voicebanks: Vec::new(),
            recent_projects: Vec::new(),
            singers_paths: default_singers,
            discord_rpc_enabled: true,
            ui_scale_factor: 1.0,
            theme: crate::gui::theme::ThemeConfig::default(),
            audio: AudioDeviceConfig::default(),
            dsp: DspEngineConfig::default(),
            memory: MemoryCacheConfig::default(),
            export: ExportDefaultsConfig::default(),
            workflow: EditorWorkflowConfig::default(),
            voicebank_tuning: VoicebankTuningConfig::default(),
            experimental: ExperimentalConfig::default(),
        }
    }
}

impl KamafeuConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join(".config").join("kamafeu");
            let _ = fs::create_dir_all(&dir);
            dir.join("kamafeu_config.json")
        } else {
            PathBuf::from("kamafeu_config.json")
        }
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Falha ao ler {}: {e}", path.display()))?;
        serde_json::from_str::<KamafeuConfig>(&content)
            .map_err(|e| format!("Configuração inválida em {}: {e}", path.display()))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Falha ao serializar configuração: {e}"))?;
        let parent = path
            .parent()
            .ok_or_else(|| "Caminho de configuração inválido".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("Falha ao criar {}: {e}", parent.display()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|e| format!("Falha ao criar arquivo temporário: {e}"))?;
        temporary
            .write_all(content.as_bytes())
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|e| format!("Falha ao gravar configuração: {e}"))?;
        temporary
            .persist(&path)
            .map_err(|e| format!("Falha ao substituir {}: {}", path.display(), e.error))?;
        Ok(())
    }

    pub fn add_recent_voicebank(&mut self, path: PathBuf) {
        if !path.exists() {
            return;
        }
        self.recent_voicebanks.retain(|p| p != &path);
        self.recent_voicebanks.insert(0, path.clone());
        self.recent_voicebanks.truncate(10);
        self.last_voicebank = Some(path);
        if let Err(error) = self.save() {
            eprintln!("[Kamafeu] {error}");
        }
    }

    pub fn add_recent_project(&mut self, path: PathBuf) {
        if !path.exists() {
            return;
        }
        self.recent_projects.retain(|p| p != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(10);
        if let Err(error) = self.save() {
            eprintln!("[Kamafeu] {error}");
        }
    }
}
