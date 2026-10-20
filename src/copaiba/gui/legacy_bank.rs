use super::*;

impl CopaibaToolkitApp {
    /// Opens a voicebank and selects the exact alias requested by the editor.
    /// This is used by the Kamafeu phoneme ruler so manual oto.ini fixes start on
    /// the offending sample instead of making the user search the bank again.
    pub fn open_alias(&mut self, path: PathBuf, alias: &str) {
        let oto_path = crate::copaiba_bridge::oto_path_for_alias(&path, alias).ok();
        let load_dir = oto_path
            .as_ref()
            .and_then(|oto| oto.parent().map(PathBuf::from))
            .unwrap_or(path);
        self.open_dir(load_dir);
        self.active_oto_path = oto_path;
        self.selected_source_alias = Some(alias.trim().to_string());
        self.preserve_oto_entry = self.active_oto_path.is_some();
        let wanted = alias.trim();
        if let Some(index) = self
            .config
            .entries
            .iter()
            .position(|entry| entry.alias.trim().eq_ignore_ascii_case(wanted))
        {
            self.selected_entry_index = Some(index);
            self.search_query.clear();
            self.filter_mode = ListFilterMode::ByAlias;
            self.load_selected_wav_samples();
            self.status_message = format!("Alias selecionado: {wanted}");
        } else {
            self.search_query = wanted.to_string();
            self.status_message = format!(
                "Alias '{wanted}' não foi encontrado; verifique o banco e crie ou ajuste a entrada."
            );
        }
    }

