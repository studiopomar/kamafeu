use super::CopaibaToolkitApp;
use eframe::egui;

pub(super) fn draw_native_create_wizard(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    ui.add_space(12.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_max_width(560.0);
        ui.heading("Criar cantor nativo com KAMUSE");
        ui.label("O banco usa cantor.toml, mantém WAV/MP3 originais e seleciona Tainah WORLD como engine nativa.");
        ui.horizontal(|ui| { ui.label("Nome"); ui.text_edit_singleline(&mut app.kamuse_name); });
        ui.horizontal(|ui| { ui.label("ID"); ui.text_edit_singleline(&mut app.kamuse_id); });
        ui.horizontal(|ui| {
            ui.label("Perfil de idioma");
            egui::ComboBox::from_id_salt("kamuse_create_language").selected_text(&app.kamuse_language).show_ui(ui, |ui| {
                for language in ["pt-BR", "en-US", "ja-JP"] { ui.selectable_value(&mut app.kamuse_language, language.into(), language); }
            });
        });
        ui.small("A criação não inclui gravações autorizadas. Após criar, importe a voz e ajuste cada limite fonético no editor de marcações.");
        ui.horizontal(|ui| {
            if ui.button("Escolher pasta e criar banco").clicked() {
                if let Some(parent) = crate::dialogs::FileDialog::new().set_title("Escolha a pasta onde criar o cantor").pick_folder() {
                    if let Err(error) = app.create_kamuse_draft(&parent) { app.status_message = format!("Falha ao criar cantor: {error}"); }
                }
            }
            if ui.button("Cancelar").clicked() { app.native_create_wizard_open = false; }
        });
    });
}
