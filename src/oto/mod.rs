pub mod diffsinger;
pub mod entry;
pub mod parser;
pub mod prefix_map;
pub mod singers;
pub mod voicebank;
#[path = "diagnostics.rs"]
pub mod voicebank_diagnostics;

pub use diffsinger::{dependency_install_message, DiffSingerConfig, DIFFSINGER_DEPENDENCY_URL};
pub use entry::OtoEntry;
pub use parser::OtoParser;
pub use prefix_map::PrefixMap;
pub use singers::{SingerInfo, SingerScanner};
pub use voicebank::{AliasResolutionReport, Voicebank};
pub use voicebank_diagnostics::{VoicebankDiagnosticReport, VoicebankIssue, VoicebankIssueKind};
