use super::*;

impl CopaibaNativeProject {
    pub fn load_dir(root: &Path) -> Result<Self, String> {
        let bank = NativeBank::load_dir(root)?;
        Ok(Self {
            root: bank.root,
            manifest: bank.manifest,
            profiles: bank.profiles,
            layers: bank.layers,
            _workspace: None,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    /// Open a folder or immutable `.kfv` snapshot for editing. Packages are
    /// extracted into a private workspace; saving edits only that workspace
    /// and publishing a new draft never mutates the source package.
    pub fn load_path(path: &Path) -> Result<Self, String> {
        if path.is_dir() {
            return Self::load_dir(path);
        }
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("kfv"))
        {
            return Err("Copaíba KFV 2 expects a folder or .kfv package".into());
        }
        let workspace = tempfile::Builder::new()
            .prefix("copaiba_kfv2_edit_")
            .tempdir()
            .map_err(|error| format!("cannot create KFV 2 editing workspace: {error}"))?;
        crate::oto::voicebank::extract_kfv(path, workspace.path())
            .map_err(|error| format!("invalid KFV 2 package: {error}"))?;
        let mut project = Self::load_dir(workspace.path())?;
        project._workspace = Some(Arc::new(workspace));
        Ok(project)
    }

    pub fn save_to_dir(&mut self, root: &Path) -> Result<(), String> {
        self.manifest.validate()?;
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == self.manifest.defaults.phonetic_profile_id)
            .ok_or_else(|| "default phonetic profile is missing".to_string())?;
        profile.validate()?;
        for (layer, file) in &self.layers {
            file.validate(&layer.id, profile)?;
        }
        write_json_atomic(&root.join("kamafeu_voicebank.json"), &self.manifest)?;
        for profile in &self.profiles {
            let reference = self
                .manifest
                .phonetic_profiles
                .iter()
                .find(|reference| reference.id == profile.id)
                .ok_or_else(|| format!("profile is not referenced by manifest: {}", profile.id))?;
            write_json_atomic(&self.manifest.resolve(root, &reference.path)?, profile)?;
        }
        for (layer, file) in &self.layers {
            write_json_atomic(&self.manifest.resolve(root, &layer.units_path)?, file)?;
        }
        self.root = root.to_path_buf();
        Ok(())
    }

    /// Build a private draft package from the manifest's reachable file set.
    /// This method does not certify public distribution or license readiness.
    pub fn package_kfv2_draft(&mut self, destination: &Path) -> Result<(), String> {
        if !destination
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("kfv"))
        {
            return Err("KFV 2 draft packages must use the .kfv extension".into());
        }
        self.save_to_dir(&self.root.clone())?;
        let checked = NativeBank::load_dir(&self.root)?;
        let files = reachable_files(&checked)?;
        let parent = destination.parent().ok_or("package path has no parent")?;
        fs::create_dir_all(parent).map_err(|e| format!("cannot create package folder: {e}"))?;
        let temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|e| format!("cannot create temporary package: {e}"))?;
        let file = temporary
            .reopen()
            .map_err(|e| format!("cannot open temporary package: {e}"))?;
        let mut zip = zip::ZipWriter::new(file);
        for (relative, source) in files {
            let metadata = fs::symlink_metadata(&source)
                .map_err(|e| format!("cannot inspect {}: {e}", source.display()))?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(format!("package asset is not a regular file: {}", relative));
            }
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file(&relative, options)
                .map_err(|e| format!("cannot add {relative} to KFV: {e}"))?;
            let mut input = fs::File::open(&source)
                .map_err(|e| format!("cannot open {}: {e}", source.display()))?;
            std::io::copy(&mut input, &mut zip)
                .map_err(|e| format!("cannot read {}: {e}", source.display()))?;
        }
        zip.finish()
            .map_err(|e| format!("cannot finish KFV package: {e}"))?
            .sync_all()
            .map_err(|e| format!("cannot sync KFV package: {e}"))?;
        let verify_dir =
            tempfile::tempdir().map_err(|e| format!("cannot create package verifier: {e}"))?;
        crate::oto::voicebank::extract_kfv(temporary.path(), verify_dir.path())
            .map_err(|e| format!("KFV package verification failed: {e}"))?;
        NativeBank::load_dir(verify_dir.path())?;
        temporary
            .persist(destination)
            .map_err(|e| format!("cannot atomically publish draft package: {}", e.error))?;
        Ok(())
    }
}

pub(super) fn write_json_atomic(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let parent = path.parent().ok_or("JSON output path has no parent")?;
    fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|e| format!("cannot serialize JSON: {e}"))?;
    super::super::atomic_write(path, &bytes)
}

fn reachable_files(bank: &NativeBank) -> Result<Vec<(String, PathBuf)>, String> {
    let mut paths = BTreeSet::new();
    paths.insert("kamafeu_voicebank.json".to_string());
    for reference in &bank.manifest.phonetic_profiles {
        paths.insert(reference.path.clone());
    }
    for (layer, units) in &bank.layers {
        paths.insert(layer.units_path.clone());
        for asset in &units.assets {
            paths.insert(asset.path.clone());
        }
    }
    if let Some(license) = &bank.manifest.license {
        paths.insert(license.path.clone());
    }
    if let Some(presentation) = &bank.manifest.presentation {
        paths.extend(
            [presentation.avatar.clone(), presentation.portrait.clone()]
                .into_iter()
                .flatten(),
        );
    }
    paths
        .into_iter()
        .map(|relative| {
            let path = bank.manifest.resolve(&bank.root, &relative)?;
            Ok((relative, path))
        })
        .collect()
}
