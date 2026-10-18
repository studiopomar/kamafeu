use super::NativeBank;
use crate::voicebank::native::{Manifest, PhoneticProfile, UnitFile};
use std::path::Path;
use std::sync::Arc;

impl NativeBank {
    /// Load either an authoring folder or a packaged `.kfv` snapshot.
    pub fn load_path(path: &Path) -> Result<Self, String> {
        if path.is_dir() {
            return Self::load_dir(path);
        }
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("kfv"))
        {
            return Err(format!(
                "KFV 2 path is not a folder or .kfv package: {}",
                path.display()
            ));
        }
        let temp_dir = tempfile::Builder::new()
            .prefix("kamafeu_kfv2_")
            .tempdir()
            .map_err(|e| format!("failed to create package workspace: {e}"))?;
        crate::oto::voicebank::extract_kfv(path, temp_dir.path())
            .map_err(|e| format!("failed to extract KFV package: {e}"))?;
        let mut bank = Self::load_dir(temp_dir.path())?;
        bank.temp_dir = Some(Arc::new(temp_dir));
        Ok(bank)
    }

    /// Load and cross-validate a KFV 2 folder. Audio is inspected by header;
    /// the sample payload is not decoded or cached at this stage.
    pub fn load_dir(root: &Path) -> Result<Self, String> {
        let root = std::fs::canonicalize(root)
            .map_err(|error| format!("cannot resolve voicebank folder: {error}"))?;
        if !root.is_dir() {
            return Err("KFV 2 voicebank root is not a directory".into());
        }
        let manifest_path = root.join("kamafeu_voicebank.json");
        let manifest_text = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
        let manifest = Manifest::parse_json(&manifest_text)?;
        let mut profiles = Vec::new();
        for reference in &manifest.phonetic_profiles {
            let path = manifest.resolve_existing(&root, &reference.path)?;
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let profile: PhoneticProfile =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            profile.validate()?;
            if profile.id != reference.id || profile.language != reference.language {
                return Err(format!(
                    "phonetic profile identity mismatch: {}",
                    reference.path
                ));
            }
            profiles.push(profile);
        }
        let mut layers = Vec::new();
        for layer in &manifest.layers {
            let path = manifest.resolve_existing(&root, &layer.units_path)?;
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let units: UnitFile =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let profile = profiles
                .iter()
                .find(|p| p.id == manifest.defaults.phonetic_profile_id)
                .ok_or_else(|| "default phonetic profile is missing".to_string())?;
            units.validate(&layer.id, profile)?;
            for asset in &units.assets {
                let asset_path = manifest.resolve_existing(&root, &asset.path)?;
                let reader = hound::WavReader::open(&asset_path)
                    .map_err(|e| format!("{}: WAV inválido: {e}", asset_path.display()))?;
                let spec = reader.spec();
                let frames = reader.duration() as u64;
                if spec.channels != asset.channels
                    || spec.sample_rate != asset.sample_rate_hz
                    || frames != asset.frame_count
                {
                    return Err(format!(
                        "{}: metadados WAV divergem do manifesto",
                        asset.path
                    ));
                }
            }
            layers.push((layer.clone(), units));
        }
        if let Some(presentation) = &manifest.presentation {
            for path in [
                presentation.avatar.as_deref(),
                presentation.portrait.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                manifest.resolve_existing(&root, path)?;
            }
        }
        if let Some(license) = &manifest.license {
            manifest.resolve_existing(&root, &license.path)?;
        }
        Ok(Self {
            root,
            manifest,
            profiles,
            layers,
            temp_dir: None,
        })
    }
}
