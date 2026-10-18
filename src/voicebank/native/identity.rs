use super::NativeBank;
use std::sync::atomic::{AtomicBool, Ordering};

impl NativeBank {
    /// Hash the normalized manifest and referenced package content. This is
    /// intentionally an explicit operation: callers should compute it when
    /// binding/revalidating a bank, never on a GUI redraw or audio callback.
    pub fn content_sha256(&self) -> Result<String, String> {
        self.content_sha256_cancellable(None)
    }

    pub fn content_sha256_cancellable(
        &self,
        cancel: Option<&AtomicBool>,
    ) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let check_cancelled = || {
            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                Err("KFV 2 content hash cancelled".to_string())
            } else {
                Ok(())
            }
        };
        check_cancelled()?;

        let root = std::fs::canonicalize(&self.root)
            .map_err(|error| format!("cannot resolve voicebank root: {error}"))?;
        let mut hasher = Sha256::new();
        let manifest = serde_json::to_vec(&self.manifest)
            .map_err(|error| format!("cannot normalize voicebank manifest: {error}"))?;
        hash_record(&mut hasher, "kamafeu_voicebank.json", &manifest);

        for reference in &self.manifest.phonetic_profiles {
            check_cancelled()?;
            let profile = self
                .profiles
                .iter()
                .find(|profile| profile.id == reference.id)
                .ok_or_else(|| format!("missing loaded phonetic profile: {}", reference.id))?;
            let bytes = serde_json::to_vec(profile)
                .map_err(|error| format!("cannot normalize phonetic profile: {error}"))?;
            hash_record(&mut hasher, &reference.path, &bytes);
        }
        for (layer, units) in &self.layers {
            check_cancelled()?;
            let bytes = serde_json::to_vec(units)
                .map_err(|error| format!("cannot normalize layer units: {error}"))?;
            hash_record(&mut hasher, &layer.units_path, &bytes);
            for asset in &units.assets {
                let path = self.manifest.resolve(&root, &asset.path)?;
                let canonical = std::fs::canonicalize(&path)
                    .map_err(|error| format!("cannot resolve asset '{}': {error}", asset.path))?;
                if !canonical.starts_with(&root) || !canonical.is_file() {
                    return Err(format!("asset escapes the voicebank root: {}", asset.path));
                }
                let mut file = std::fs::File::open(&canonical)
                    .map_err(|error| format!("cannot open asset '{}': {error}", asset.path))?;
                let length = file
                    .metadata()
                    .map_err(|error| format!("cannot inspect asset '{}': {error}", asset.path))?
                    .len();
                hasher.update((asset.path.len() as u64).to_be_bytes());
                hasher.update(asset.path.as_bytes());
                hasher.update(length.to_be_bytes());
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    check_cancelled()?;
                    let read = file
                        .read(&mut buffer)
                        .map_err(|error| format!("cannot hash asset '{}': {error}", asset.path))?;
                    if read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..read]);
                }
            }
        }
        let mut optional_files = std::collections::BTreeMap::new();
        if let Some(license) = &self.manifest.license {
            optional_files.insert(license.path.clone(), "license");
        }
        if let Some(presentation) = &self.manifest.presentation {
            for relative in [presentation.avatar.as_ref(), presentation.portrait.as_ref()]
                .into_iter()
                .flatten()
            {
                optional_files.insert(relative.clone(), "presentation asset");
            }
        }
        for (relative, name) in optional_files {
            check_cancelled()?;
            let path = self.manifest.resolve(&root, &relative)?;
            let canonical = std::fs::canonicalize(&path)
                .map_err(|error| format!("cannot resolve {name} '{}': {error}", relative))?;
            if !canonical.starts_with(&root) || !canonical.is_file() {
                return Err(format!("{name} escapes the voicebank root: {relative}"));
            }
            let mut file = std::fs::File::open(&canonical)
                .map_err(|error| format!("cannot read {name} '{}': {error}", relative))?;
            let length = file
                .metadata()
                .map_err(|error| format!("cannot inspect {name} '{}': {error}", relative))?
                .len();
            hasher.update((relative.len() as u64).to_be_bytes());
            hasher.update(relative.as_bytes());
            hasher.update(length.to_be_bytes());
            let mut buffer = [0u8; 16 * 1024];
            loop {
                check_cancelled()?;
                let read = file
                    .read(&mut buffer)
                    .map_err(|error| format!("cannot read {name} '{}': {error}", relative))?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
        }
        check_cancelled()?;
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    pub fn project_reference(
        &self,
        source_path_hint: Option<String>,
    ) -> Result<crate::project::NativeVoicebankReference, String> {
        Ok(crate::project::NativeVoicebankReference {
            bank_id: self.manifest.bank_id.clone(),
            bank_version: self.manifest.bank_version.clone(),
            package_revision: self.manifest.package_revision,
            content_sha256: self.content_sha256()?,
            engine_id: self.manifest.engine.id.clone(),
            engine_api_major: self.manifest.engine.api_major,
            planner_version: "kamafeu.planner.v1".into(),
            phonetic_profile_id: self.manifest.defaults.phonetic_profile_id.clone(),
            style_id: self.manifest.defaults.style_id.clone(),
            source_path_hint,
        })
    }
}

fn hash_record(hasher: &mut sha2::Sha256, name: &str, bytes: &[u8]) {
    use sha2::Digest;
    hasher.update((name.len() as u64).to_be_bytes());
    hasher.update(name.as_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}
