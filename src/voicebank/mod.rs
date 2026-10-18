//! Versioned voicebank contracts. The classic loader for oto.ini banks remains in `crate::oto`.
pub mod migration;
pub mod native;
pub mod package;
pub mod planner;
pub mod project;
pub mod render;

use std::path::Path;

#[derive(Debug, Clone)]
// Boxing a variant would change the public enum's construction and pattern API.
#[allow(clippy::large_enum_variant)]
pub enum VoicebankHandle {
    Classic(crate::oto::Voicebank),
    NativeV2(native::NativeBank),
}

impl VoicebankHandle {
    /// Format-aware facade. Native manifests are fully validated before the
    /// caller receives a KFV 2 handle; classic folders and legacy packages
    /// keep using the existing loader for oto.ini banks and Copaíba.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        match native::NativeBank::load_path(path) {
            Ok(bank) => Ok(Self::NativeV2(bank)),
            Err(native_error) => match crate::oto::Voicebank::new(path) {
                Ok(bank) => Ok(Self::Classic(bank)),
                Err(classic_error) => Err(format!(
                    "voicebank load failed (native: {native_error}; classic: {classic_error})"
                )),
            },
        }
    }
}
