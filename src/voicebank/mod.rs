//! Versioned voicebank contracts. The classic loader for oto.ini banks remains in `crate::oto`.
pub mod cantor;
pub mod diffsinger;
#[cfg(not(target_arch = "wasm32"))]
pub mod diffsinger_render;
pub mod migration;
pub mod native;
pub mod package;
pub mod planner;
pub mod project;
pub mod render;

use std::path::Path;

#[derive(Debug, Clone)]
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
        if path.is_dir() && path.join("cantor.toml").exists() {
            let bank = cantor::CantorBank::load_path(path)?;
            return Ok(Self::NativeV2(bank.native_snapshot()?));
        }
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
