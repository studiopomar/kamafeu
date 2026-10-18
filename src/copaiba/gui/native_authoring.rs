use super::CopaibaToolkitApp;
use std::path::PathBuf;

impl CopaibaToolkitApp {
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
