use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use web_time::Instant;

impl KamafeuStudioApp {
    pub(crate) fn open_kamuse_for_current_context(&mut self) {
        if let Some((alias, pitch)) = self.piano_roll_state.selected_copaiba_alias.clone() {
            // Keep the KAMUSE command and the phoneme context menu on the same
            // bank-loading path, so both open the selected alias or native bank.
            self.open_voicebank_editor_for_alias(&alias, &pitch);
            return;
        }

        let native_bank_active = self
            .project
            .tracks
            .get(self.active_track_index)
            .and_then(|track| track.native_voicebank.as_ref())
            .and_then(|reference| reference.source_path_hint.as_ref())
            .is_some();
        if native_bank_active {
            self.open_native_bank_settings();
        } else {
            self.transport_state.status_message =
                "Selecione um fonema no editor ou vincule um banco nativo para abrir o KAMUSE no contexto correto.".into();
        }
    }

    pub(crate) fn refresh_active_native_bank_reference(&mut self) {
        let Some(path) = self
            .project
            .tracks
            .get(self.active_track_index)
            .and_then(|track| track.native_voicebank.as_ref())
            .and_then(|reference| reference.source_path_hint.as_deref())
            .map(std::path::PathBuf::from)
        else {
            return;
        };
        let refreshed = crate::voicebank::cantor::CantorBank::load_path(&path)
            .and_then(|bank| bank.project_reference(Some(path.display().to_string())));
        match refreshed {
            Ok(reference) => {
                if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
                    track.native_voicebank = Some(reference);
                }
                self.project.render_state = Some(Default::default());
                self.piano_roll_state.phoneme_cache_hash = 0;
                self.piano_roll_state.phoneme_cache.clear();
                self.piano_roll_state.note_phonemes_cache.clear();
                self.piano_roll_state.selected_copaiba_alias = None;
                self.is_dirty = true;
            }
            Err(error) => {
                self.transport_state.status_message = format!(
                    "O banco foi fechado, mas a referência do projeto não foi atualizada: {error}"
                );
            }
        }
    }

    pub(crate) fn open_native_bank_settings(&mut self) {
        let path = self
            .project
            .tracks
            .get(self.active_track_index)
            .and_then(|track| track.native_voicebank.as_ref())
            .and_then(|reference| reference.source_path_hint.as_deref())
            .map(std::path::PathBuf::from);
        let Some(path) = path else {
            self.transport_state.status_message =
                "A faixa ativa não tem um banco Kamafeu nativo com caminho de origem.".into();
            return;
        };
        match crate::copaiba::kamuse::KamuseProject::load(&path) {
            Ok(project) => {
                let name = project.cantor.nome.clone();
                let source = project
                    .cantor
                    .tons
                    .first()
                    .and_then(|tone| tone.gravacoes.first())
                    .map(|recording| recording.arquivo.clone())
                    .unwrap_or_else(|| "sem gravação".into());
                self.copaiba_app.activate_kamuse_project(project);
                self.copaiba_window_open = true;
                self.transport_state.status_message =
                    format!("KAMUSE abriu {name} · fonte selecionada: {source}");
            }
            Err(error) => {
                self.transport_state.status_message =
                    format!("Não foi possível abrir a configuração do banco nativo: {error}");
            }
        }
    }

    /// Bind a `cantor.toml` bank to the active APS track. Native banks do not
    /// have an `oto.ini`, so they must never go through `Voicebank::new`.
    pub(crate) fn bind_native_singer_to_active_track(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let bank = crate::voicebank::cantor::CantorBank::load_path(path)?;
        let snapshot = bank.native_snapshot()?;
        let reference = bank.project_reference(Some(path.display().to_string()))?;
        let coverage =
            snapshot.coverage_summary(&reference.phonetic_profile_id, &reference.style_id)?;
        let track_index = self.active_track_index;
        let track = self
            .project
            .tracks
            .get_mut(track_index)
            .ok_or_else(|| "Não há faixa ativa para vincular o banco nativo".to_string())?;
        track.native_voicebank = Some(reference);
        track.voicebank_path = Some(path.display().to_string());
        track.singer = bank.cantor.nome.clone();
        // Native Kamafeu banks currently use direct space-separated phones.
        track.phonemizer = Some(crate::phonemizer::PhonemizerMode::None);
        let track_name = track.name.clone();
        self.project.voicebank = Some(bank.cantor.nome.clone());
        self.project.voicebank_path = Some(path.display().to_string());
        self.project.render_state = Some(Default::default());
        self.vocal_mode_params.phonemizer_mode = crate::phonemizer::PhonemizerMode::None;
        self.transport_state.voicebank_name = bank.cantor.nome.clone();
        self.transport_state.voicebank_path = Some(path.to_path_buf());
        self.transport_state.status_message = format!(
            "{} vinculada à faixa {} · {} unidades disponíveis · Tainah WORLD · fonemas diretos (ex.: k a)",
            bank.cantor.nome, track_name, coverage.unit_count
        );
        self.is_dirty = true;
        self.piano_roll_state.phoneme_cache_hash = 0;
        self.piano_roll_state.phoneme_cache.clear();
        self.piano_roll_state.note_phonemes_cache.clear();
        self.piano_roll_state.selected_copaiba_alias = None;
        Ok(())
    }

    /// Makes every voicebank entry point restore the singer-specific render
    /// profile (including phonemizer), invalidate aliases, and persist the
    /// selection consistently.
    pub(crate) fn activate_voicebank(&mut self, voicebank: Voicebank) {
        let lang = self.config.language;
        self.transport_state.status_message = if lang.is_en() {
            format!("Voicebank Loaded: {}", voicebank.name)
        } else {
            format!("Voicebank Carregado: {}", voicebank.name)
        };
        self.transport_state.voicebank_name = voicebank.name.clone();
        self.transport_state.voicebank_path = Some(voicebank.root_path.clone());
        self.config
            .add_recent_voicebank(voicebank.root_path.clone());
        self.voicebank = Some(voicebank);
        self.restore_render_profile_for_active_context();
        self.piano_roll_state.phoneme_cache_hash = 0;
        self.piano_roll_state.phoneme_cache.clear();
        self.piano_roll_state.note_phonemes_cache.clear();
        self.persist_config();
    }

    pub fn reload_singers(&mut self) {
        self.singers_list = crate::oto::SingerScanner::scan_directories(&self.config.singers_paths);
    }

    pub(super) fn refresh_voicebank_oto(&mut self) {
        if self.last_voicebank_oto_check.elapsed() < std::time::Duration::from_millis(350) {
            return;
        }
        self.last_voicebank_oto_check = Instant::now();

        let Some(root_path) = self
            .voicebank
            .as_ref()
            .map(|voicebank| voicebank.root_path.clone())
        else {
            return;
        };
        let Ok(signature) = crate::copaiba_bridge::oto_signature(&root_path) else {
            return;
        };
        if self.voicebank_oto_signature.is_none() {
            self.voicebank_oto_signature = Some(signature);
            return;
        }
        if self.voicebank_oto_signature == Some(signature) {
            return;
        }

        if let Ok(mut voicebank) = Voicebank::new(&root_path) {
            if let Some(previous) = self.voicebank.as_ref() {
                voicebank
                    .prefix_map
                    .select_color(previous.prefix_map.selected_color());
            }
            self.transport_state.voicebank_name = voicebank.name.clone();
            self.transport_state.voicebank_path = Some(voicebank.root_path.clone());
            self.voicebank = Some(voicebank);
            self.voicebank_oto_signature = Some(signature);
            self.piano_roll_state.phoneme_cache_hash = 0;
            self.piano_roll_state.phoneme_cache.clear();
            self.piano_roll_state.note_phonemes_cache.clear();
            self.transport_state.status_message = "oto.ini atualizado pelo Copaiba".to_string();
        }
    }

    pub(super) fn open_voicebank_editor_for_alias(&mut self, requested_alias: &str, pitch: &str) {
        if requested_alias.trim().is_empty() || requested_alias.trim() == "+" {
            return;
        }
        if self
            .project
            .tracks
            .get(self.active_track_index)
            .is_some_and(|track| track.native_voicebank.is_some())
        {
            self.open_native_bank_settings();
            return;
        }
        let Some(voicebank) = self.voicebank.as_ref() else {
            self.transport_state.status_message =
                "Carregue um voicebank para editar o oto.ini.".to_string();
            return;
        };
        let alias = voicebank
            .entries
            .keys()
            .find(|alias| alias.trim().eq_ignore_ascii_case(requested_alias.trim()))
            .cloned()
            .or_else(|| {
                voicebank
                    .find_entry(requested_alias, pitch)
                    .map(|entry| entry.alias.clone())
            })
            .unwrap_or_else(|| requested_alias.trim().to_string());
        // Use the embedded Copaiba editor so the selected phoneme, its WAV and
        // all oto.ini timing fields are immediately editable. The external NEO
        // launcher remains available to callers that explicitly need it.
        self.copaiba_app
            .open_alias(voicebank.root_path.clone(), &alias);
        self.copaiba_window_open = true;
        self.transport_state.status_message = format!(
            "Copaiba aberto no alias UTAU {alias}; ajuste os parâmetros e clique em Salvar."
        );
    }
}
