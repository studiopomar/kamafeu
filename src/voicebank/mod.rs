//! Versioned voicebank contracts. The classic loader for oto.ini banks remains in `crate::oto`.
pub mod cantor;
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
    Cantor(cantor::CantorBank),
}

impl VoicebankHandle {
    /// Format-aware facade. Native manifests are fully validated before the
    /// caller receives a KFV 2 handle; classic folders and legacy packages
    /// keep using the existing loader for oto.ini banks and Copaíba.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        if path.is_dir() {
            let cantor_config = path.join("cantor.toml");
            let oto_config = path.join("oto.ini");
            if cantor_config.exists() && oto_config.exists() {
                return Err("conflito de formato: o banco contém cantor.toml e oto.ini".into());
            }
            if cantor_config.exists() {
                return cantor::CantorBank::load_path(path).map(Self::Cantor);
            }
            if oto_config.exists() {
                return crate::oto::Voicebank::new(path)
                    .map(Self::Classic)
                    .map_err(|error| error.to_string());
            }
            if path.join("kamafeu_voicebank.json").exists() {
                return Err(
                    "formato nativo JSON legado não é compatível; bancos nativos usam cantor.toml"
                        .into(),
                );
            }
            return Err(
                "formato de voicebank não identificado: esperado cantor.toml ou oto.ini".into(),
            );
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("kfv"))
        {
            return cantor::CantorBank::load_path(path).map(Self::Cantor);
        }
        Err(format!(
            "formato de voicebank não aceito: {}",
            path.display()
        ))
    }
}

#[cfg(test)]
mod format_detection_tests {
    use super::*;

    #[test]
    fn cantor_toml_is_a_strict_discriminator_and_conflicts_are_reported() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("cantor.toml"),
            "versao = 999\nid = 'x'\nnome = 'x'\n",
        )
        .unwrap();
        assert!(VoicebankHandle::open(root.path())
            .err()
            .unwrap()
            .contains("versão cantor.toml"));
        std::fs::write(root.path().join("oto.ini"), "").unwrap();
        assert!(VoicebankHandle::open(root.path())
            .err()
            .unwrap()
            .contains("conflito de formato"));
    }
}
