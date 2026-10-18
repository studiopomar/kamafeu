pub mod chunked;
pub mod diagnostics;
mod execution;
pub mod exporter;
mod mix;
pub mod options;
pub mod phase;
mod preview;
pub mod progress;
mod progressive;
pub mod project;
pub mod provenance;
pub mod resampler_cache;
pub mod timing;
pub mod track;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod worker_pool;

/// Callback shape shared by synchronous render APIs that report progress and
/// diagnostic lines to the caller.
pub type RenderProgressCallback<'a> = dyn Fn(f32, &str) + Send + Sync + 'a;

pub use chunked::ChunkedRenderer;
pub use diagnostics::AudioDiagnostics;
pub use exporter::{AudioExportFormat, AudioExporter, DitherMode, MasteringOptions};
pub use options::{CompatibilityMode, RenderOptions, VocalRenderPreset};
pub use preview::PreviewTarget;
pub use progress::{RenderProgressEvent, RenderStage};
pub use project::{NativeTrackRender, ProgressiveChunk, ProjectRenderer, RenderedAudio};
pub use provenance::{
    PhonemeRenderDiagnostic, PhonemeResolutionDiagnostic, RenderExtension, RenderProvenance,
    PROVENANCE_SCHEMA, RENDER_PIPELINE_VERSION,
};
pub use track::TrackRenderer;
