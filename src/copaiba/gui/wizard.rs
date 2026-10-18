use super::CopaibaToolkitApp;
use eframe::egui;

pub(super) fn draw_native_create_wizard(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    ui.add_space(12.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_max_width(540.0);
        ui.heading("Criar banco nativo · rascunho");
        ui.label("Começa com estilo Natural, camada MIDI 60 e um inventário PT-BR básico. Não inclui G2P nem gravações.");
        ui.horizontal(|ui| {
            ui.label("Nome");
            ui.text_edit_singleline(&mut app.native_draft_display_name);
        });
        ui.horizontal(|ui| {
            ui.label("Voice ID");
            ui.text_edit_singleline(&mut app.native_draft_voice_id);
        });
        ui.horizontal(|ui| {
            ui.label("Bank ID");
            ui.text_edit_singleline(&mut app.native_draft_bank_id);
        });
        ui.horizontal(|ui| {
            ui.label("Autor");
            ui.text_edit_singleline(&mut app.native_draft_publisher);
        });
        ui.small("O inventário é só uma lista de símbolos fonéticos. As pronúncias devem ser informadas diretamente; defina autorização e licença antes de distribuir.");
        ui.horizontal(|ui| {
            if ui.button("Escolher pasta e criar").clicked() {
                if let Some(parent) = crate::dialogs::FileDialog::new()
                    .set_title("Escolha onde criar o rascunho KFV 2")
                    .pick_folder()
                {
                    if let Err(error) = app.create_native_draft(&parent) {
                        app.status_message = format!("Não foi possível criar o banco: {error}");
                    }
                }
            }
            if ui.button("Cancelar").clicked() {
                app.native_create_wizard_open = false;
            }
        });
    });
}
