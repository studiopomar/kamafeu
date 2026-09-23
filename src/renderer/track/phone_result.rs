use crate::drivers::WavtoolArgs;

/// Complete result of rendering one phone before the ordered track merge.
/// Keeping this transport object separate makes the parallel synthesis phase
/// explicit without exposing it outside the track renderer.
pub(super) struct PhoneResult {
    pub(super) idx: usize,
    pub(super) note_rendered: Vec<f32>,
    pub(super) actual_start_ms: f64,
    pub(super) source_skip_ms: f64,
    pub(super) crossfade_ms: f64,
    pub(super) pitch_freq: f64,
    pub(super) logs: Vec<(f32, String)>,
    pub(super) wav_args: WavtoolArgs,
    pub(super) adjacent: bool,
}
