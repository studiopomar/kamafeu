use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use web_time::Instant;

impl KamafeuStudioApp {
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

    pub(super) fn open_copaiba_for_alias(&mut self, requested_alias: &str, pitch: &str) {
        if requested_alias.trim().is_empty() || requested_alias.trim() == "+" {
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
        // all OTO timing fields are immediately editable. The external NEO
        // launcher remains available to callers that explicitly need it.
        self.copaiba_app
            .open_alias(voicebank.root_path.clone(), &alias);
        self.copaiba_window_open = true;
        self.transport_state.status_message =
            format!("Copaiba aberto no alias {alias}; ajuste os parâmetros e clique em Salvar.");
    }
}
