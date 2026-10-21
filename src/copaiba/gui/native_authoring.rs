use super::CopaibaToolkitApp;
use std::path::PathBuf;

impl CopaibaToolkitApp {
    pub fn create_kamuse_draft(&mut self, parent: &std::path::Path) -> Result<(), String> {
        let project = crate::copaiba::kamuse::KamuseProject::create(
            parent,
            &self.kamuse_id,
            &self.kamuse_name,
            &self.kamuse_language,
        )?;
        self.activate_kamuse_project(project);
        self.native_create_wizard_open = false;
        Ok(())
    }

    pub fn activate_kamuse_project(&mut self, project: crate::copaiba::kamuse::KamuseProject) {
        self.current_dir = Some(project.root.clone());
        self.active_oto_path = None;
        self.selected_source_alias = None;
        self.preserve_oto_entry = false;
        self.selected_entry_index = None;
        self.search_query.clear();
        self.loaded_waveform = None;
        self.loaded_wav_filename = None;
        self.avatar_texture = None;
        self.kamuse_waveform = None;
        self.kamuse_drag_mark = None;
        self.kamuse_marks_dirty = false;
        self.kamuse_editing_tone_id = None;
        self.kamuse_editing_recording = None;
        self.kamuse_pending_delete_tone = None;
        self.kamuse_pending_delete_recording = None;
        self.native_selected_layer = None;
        self.native_selected_unit = None;
        self.native_selected_asset = None;
        self.native_project = None;
        self.kamuse_selected_tone = project.cantor.tons.first().map(|tone| tone.id.clone());
        self.kamuse_selected_recording = project
            .cantor
            .tons
            .first()
            .and_then(|tone| tone.gravacoes.first())
            .map(|take| take.id.clone());
        self.kamuse_project = Some(project);
        self.config = crate::copaiba::CopaibaConfig::default();
        self.status_message.clear();
    }

    pub fn kamuse_import_recording(&mut self, source: PathBuf) -> Result<(), String> {
        let phones = self
            .kamuse_recording_phones
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let project = self
            .kamuse_project
            .as_mut()
            .ok_or("nenhum banco KAMUSE aberto")?;
        let recording_id =
            project.import_recording(&source, self.kamuse_recording_midi, &phones)?;
        self.kamuse_selected_tone = Some(format!("midi-{}", self.kamuse_recording_midi));
        self.kamuse_selected_recording = Some(recording_id);
        Ok(())
    }

    pub fn create_native_draft(&mut self, parent: &std::path::Path) -> Result<(), String> {
        let project = crate::copaiba::native_project::CopaibaNativeProject::create_new_draft(
            parent,
            &self.native_draft_voice_id,
            &self.native_draft_bank_id,
            &self.native_draft_display_name,
            &self.native_draft_publisher,
            crate::copaiba::native_project::CopaibaNativeProject::starter_pt_br_profile(),
        )?;
        let folder = project.root.clone();
        self.activate_native_project(project);
        self.native_create_wizard_open = false;
        self.status_message = format!(
            "Rascunho KFV 2 criado em {} · inventário fonético sem G2P; licenciamento e gravações ainda precisam ser definidos",
            folder.display()
        );
        Ok(())
    }

    pub fn import_native_recording(
        &mut self,
        source: PathBuf,
        asset_id: &str,
    ) -> Result<(), String> {
        let layer_id = self
            .native_selected_layer
            .clone()
            .ok_or("Nenhuma camada selecionada")?;
        let project = self
            .native_project
            .as_mut()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let path = project.import_recording(&layer_id, &source, asset_id)?;
        self.native_selected_asset = Some(asset_id.to_string());
        self.status_message = format!("Gravação copiada sem modificar o original: {path}");
        Ok(())
    }

    pub fn add_native_unit_from_selected_asset(&mut self) -> Result<(), String> {
        let layer_id = self
            .native_selected_layer
            .clone()
            .ok_or("Nenhuma camada selecionada")?;
        let asset_id = self
            .native_selected_asset
            .clone()
            .ok_or("Nenhuma gravação selecionada")?;
        let phones: Vec<String> = self
            .native_new_phones
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let project = self
            .native_project
            .as_mut()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        project.add_unit_from_asset(
            &layer_id,
            &asset_id,
            &self.native_new_unit_id,
            &self.native_new_kind,
            phones,
        )?;
        self.native_selected_unit = Some(self.native_new_unit_id.clone());
        self.load_native_selected_wav_samples();
        Ok(())
    }

    pub fn package_native_draft(&mut self, destination: &std::path::Path) -> Result<(), String> {
        self.save_config()?;
        self.native_project
            .as_mut()
            .ok_or("Nenhum banco KFV 2 aberto")?
            .package_kfv2_draft(destination)
    }
}
