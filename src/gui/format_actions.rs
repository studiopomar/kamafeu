use crate::formats::MidiFormat;
use crate::formats::SvpFormat;
use crate::formats::UfdataFormat;
use crate::formats::UstFormat;
use crate::formats::UstxFormat;
use crate::formats::VsqxFormat;
use crate::gui::KamafeuStudioApp;
use std::time::Instant;

impl KamafeuStudioApp {
    pub fn export_midi_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Arquivo MIDI")
            .set_file_name("export.mid")
            .add_filter("Arquivo MIDI (*.mid, *.midi)", &["mid", "midi"])
            .save_file()
        {
            if let Err(e) = MidiFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar MIDI: {}", e);
            } else {
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.transport_state.status_message = format!("MIDI exportado: {fname}");
                self.last_exported_notification = Some((
                    path.clone(),
                    format!("MIDI exportado: {fname}"),
                    Instant::now(),
                ));
            }
        }
    }

    pub fn export_ust_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Sequência UTAU")
            .set_file_name("export.ust")
            .add_filter("Sequência UTAU (*.ust)", &["ust"])
            .save_file()
        {
            if let Err(e) = UstFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar UST: {}", e);
            } else {
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.transport_state.status_message = format!("UST exportado: {fname}");
                self.last_exported_notification = Some((
                    path.clone(),
                    format!("UST exportado: {fname}"),
                    Instant::now(),
                ));
            }
        }
    }

    pub fn export_ustx_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Projeto OpenUTAU")
            .set_file_name("export.ustx")
            .add_filter("Projeto OpenUTAU (*.ustx)", &["ustx"])
            .save_file()
        {
            if let Err(e) = UstxFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar USTX: {}", e);
            } else {
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.transport_state.status_message = format!("USTX exportado: {fname}");
                self.last_exported_notification = Some((
                    path.clone(),
                    format!("USTX exportado: {fname}"),
                    Instant::now(),
                ));
            }
        }
    }

    pub fn export_ufdata_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar UtaFormatix Data")
            .set_file_name("export.ufdata")
            .add_filter("UtaFormatix Data (*.ufdata, *.json)", &["ufdata", "json"])
            .save_file()
        {
            if let Err(e) = UfdataFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar UFData: {}", e);
            } else {
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.transport_state.status_message = format!("UFData exportado: {fname}");
                self.last_exported_notification = Some((
                    path.clone(),
                    format!("UFData exportado: {fname}"),
                    Instant::now(),
                ));
            }
        }
    }

    pub fn export_svp_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Projeto Synthesizer V")
            .set_file_name("export.svp")
            .add_filter("Projeto Synthesizer V (*.svp)", &["svp"])
            .save_file()
        {
            if let Err(e) = SvpFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar SVP: {}", e);
            } else {
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.transport_state.status_message = format!("SVP exportado: {fname}");
                self.last_exported_notification = Some((
                    path.clone(),
                    format!("SVP exportado: {fname}"),
                    Instant::now(),
                ));
            }
        }
    }

    pub fn export_vsqx_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Sequência Vocaloid")
            .set_file_name("export.vsqx")
            .add_filter("Sequência Vocaloid (*.vsqx)", &["vsqx"])
            .save_file()
        {
            if let Err(e) = VsqxFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar VSQX: {}", e);
            } else {
                let fname = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.transport_state.status_message = format!("VSQX exportado: {fname}");
                self.last_exported_notification = Some((
                    path.clone(),
                    format!("VSQX exportado: {fname}"),
                    Instant::now(),
                ));
            }
        }
    }

    pub fn import_midi_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Arquivo MIDI")
            .add_filter("Arquivo MIDI (*.mid, *.midi)", &["mid", "midi"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_ust_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Sequência UTAU")
            .add_filter("Sequência UTAU (*.ust)", &["ust"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_ustx_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Projeto OpenUTAU")
            .add_filter("Projeto OpenUTAU (*.ustx)", &["ustx"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_ufdata_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar UtaFormatix Data")
            .add_filter("UtaFormatix Data (*.ufdata, *.json)", &["ufdata", "json"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_svp_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Projeto Synthesizer V")
            .add_filter("Projeto Synthesizer V (*.svp)", &["svp"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_vsqx_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Sequência Vocaloid")
            .add_filter("Sequência Vocaloid (*.vsqx, *.vsq)", &["vsqx", "vsq"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_audio_track_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Faixa de Áudio")
            .add_filter(
                "Áudio (*.wav, *.mp3, *.ogg, *.flac)",
                &["wav", "mp3", "ogg", "flac"],
            )
            .pick_file()
        {
            self.push_history();
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Audio Track")
                .to_string();
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Audio Track")
                .to_string();
            let file_path_str = path.to_string_lossy().to_string();

            let new_idx = self.project.tracks.len();
            self.project.tracks.push(crate::project::model::UTrack {
                name: file_stem,
                singer: "Instrumental / Áudio".to_string(),
                volume_db: 0.0,
                pan: 0.0,
                mute: false,
                solo: false,
                ..crate::project::model::UTrack::default()
            });
            let wave = crate::project::model::UWavePart::new(file_name, file_path_str, new_idx);
            self.project.wave_parts.push(wave);
            self.active_track_index = new_idx;
            self.transport_state.status_message =
                "Faixa de áudio adicionada com sucesso!".to_string();
        }
    }
}
