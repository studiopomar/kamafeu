mod engine;
mod note;
mod singer;

use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::theme::ThemeConfig;
use crate::gui::types::RightSidebarTab;
use crate::oto::Voicebank;
use crate::project::model::UNote;
pub use crate::renderer::RenderOptions as VocalModeParams;
use eframe::egui::{self, Color32, Frame, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;

pub fn draw_unified_panel(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    recent_voicebanks: &[PathBuf],
    singers_list: &[crate::oto::SingerInfo],
    singer_search_query: &mut String,
    singers_paths: &mut Vec<PathBuf>,
    vocal_mode_params: &mut VocalModeParams,
    selected_note_idx: Option<usize>,
    notes: &mut [UNote],
    selected_indices: &std::collections::HashSet<usize>,
    selected_ruler_alias: Option<&str>,
    active_tab: &mut RightSidebarTab,
    _phoneme_state: &mut PhonemePaletteState,
    render_threads: &mut u32,
    sample_rate: &mut u32,
    selected_resampler: &mut String,
    selected_wavtool: &mut String,
    custom_resampler_path: &mut Option<PathBuf>,
    custom_wavtool_path: &mut Option<PathBuf>,
    discord_rpc_enabled: &mut bool,
    on_load_vb: &mut dyn FnMut(Option<PathBuf>),
    on_add_singers_dir: &mut dyn FnMut(),
    on_reload_singers: &mut dyn FnMut(),
    on_open_gallery: &mut dyn FnMut(),
    _on_preview_phoneme: &mut dyn FnMut(&str),
    _on_insert_phoneme: &mut dyn FnMut(&str),
    _on_edit_phoneme: &mut dyn FnMut(&str),
    on_edit_selected_ruler_alias: &mut dyn FnMut(),
) {
    ui.vertical(|ui| {
        // --- 1. SEGMENTED TABS HEADER ---
        Frame::none()
            .fill(theme.elevated_surface_c32())
            .rounding(theme.ui_rounding())
            .stroke(theme.card_stroke())
            .inner_margin(egui::Margin::symmetric(4.0, 4.0))
            .show(ui, |ui| {
                ui.columns(3, |cols| {
                    let tabs = [
                        (RightSidebarTab::SingerTrack, lang.tr("Cantor", "Singer")),
                        (
                            RightSidebarTab::Phonemes,
                            lang.tr("Nota & Transição", "Note & Transition"),
                        ),
                        (RightSidebarTab::Engine, lang.tr("Motor", "Engine")),
                    ];

                    for (i, (tab, label)) in tabs.iter().enumerate() {
                        let is_selected = *active_tab == *tab;
                        let selection_progress = cols[i]
                            .ctx()
                            .animate_bool(egui::Id::new(("unified_inspector_tab", i)), is_selected);
                        let (bg, text_color, stroke) = if is_selected {
                            (
                                theme.c32_alpha(
                                    theme.accent_color,
                                    0.08 + 0.20 * selection_progress,
                                ),
                                theme.accent_c32(),
                                Stroke::new(
                                    0.7 + 0.5 * selection_progress,
                                    theme.accent_soft_c32(),
                                ),
                            )
                        } else {
                            (
                                theme.c32_alpha(theme.bg_canvas, 0.12),
                                theme.text_muted_c32(),
                                Stroke::NONE,
                            )
                        };

                        let btn = egui::Button::new(
                            RichText::new(*label).size(10.5).color(text_color).strong(),
                        )
                        .fill(bg)
                        .stroke(stroke)
                        .rounding(theme.ui_rounding())
                        .min_size(Vec2::new(cols[i].available_width(), 22.0));

                        let response = cols[i].add(btn);
                        if is_selected {
                            let underline = Rect::from_min_max(
                                Pos2::new(response.rect.left() + 8.0, response.rect.bottom() - 2.0),
                                Pos2::new(response.rect.right() - 8.0, response.rect.bottom()),
                            );
                            cols[i].painter().rect_filled(
                                underline,
                                Rounding::same(1.0),
                                theme.accent_c32(),
                            );
                        }
                        if response.clicked() {
                            *active_tab = *tab;
                        }
                    }
                });
            });

        ui.add_space(6.0);

        match active_tab {
            // ==========================================
            // ABA 1: CANTOR & PRESET VOCAL
            // ==========================================
            RightSidebarTab::SingerTrack => {
                singer::draw(
                    ui,
                    theme,
                    lang,
                    voicebank,
                    recent_voicebanks,
                    singers_list,
                    singer_search_query,
                    singers_paths,
                    vocal_mode_params,
                    on_load_vb,
                    on_add_singers_dir,
                    on_reload_singers,
                    on_open_gallery,
                );
            }

            // ==========================================
            // ABA 2: EDITOR DE NOTA, EXPRESSÃO E TRANSIÇÃO
            // ==========================================
            RightSidebarTab::Note | RightSidebarTab::Phonemes => {
                note::draw(
                    ui,
                    theme,
                    lang,
                    voicebank,
                    selected_note_idx,
                    notes,
                    selected_indices,
                    selected_ruler_alias,
                    selected_resampler,
                    selected_wavtool,
                    &vocal_mode_params.flags,
                    on_edit_selected_ruler_alias,
                );
            }

            // ==========================================
            // ABA 3: CONFIGURAÇÕES DE MOTOR & ÁUDIO
            // ==========================================
            RightSidebarTab::Engine => {
                engine::draw(
                    ui,
                    theme,
                    lang,
                    render_threads,
                    sample_rate,
                    selected_resampler,
                    selected_wavtool,
                    &mut vocal_mode_params.flags,
                    custom_resampler_path,
                    custom_wavtool_path,
                    discord_rpc_enabled,
                );
            }
        }
    });
}
