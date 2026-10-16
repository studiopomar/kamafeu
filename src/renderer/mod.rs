pub mod chunked;
pub mod diagnostics;
pub mod exporter;
pub mod options;
pub mod phase;
pub mod progress;
pub mod project;
pub mod provenance;
pub mod resampler_cache;
pub mod timing;
pub mod track;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod worker_pool;

pub use chunked::ChunkedRenderer;
pub use diagnostics::AudioDiagnostics;
pub use exporter::{AudioExportFormat, AudioExporter, DitherMode, MasteringOptions};
pub use options::{CompatibilityMode, RenderOptions, VocalRenderPreset};
pub use progress::{RenderProgressEvent, RenderStage};
pub use project::{
    NativeTrackRender, PreviewTarget, ProgressiveChunk, ProjectRenderer, RenderedAudio,
};
pub use provenance::{
    PhonemeRenderDiagnostic, PhonemeResolutionDiagnostic, RenderExtension, RenderProvenance,
    PROVENANCE_SCHEMA, RENDER_PIPELINE_VERSION,
};
pub use track::TrackRenderer;
