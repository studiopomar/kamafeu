use super::KamafeuStudioApp;
use crate::gui::theme::MelodyneTheme;
use crate::gui::types::EditTool;
use crate::oto::Voicebank;
use eframe::egui::{self, Frame, TopBottomPanel};
use std::path::PathBuf;

impl KamafeuStudioApp {
    pub(crate) fn render_menu_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("top_menu_bar")
            .exact_height(26.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("Arquivo", |ui| {
                        if ui.button("📄 Novo Projeto  (Ctrl+N / Cmd+N)").clicked() {
                            self.new_project();
                            ui.close_menu();
                        }
                        if ui.button("📂 Abrir Projeto...  (Ctrl+O / Cmd+O)").clicked() {
                            self.open_project_dialog();
                            ui.close_menu();
                        }

                        ui.menu_button("🗂 Projetos Recentes", |ui| {
                            if self.config.recent_projects.is_empty() {
                                ui.label(
                                    egui::RichText::new("Nenhum projeto recente")
                                        .size(11.0)
                                        .color(MelodyneTheme::TEXT_MUTED),
                                );
                            } else {
                                let mut to_open: Option<PathBuf> = None;
                                for p_path in &self.config.recent_projects {
                                    let label = p_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Projeto");
                                    if ui
                                        .button(egui::RichText::new(label).size(11.0))
                                        .on_hover_text(p_path.to_string_lossy())
                                        .clicked()
                                    {
                                        to_open = Some(p_path.clone());
                                        ui.close_menu();
                                    }
                                }
                                if let Some(p) = to_open {
                                    self.open_project_from_path(&p);
                                }
                            }
                        });

                        ui.separator();
                        if ui.button("💾 Salvar Projeto  (Ctrl+S / Cmd+S)").clicked() {
                            self.save_project();
                            ui.close_menu();
                        }
                        if ui.button("💾 Salvar Como...  (Ctrl+Shift+S)").clicked() {
                            self.save_project_as_dialog();
                            ui.close_menu();
                        }

                        ui.separator();
                        ui.menu_button("📥 Importar", |ui| {
                            if ui
                                .button("✨ Qualquer Formato Compatível (UtaFormatix)...")
                                .clicked()
                            {
                                self.open_project_dialog();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .button("🌐 UtaFormatix Data (.ufdata, .json)...")
                                .clicked()
                            {
                                self.import_ufdata_dialog();
                                ui.close_menu();
                            }
                            if ui.button("📦 Projeto OpenUTAU (.ustx)...").clicked() {
                                self.import_ustx_dialog();
                                ui.close_menu();
                            }
                            if ui.button("📜 Sequência UTAU (.ust)...").clicked() {
                                self.import_ust_dialog();
                                ui.close_menu();
                            }
                            if ui.button("🎹 Projeto Synthesizer V (.svp)...").clicked() {
                                self.import_svp_dialog();
                                ui.close_menu();
                            }
                            if ui
                                .button("🎤 Sequência Vocaloid (.vsqx, .vsq)...")
                                .clicked()
                            {
                                self.import_vsqx_dialog();
                                ui.close_menu();
                            }
                            if ui.button("🎵 Arquivo MIDI (.mid, .midi)...").clicked() {
                                self.import_midi_dialog();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .button(
                                    "🎧 Faixa de Áudio / Instrumental (.wav, .mp3, .ogg, .flac)...",
                                )
                                .clicked()
                            {
                                self.import_audio_track_dialog();
                                ui.close_menu();
                            }
                        });

                        ui.menu_button("📤 Exportar", |ui| {
                            if ui
                                .button("🔊 Exportar Áudio WAV... (Ctrl+E / Cmd+E)")
                                .clicked()
                            {
                                self.export_wav();
                                ui.close_menu();
                            }
                            if !self.project.wave_parts.is_empty() {
                                if ui
                                    .button("🎙️ Exportar Apenas Vocais / Acapella (.wav)...")
                                    .clicked()
                                {
                                    self.execute_export_wav(
                                        crate::gui::types::ExportAudioScope::VocalsOnly,
                                    );
                                    ui.close_menu();
                                }
                                if ui
                                    .button("🎵 Exportar Mix Completa com Áudios (.wav)...")
                                    .clicked()
                                {
                                    self.execute_export_wav(
                                        crate::gui::types::ExportAudioScope::VocalsAndAudio,
                                    );
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            if ui.button("🌐 UtaFormatix Data (*.ufdata)...").clicked() {
                                self.export_ufdata_dialog();
                                ui.close_menu();
                            }
                            if ui.button("📦 Projeto OpenUTAU (*.ustx)...").clicked() {
                                self.export_ustx_dialog();
                                ui.close_menu();
                            }
                            if ui.button("📜 Sequência UTAU (*.ust)...").clicked() {
                                self.export_ust_dialog();
                                ui.close_menu();
                            }
                            if ui.button("🎹 Projeto Synthesizer V (*.svp)...").clicked() {
                                self.export_svp_dialog();
                                ui.close_menu();
                            }
                            if ui.button("🎤 Sequência Vocaloid (*.vsqx)...").clicked() {
                                self.export_vsqx_dialog();
                                ui.close_menu();
                            }
                            if ui.button("🎵 Arquivo MIDI (*.mid)...").clicked() {
                                self.export_midi_dialog();
                                ui.close_menu();
                            }
                        });

                        ui.separator();
                        if ui.button("🚪 Fechar").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button("Editar", |ui| {
                        if ui.button("↩ Desfazer (Ctrl+Z / Cmd+Z)").clicked() {
                            self.pending_edit_snapshot = None;
                            if let Some(prev) = self.undo_manager.undo(self.project.clone()) {
                                self.project = prev;
                            }
                            ui.close_menu();
                        }
                        if ui.button("↪ Refazer (Ctrl+Y / Cmd+Shift+Z)").clicked() {
                            self.pending_edit_snapshot = None;
                            if let Some(next) = self.undo_manager.redo(self.project.clone()) {
                                self.project = next;
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("✂ Recortar (Ctrl+X / Cmd+X)").clicked() {
                            self.cut_selected_notes();
                            ui.close_menu();
                        }
                        if ui.button("📋 Copiar   (Ctrl+C / Cmd+C)").clicked() {
                            self.copy_selected_notes();
                            ui.close_menu();
                        }
                        if ui.button("📥 Colar    (Ctrl+V / Cmd+V)").clicked() {
                            self.paste_notes();
                            ui.close_menu();
                        }
                        if ui.button("📑 Duplicar Nota (Ctrl+D / Cmd+D)").clicked() {
                            if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                                let notes = self.current_notes();
                                if sel_idx < notes.len() {
                                    let mut dup = notes[sel_idx].clone();
                                    dup.position_ms += dup.duration_ms;
                                    self.push_history();
                                    self.current_notes_mut().push(dup);
                                    self.piano_roll_state.selected_note_index =
                                        Some(self.current_notes().len() - 1);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("🗑 Excluir  (Delete / Backspace)").clicked() {
                            self.delete_selected_notes();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("🔲 Selecionar Tudo (Ctrl+A / Cmd+A)").clicked() {
                            let note_count = self.current_notes().len();
                            self.piano_roll_state.selected_note_indices = (0..note_count).collect();
                            if note_count > 0 {
                                self.piano_roll_state.selected_note_index = Some(0);
                            }
                            ui.close_menu();
                        }
                        if ui.button("🔳 Desmarcar Seleção (Ctrl+Shift+A)").clicked() {
                            self.piano_roll_state.selected_note_indices.clear();
                            self.piano_roll_state.selected_note_index = None;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button("✍️ Inserir Letras em Lote (Ctrl+Shift+L)")
                            .clicked()
                        {
                            self.batch_lyrics_open = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("🎵 Transposição", |ui| {
                            if ui.button("Transpor +1 Semitom  (Seta Cima)").clicked() {
                                self.transpose_selected_note(1);
                                ui.close_menu();
                            }
                            if ui.button("Transpor -1 Semitom  (Seta Baixo)").clicked() {
                                self.transpose_selected_note(-1);
                                ui.close_menu();
                            }
                            if ui
                                .button("Transpor +1 Oitava   (Shift + Seta Cima)")
                                .clicked()
                            {
                                self.transpose_selected_note(12);
                                ui.close_menu();
                            }
                            if ui
                                .button("Transpor -1 Oitava   (Shift + Seta Baixo)")
                                .clicked()
                            {
                                self.transpose_selected_note(-12);
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                        if ui
                            .button("✨ AutoPitch Vocal Studio...  (Ctrl+Alt+P)")
                            .clicked()
                        {
                            self.autopitch_window_open = true;
                            ui.close_menu();
                        }
                        if ui
                            .button("🌸 Aplicar AutoPitch Suave/Pop em Tudo")
                            .clicked()
                        {
                            self.apply_autopitch_all();
                            ui.close_menu();
                        }
                        if ui
                            .button("🧹 Limpar Curvas de Pitch de Todas as Notas")
                            .clicked()
                        {
                            self.clear_all_pitch_curves();
                            ui.close_menu();
                        }
                        if ui
                            .button("🧹 Resetar Envelopes de Volume para Padrão")
                            .clicked()
                        {
                            self.reset_all_volume_envelopes();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Faixas", |ui| {
                        if ui.button("➕ Nova Faixa (Track)").clicked() {
                            let new_idx = self.project.tracks.len();
                            let track_name = format!("Track {}", new_idx + 1);
                            self.project.tracks.push(crate::project::model::UTrack {
                                name: track_name.clone(),
                                singer: "Cantor Padrão".to_string(),
                                volume_db: 0.0,
                                pan: 0.0,
                                mute: false,
                                solo: false,
                                ..crate::project::model::UTrack::default()
                            });
                            self.project
                                .parts
                                .push(crate::project::model::UVoicePart::new(
                                    format!("Parte {}", new_idx + 1),
                                    new_idx,
                                ));
                            self.active_track_index = new_idx;
                            ui.close_menu();
                        }
                        if self.project.tracks.len() > 1
                            && ui.button("🗑️ Excluir Faixa Ativa").clicked()
                        {
                            let del_idx = self.active_track_index;
                            if del_idx < self.project.tracks.len() {
                                self.project.tracks.remove(del_idx);
                                self.project.parts.retain(|p| p.track_index != del_idx);
                                for p in self.project.parts.iter_mut() {
                                    if p.track_index > del_idx {
                                        p.track_index -= 1;
                                    }
                                }
                                if self.active_track_index >= self.project.tracks.len() {
                                    self.active_track_index =
                                        self.project.tracks.len().saturating_sub(1);
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
                            if ui.checkbox(&mut track.mute, "Mudo (Mute) [M]").clicked() {
                                ui.close_menu();
                            }
                            if ui.checkbox(&mut track.solo, "Solo").clicked() {
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("🔤 Refonetizar Toda a Faixa Ativa").clicked() {
                            self.rephonemize_all_notes();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Modos Vocais", |ui| {
                        let mode = &mut self.vocal_mode_params.phonemizer_mode;
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::None,
                                "🚫 Sem Fonemizador (Manual)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("[JA] Japonês")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 215, 0)),
                        );
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::BasicCV,
                                "  JA: Basic CV (Hiragana)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::VCV,
                                "  JA: Japanese VCV (- あ, a か)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::CVVC,
                                "  JA: Japanese CVVC",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("[EN] Inglês")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 215, 0)),
                        );
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::EnglishG2P,
                                "  ✨ EN: English G2P (Palavras -> Fonemas)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::EnglishArpasing,
                                "  🔤 EN: English Arpasing (Fonética Direta)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::EnglishVCCV,
                                "  🔤 EN: English VCCV (Fonética Direta)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("[PT] Português")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 215, 0)),
                        );
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::PortugueseG2P,
                                "  ✨ PT: Português G2P (Palavras -> Fonemas)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV,
                                "  🔥 PT: VCCV BRAPA (xiao / PT-BR 3.7)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC,
                                "  🔤 PT: BRAPA CVC (Fonética Direta)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::PortugueseCVVC,
                                "  🔤 PT: Portuguese CVVC (Fonética Direta)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(
                                mode,
                                crate::phonemizer::PhonemizerMode::PortugueseVCV,
                                "  🔤 PT: Portuguese VCV (Fonética Direta)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("🔤 Forçar Atualização de Fonemas").clicked() {
                            self.rephonemize_all_notes();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Cantores", |ui| {
                        if ui.button("🎭 Galeria de Cantores...").clicked() {
                            self.singers_gallery_window_open = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("📁 Carregar Voicebank Único...").clicked() {
                            #[cfg(not(target_os = "android"))]
                            if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                                if let Ok(vb) = Voicebank::new(&folder) {
                                    self.transport_state.status_message =
                                        format!("Voicebank Carregado: {}", vb.name);
                                    self.transport_state.voicebank_name = vb.name.clone();
                                    self.transport_state.voicebank_path =
                                        Some(vb.root_path.clone());
                                    self.config.add_recent_voicebank(vb.root_path.clone());
                                    self.voicebank = Some(vb);
                                }
                            } else {
                                self.folder_picker_open = true;
                            }
                            #[cfg(target_os = "android")]
                            {
                                self.folder_picker_open = true;
                            }
                            ui.close_menu();
                        }
                        if ui
                            .button("➕ Registrar Pasta do OpenUtau / Singers...")
                            .clicked()
                        {
                            #[cfg(not(target_os = "android"))]
                            if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                                if !self.config.singers_paths.contains(&folder) {
                                    self.config.singers_paths.push(folder);
                                    self.persist_config();
                                    self.reload_singers();
                                }
                            } else {
                                self.folder_picker_open = true;
                            }
                            #[cfg(target_os = "android")]
                            {
                                self.folder_picker_open = true;
                            }
                            ui.close_menu();
                        }
                        if ui.button("🔄 Recarregar Cantores").clicked() {
                            self.reload_singers();
                            ui.close_menu();
                        }

                        ui.menu_button("🎤 Voicebanks Recentes", |ui| {
                            if self.config.recent_voicebanks.is_empty() {
                                ui.label(
                                    egui::RichText::new("Nenhum voicebank recente")
                                        .size(11.0)
                                        .color(MelodyneTheme::TEXT_MUTED),
                                );
                            } else {
                                let mut to_load: Option<PathBuf> = None;
                                for vb_path in &self.config.recent_voicebanks {
                                    let label = vb_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Voicebank");
                                    if ui
                                        .button(egui::RichText::new(label).size(11.0))
                                        .on_hover_text(vb_path.to_string_lossy())
                                        .clicked()
                                    {
                                        to_load = Some(vb_path.clone());
                                        ui.close_menu();
                                    }
                                }
                                if let Some(vb_dir) = to_load {
                                    if let Ok(vb) = Voicebank::new(&vb_dir) {
                                        self.transport_state.status_message =
                                            format!("Voicebank Carregado: {}", vb.name);
                                        self.transport_state.voicebank_name = vb.name.clone();
                                        self.transport_state.voicebank_path =
                                            Some(vb.root_path.clone());
                                        self.config.add_recent_voicebank(vb.root_path.clone());
                                        self.voicebank = Some(vb);
                                    }
                                }
                            }
                        });

                        ui.separator();
                        if ui.button("🛠 Copaiba Voicebank Toolkit").clicked() {
                            self.copaiba_window_open = true;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Ferramentas", |ui| {
                        if ui.button("👆 Ponteiro (Seleção) [V / 1]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::Pointer;
                            ui.close_menu();
                        }
                        if ui.button("✏ Lápis (Desenhar)   [N / 2]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::Pencil;
                            ui.close_menu();
                        }
                        if ui.button("📈 Desenhar Pitch     [P / 3]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::PitchDraw;
                            ui.close_menu();
                        }
                        if self.piano_roll_state.active_tool == EditTool::PitchDraw {
                            ui.separator();
                            if ui
                                .radio_value(
                                    &mut self.piano_roll_state.pitch_sub_tool,
                                    crate::gui::types::PitchSubTool::Freehand,
                                    "  ✏ Pitch Livre (Suave)",
                                )
                                .clicked()
                            {
                                ui.close_menu();
                            }
                            if ui
                                .radio_value(
                                    &mut self.piano_roll_state.pitch_sub_tool,
                                    crate::gui::types::PitchSubTool::Line,
                                    "  📏 Reta / Glissando",
                                )
                                .clicked()
                            {
                                ui.close_menu();
                            }
                            if ui
                                .radio_value(
                                    &mut self.piano_roll_state.pitch_sub_tool,
                                    crate::gui::types::PitchSubTool::Vibrato,
                                    "  〰 Pincel de Vibrato",
                                )
                                .clicked()
                            {
                                ui.close_menu();
                            }
                            if ui
                                .radio_value(
                                    &mut self.piano_roll_state.pitch_sub_tool,
                                    crate::gui::types::PitchSubTool::Smooth,
                                    "  🪄 Pincel Suavizador",
                                )
                                .clicked()
                            {
                                ui.close_menu();
                            }
                            ui.separator();
                        }
                        if ui.button("🧹 Borracha           [E / 4]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::Eraser;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("✨ AutoPitch (Afinador Orgânico)...").clicked() {
                            self.autopitch_window_open = true;
                            ui.close_menu();
                        }
                        if ui.button("🎨 Paleta de Fonemas").clicked() {
                            self.right_sidebar_tab = crate::gui::types::RightSidebarTab::Phonemes;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Exibir", |ui| {
                        if ui.button("🔍 Aumentar Zoom X  (Ctrl+= / Cmd+=)").clicked() {
                            self.piano_roll_state.px_per_ms =
                                (self.piano_roll_state.px_per_ms * 1.25).min(1.5);
                            ui.close_menu();
                        }
                        if ui.button("🔍 Diminuir Zoom X  (Ctrl+- / Cmd+-)").clicked() {
                            self.piano_roll_state.px_per_ms =
                                (self.piano_roll_state.px_per_ms * 0.8).max(0.05);
                            ui.close_menu();
                        }
                        if ui
                            .button("🔎 Redefinir Zoom Padrão (Ctrl+0 / Cmd+0)")
                            .clicked()
                        {
                            self.piano_roll_state.px_per_ms = 0.25;
                            self.piano_roll_state.row_height = 22.0;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("🖥 Escala de Interface (DPI / Zoom)", |ui| {
                            let scales = [
                                ("75% (Muito Compacto)", 0.75f32),
                                ("85% (Compacto)", 0.85f32),
                                ("90% (Espaçoso)", 0.90f32),
                                ("100% (Padrão)", 1.00f32),
                                ("110% (Ampliado)", 1.10f32),
                                ("125% (Grande / HiDPI)", 1.25f32),
                                ("150% (Muito Grande)", 1.50f32),
                            ];
                            for (label, s) in scales {
                                let is_active = (self.config.ui_scale_factor - s).abs() < 0.02;
                                if ui.selectable_label(is_active, label).clicked() {
                                    self.config.ui_scale_factor = s;
                                    self.persist_config();
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                        if ui
                            .checkbox(
                                &mut self.piano_roll_state.show_arrangement_view,
                                "🎼 Exibir Painel de Multifaixas / Arrangement",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(
                                &mut self.piano_roll_state.show_waveform,
                                "🌊 Exibir Trilha de Waveform",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(
                                &mut self.piano_roll_state.show_parameters_drawer,
                                "🎛 Painel de Parâmetros e Expressões (Tab)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(
                                &mut self.render_log_window_open,
                                "⚡ Janela de Log do Engine DSP (Ctrl+L)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("📜 Auto-Rolagem", |ui| {
                            let mode = &mut self.piano_roll_state.auto_scroll_mode;
                            let is_off = *mode == crate::gui::types::AutoScrollMode::Off;
                            let is_stationary =
                                *mode == crate::gui::types::AutoScrollMode::StationaryCursor;
                            let is_page = *mode == crate::gui::types::AutoScrollMode::PageScroll;

                            if ui
                                .button(if is_off {
                                    "✓  Desligar"
                                } else {
                                    "    Desligar"
                                })
                                .clicked()
                            {
                                *mode = crate::gui::types::AutoScrollMode::Off;
                                ui.close_menu();
                            }
                            if ui
                                .button(if is_stationary {
                                    "✓  Cursor Estacionário"
                                } else {
                                    "    Cursor Estacionário"
                                })
                                .clicked()
                            {
                                *mode = crate::gui::types::AutoScrollMode::StationaryCursor;
                                ui.close_menu();
                            }
                            if ui
                                .button(if is_page {
                                    "✓  Rolagem de Página"
                                } else {
                                    "    Rolagem de Página"
                                })
                                .clicked()
                            {
                                *mode = crate::gui::types::AutoScrollMode::PageScroll;
                                ui.close_menu();
                            }
                        });
                    });

                    ui.menu_button("Reprodução", |ui| {
                        let is_playing = self.audio_player.is_playing() || self.render_rx.is_some();
                        if ui
                            .button(if is_playing {
                                "⏸  Pausar Reprodução (Espaço)"
                            } else {
                                "▶  Tocar / Iniciar (Espaço)"
                            })
                            .clicked()
                        {
                            if is_playing {
                                self.pause_audio();
                            } else {
                                self.play_current_track();
                            }
                            ui.close_menu();
                        }
                        if ui.button("⏹  Parar e Ir para o Início (Esc)").clicked() {
                            self.stop_audio();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("⏪ Rebobinar para o Início (0ms)").clicked() {
                            self.piano_roll_state.playhead_ms = 0.0;
                            ui.close_menu();
                        }
                        if ui.button("⏩ Ir para o Final da Música").clicked() {
                            let max_end = self
                                .project
                                .parts
                                .iter()
                                .flat_map(|p| p.notes.iter())
                                .map(|n| n.position_ms + n.duration_ms)
                                .fold(0.0f64, f64::max);
                            self.piano_roll_state.playhead_ms = max_end;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("⚡ Forçar Pré-renderização do Áudio").clicked() {
                            self.play_current_track();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Ajuda", |ui| {
                        if ui
                            .button("⌨ Guia de Teclas de Atalho... (F1 / Cmd+?)")
                            .clicked()
                        {
                            self.shortcuts_guide_open = true;
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(
                                &mut self.config.discord_rpc_enabled,
                                "💬 Discord Rich Presence",
                            )
                            .clicked()
                        {
                            self.persist_config();
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Kamafeu Studio v0.0.4-beta (público)")
                                .strong()
                                .size(11.5)
                                .color(egui::Color32::from_rgb(0, 255, 157)),
                        );
                        ui.label(
                            egui::RichText::new("Motor Vocal OpenUTAU / UTAU em Rust")
                                .size(9.5)
                                .color(MelodyneTheme::TEXT_MUTED),
                        );
                    });

                    let project_display_name = if let Some(ref path) = self.current_project_path {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("project.aps")
                            .to_string()
                    } else {
                        let stem = self.project.name.trim();
                        if stem.is_empty() {
                            "Novo Projeto".to_string()
                        } else {
                            stem.to_string()
                        }
                    };

                    let title_text = if self.is_dirty {
                        format!("* {} (Não salvo)", project_display_name)
                    } else {
                        project_display_name.clone()
                    };

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        let (badge_bg, badge_border, badge_fg) = if self.is_dirty {
                            (
                                egui::Color32::from_rgb(48, 28, 16),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 170, 50)),
                                egui::Color32::from_rgb(255, 200, 100),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(20, 26, 36),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 60, 85)),
                                egui::Color32::from_rgb(170, 190, 220),
                            )
                        };

                        egui::Frame::none()
                            .fill(badge_bg)
                            .stroke(badge_border)
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(7.0, 2.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("📁 {}", title_text))
                                        .size(10.5)
                                        .strong()
                                        .color(badge_fg),
                                );
                            });
                    });
                });
            });
    }
}
