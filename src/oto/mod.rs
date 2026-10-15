pub mod entry;
pub mod parser;
pub mod prefix_map;
pub mod singers;
pub mod voicebank;
#[path = "diagnostics.rs"]
pub mod voicebank_diagnostics;

pub use entry::OtoEntry;
pub use parser::{OtoParseDiagnostic, OtoParser};
pub use prefix_map::PrefixMap;
pub use singers::{SingerInfo, SingerScanner};
pub use voicebank::{AliasResolutionReport, Voicebank};
pub use voicebank_diagnostics::{VoicebankDiagnosticReport, VoicebankIssue, VoicebankIssueKind};