    pub fn open_dir(&mut self, path: PathBuf) {
        self.pending_classic_migration = None;
        self.migration_report = None;
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("kfv"))
        {
            match crate::copaiba::native_project::CopaibaNativeProject::load_path(&path) {
                Ok(project) => self.activate_native_project(project),
                Err(error) => {
                    self.status_message = format!("Erro ao abrir pacote KFV 2 no Copaíba: {error}")
                }
            }
            return;
        }
        let manifest_path = path.join("kamafeu_voicebank.json");
        if manifest_path.is_file() {
            match std::fs::read_to_string(&manifest_path)
                .map_err(|e| format!("cannot read KFV manifest: {e}"))
                .and_then(|text| {
                    serde_json::from_str::<serde_json::Value>(&text)
                        .map_err(|e| format!("invalid KFV manifest JSON: {e}"))
                }) {
                Ok(value)
                    if value.get("format").and_then(|v| v.as_str())
                        == Some("kamafeu.voicebank") =>
                {
                    match crate::copaiba::native_project::CopaibaNativeProject::load_dir(&path) {
                        Ok(project) => self.activate_native_project(project),
                        Err(error) => {
                            self.status_message = format!("Erro ao abrir KFV 2 no Copaíba: {error}")
                        }
                    }
                    return;
                }
                Err(error) => {
                    self.status_message = error;
                    return;
                }
                _ => {}
            }
        }
        match CopaibaConfig::load_from_dir(&path) {
            Ok(cfg) => {
                self.native_project = None;
                self.native_selected_layer = None;
                self.native_selected_unit = None;
                self.native_selected_asset = None;
                self.native_compare_asset = None;
                self.native_comparison_a = None;
                self.native_comparison_b = None;
                self.current_dir = Some(path);
                self.active_oto_path = None;
                self.selected_source_alias = None;
                self.preserve_oto_entry = false;
                self.config = cfg;
                self.avatar_texture = None;
                self.status_message.clear();
                if !self.config.entries.is_empty() {
                    self.selected_entry_index = Some(0);
                } else {
                    self.selected_entry_index = None;
                }
                self.load_selected_wav_samples();
            }
            Err(err) => {
                self.status_message = format!("Erro ao abrir voicebank: {err}");
            }
        }
    }

    pub fn selected_entry(&self) -> Option<&CopaibaEntry> {
        self.selected_entry_index
            .and_then(|idx| self.config.entries.get(idx))
    }

    pub fn duplicate_selected_entry(&mut self) {
        if let Some(idx) = self.selected_entry_index {
            if let Some(entry) = self.config.entries.get(idx).cloned() {
                let mut dup = entry;
                dup.alias = format!("{}_copia", dup.alias);
                let new_idx = idx + 1;
                self.config.entries.insert(new_idx, dup);
                self.selected_entry_index = Some(new_idx);
                self.load_selected_wav_samples();
            }
        }
    }

    pub fn delete_selected_entry(&mut self) {
        if let Some(idx) = self.selected_entry_index {
            if idx < self.config.entries.len() {
                self.config.entries.remove(idx);
                if self.config.entries.is_empty() {
                    self.selected_entry_index = None;
                } else if idx >= self.config.entries.len() {
                    self.selected_entry_index = Some(self.config.entries.len() - 1);
                }
                self.load_selected_wav_samples();
            }
        }
    }

    pub fn save_config(&mut self) -> Result<(), String> {
        if let Some(project) = self.native_project.as_mut() {
            let directory = self.current_dir.clone().ok_or("Nenhum diretório aberto")?;
            return project.save_to_dir(&directory);
        }
        if self.preserve_oto_entry {
            let oto_path = self
                .active_oto_path
                .as_ref()
                .ok_or_else(|| "nenhum oto.ini ativo".to_string())?;
            let index = self
                .selected_entry_index
                .ok_or_else(|| "nenhum alias selecionado".to_string())?;
            let entry = self
                .config
                .entries
                .get(index)
                .ok_or_else(|| "alias selecionado inválido".to_string())?;
            let oto_entry = crate::oto::OtoEntry::new(
                entry.wav_filename.clone(),
                entry.alias.clone(),
                entry.corte_inicial_ms,
                entry.consoante_ms,
                entry.corte_final_ms,
                entry.preutterance_ms,
                entry.overlap_ms,
            );
            let original_alias = self
                .selected_source_alias
                .as_deref()
                .unwrap_or(entry.alias.as_str());
            crate::copaiba_bridge::update_oto_entry(oto_path, original_alias, &oto_entry)?;
            self.selected_source_alias = Some(entry.alias.clone());
            self.status_message = format!("Entrada salva em {}", oto_path.display());
            return Ok(());
        }
        if let Some(ref dir) = self.current_dir {
            self.config.save_to_dir(dir)
        } else {
            Err("Nenhum diretório aberto".to_string())
        }
    }

    pub fn reload_current(&mut self) {
        if let Some(alias) = self.selected_source_alias.clone() {
            if let Some(oto_path) = self.active_oto_path.clone() {
                if let Some(dir) = oto_path.parent() {
                    self.open_alias(dir.to_path_buf(), &alias);
                    return;
                }
            }
        }
        if let Some(dir) = self.current_dir.clone() {
            self.open_dir(dir);
        }
    }

    pub fn compile_and_package_kfv(&self, dest_path: &std::path::Path) -> Result<(), String> {
        use std::fs;
        let src_dir = self.current_dir.as_ref().ok_or("Nenhum diretório aberto")?;

        self.config.save_to_dir(src_dir)?;

        let file = fs::File::create(dest_path)
            .map_err(|e| format!("Falha ao criar arquivo .kfv: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        fn walk_rec(
            src_dir: &std::path::Path,
            dir: &std::path::Path,
            zip: &mut zip::ZipWriter<fs::File>,
            options: zip::write::SimpleFileOptions,
        ) -> Result<(), String> {
            let read_dir =
                fs::read_dir(dir).map_err(|e| format!("Falha ao ler diretório: {}", e))?;
            for entry in read_dir {
                let entry = entry.map_err(|e| format!("Falha ao ler entrada do diretório: {e}"))?;
                let path = entry.path();
                let name = path
                    .strip_prefix(src_dir)
                    .map_err(|e| format!("Erro no strip_prefix: {}", e))?
                    .to_string_lossy()
                    .to_string();

                let file_type = entry
                    .file_type()
                    .map_err(|e| format!("Falha ao identificar tipo do arquivo: {e}"))?;
                if file_type.is_symlink() {
                    continue;
                } else if file_type.is_dir() {
                    zip.add_directory(&name, options)
                        .map_err(|e| format!("Falha ao adicionar pasta ao zip: {}", e))?;
                    walk_rec(src_dir, &path, zip, options)?;
                } else if file_type.is_file() {
                    if path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("kfv"))
                    {
                        continue;
                    }
                    zip.start_file(&name, options)
                        .map_err(|e| format!("Falha ao iniciar arquivo no zip: {}", e))?;
                    let mut f = fs::File::open(&path)
                        .map_err(|e| format!("Falha ao abrir arquivo: {}", e))?;
                    std::io::copy(&mut f, zip)
                        .map_err(|e| format!("Falha ao copiar dados para zip: {}", e))?;
                }
            }
            Ok(())
        }

        walk_rec(src_dir, src_dir, &mut zip, options)?;

        zip.finish()
            .map_err(|e| format!("Falha ao finalizar zip: {}", e))?;
        Ok(())
    }
}
