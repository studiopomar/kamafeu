use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use std::time::Instant;

impl KamafeuStudioApp {
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

        if let Ok(voicebank) = Voicebank::new(&root_path) {
            self.transport_state.voicebank_name = voicebank.name.clone();
            self.transport_state.voicebank_path = Some(voicebank.root_path.clone());
            self.voicebank = Some(voicebank);
            self.voicebank_oto_signature = Some(signature);
            self.piano_roll_state.phoneme_cache_hash = 0;
            self.piano_roll_state.phoneme_cache.clear();
            self.piano_roll_state.note_phonemes_cache.clear();
            self.transport_state.status_message = "oto.ini atualizado pelo Copaiba NEO".to_string();
        }
    }

    #[cfg(not(target_os = "android"))]
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
        let result = crate::copaiba_bridge::launch_editor(&voicebank.root_path, &alias);
        self.transport_state.status_message = match result {
            Ok(()) => format!("Copaiba NEO aberto em: {alias}"),
            Err(error) => error,
        };
    }
}
