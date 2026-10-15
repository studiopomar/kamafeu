mod engine;
mod note;
mod phonemes;
mod singer;

use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::theme::ThemeConfig;
use crate::gui::types::{InspectorSection, RightSidebarTab};
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
    phoneme_state: &mut PhonemePaletteState,
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
    on_preview_phoneme: &mut dyn FnMut(&str),
    on_insert_phoneme: &mut dyn FnMut(&str),
    on_edit_phoneme: &mut dyn FnMut(&str),
    on_edit_selected_ruler_alias: &mut dyn FnMut(),
) {
    let compact = cfg!(target_os = "android") || ui.ctx().screen_rect().width() < 1100.0;
    let has_selection = selected_note_idx.is_some();
    let multiple_selection = selected_indices.len() > 1;

    ui.vertical(|ui| {
        draw_context_header(
            ui,
            theme,
            lang,
            voicebank,
            selected_note_idx,
            selected_indices,
            selected_ruler_alias,
            compact,
        );
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .id_salt("unified_inspector_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let singer_default = *active_tab == RightSidebarTab::SingerTrack;
                let note_default = *active_tab == RightSidebarTab::Note;
                let phoneme_default = *active_tab == RightSidebarTab::Phonemes;
                let expressions_default = *active_tab == RightSidebarTab::Expressions;
                let envelope_default = *active_tab == RightSidebarTab::Envelope;
                let engine_default = *active_tab == RightSidebarTab::Engine;

                draw_section(
                    ui,
                    theme,
                    lang,
                    InspectorSection::Singer,
                    lang.tr("Cantor e faixa", "Singer & track"),
                    lang.tr(
                        "Voicebank, autor e cantor ativo",
                        "Voicebank, author and active singer",
                    ),
                    singer_default,
                    singer_badge(voicebank, lang),
                    active_tab,
                    RightSidebarTab::SingerTrack,
                    |ui| {
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
                    },
                );

                ui.add_space(6.0);
                draw_section(
                    ui,
                    theme,
                    lang,
                    InspectorSection::Note,
                    if multiple_selection {
                        lang.tr("Notas selecionadas", "Selected notes")
                    } else {
                        lang.tr("Nota", "Note")
                    },
                    lang.tr(
                        "Pitch, duração, dinâmica e articulação",
                        "Pitch, duration, dynamics and articulation",
                    ),
                    note_default,
                    if multiple_selection {
                        Some(format!("{}", selected_indices.len()))
                    } else if has_selection {
                        Some(lang.tr("ativa", "active").to_string())
                    } else {
                        None
                    },
                    active_tab,
                    RightSidebarTab::Note,
                    |ui| {
                        draw_note_page(
                            ui,
                            note::NoteInspectorSection::Note,
                            theme,
                            lang,
                            voicebank,
                            vocal_mode_params.phonemizer_mode,
                            selected_note_idx,
                            notes,
                            selected_indices,
                            selected_ruler_alias,
                            selected_resampler,
                            selected_wavtool,
                            &vocal_mode_params.flags,
                            on_edit_selected_ruler_alias,
                        );
                    },
                );

                ui.add_space(6.0);
                draw_section(
                    ui,
                    theme,
                    lang,
                    InspectorSection::Phonemes,
                    lang.tr("Fonemas", "Phonemes"),
                    lang.tr("Paleta e edição de alias", "Palette and alias editing"),
                    phoneme_default || selected_ruler_alias.is_some(),
                    selected_ruler_alias.map(str::to_string),
                    active_tab,
                    RightSidebarTab::Phonemes,
                    |ui| {
                        phonemes::draw(
                            ui,
                            theme,
                            lang,
                            voicebank,
                            phoneme_state,
                            on_preview_phoneme,
                            on_insert_phoneme,
                            on_edit_phoneme,
                        );
                        draw_note_page(
                            ui,
                            note::NoteInspectorSection::Phonemes,
                            theme,
                            lang,
                            voicebank,
                            vocal_mode_params.phonemizer_mode,
                            selected_note_idx,
                            notes,
                            selected_indices,
                            selected_ruler_alias,
                            selected_resampler,
                            selected_wavtool,
                            &vocal_mode_params.flags,
                            on_edit_selected_ruler_alias,
                        );
                    },
                );

                ui.add_space(6.0);
                draw_section(
                    ui,
                    theme,
                    lang,
                    InspectorSection::Expressions,
                    lang.tr("Expressões", "Expressions"),
                    lang.tr(
                        "Dinâmica, timbre e articulação",
                        "Dynamics, timbre and articulation",
                    ),
                    expressions_default,
                    None,
                    active_tab,
                    RightSidebarTab::Expressions,
                    |ui| {
                        draw_note_page(
                            ui,
                            note::NoteInspectorSection::Expressions,
                            theme,
                            lang,
                            voicebank,
                            vocal_mode_params.phonemizer_mode,
                            selected_note_idx,
                            notes,
                            selected_indices,
                            selected_ruler_alias,
                            selected_resampler,
                            selected_wavtool,
                            &vocal_mode_params.flags,
                            on_edit_selected_ruler_alias,
                        );
                    },
                );

                ui.add_space(6.0);
                draw_section(
                    ui,
                    theme,
                    lang,
                    InspectorSection::Envelope,
                    lang.tr("Envelope e vibrato", "Envelope & vibrato"),
                    lang.tr(
                        "Fade, crossfade, portamento e vibrato",
                        "Fade, crossfade, portamento and vibrato",
                    ),
                    envelope_default,
                    None,
                    active_tab,
                    RightSidebarTab::Envelope,
                    |ui| {
                        draw_note_page(
                            ui,
                            note::NoteInspectorSection::Envelope,
                            theme,
                            lang,
                            voicebank,
                            vocal_mode_params.phonemizer_mode,
                            selected_note_idx,
                            notes,
                            selected_indices,
                            selected_ruler_alias,
                            selected_resampler,
                            selected_wavtool,
                            &vocal_mode_params.flags,
                            on_edit_selected_ruler_alias,
                        );
                    },
                );

                ui.add_space(6.0);
                draw_section(
                    ui,
                    theme,
                    lang,
                    InspectorSection::Engine,
                    lang.tr("Motor e renderização", "Engine & rendering"),
                    lang.tr(
                        "Resampler, wavtool e perfil de áudio",
                        "Resampler, wavtool and audio profile",
                    ),
                    engine_default,
                    Some(selected_resampler.clone()),
                    active_tab,
                    RightSidebarTab::Engine,
                    |ui| {
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
                    },
                );
            });
    });
}

fn draw_context_header(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    selected_note_idx: Option<usize>,
    selected_indices: &std::collections::HashSet<usize>,
    selected_ruler_alias: Option<&str>,
    compact: bool,
) {
    Frame::none()
        .fill(theme.c32_alpha(theme.bg_header, 0.88))
        .rounding(theme.ui_rounding())
        .stroke(Stroke::new(1.0_f32, theme.card_stroke().color))
        .inner_margin(egui::Margin::symmetric(
            10.0,
            if compact { 8.0 } else { 7.0 },
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(lang.tr("INSPETOR", "INSPECTOR"))
                        .monospace()
                        .size(10.0)
                        .strong()
                        .color(theme.accent_c32()),
                );
                ui.separator();
                let context = if selected_indices.len() > 1 {
                    format!("{} {}", selected_indices.len(), lang.tr("notas", "notes"))
                } else if selected_note_idx.is_some() {
                    lang.tr("nota ativa", "active note").to_string()
                } else {
                    lang.tr("sem nota selecionada", "no note selected")
                        .to_string()
                };
                ui.label(
                    RichText::new(context)
                        .size(if compact { 10.5 } else { 10.0 })
                        .color(theme.text_primary_c32()),
                );
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                let singer = voicebank
                    .map(|vb| vb.name.as_str())
                    .unwrap_or_else(|| lang.tr("Cantor padrão", "Default singer"));
                context_badge(ui, singer, theme.accent_c32());
                if let Some(alias) = selected_ruler_alias {
                    context_badge(ui, alias, Color32::from_rgb(0, 210, 235));
                }
                if selected_indices.len() > 1 {
                    context_badge(
                        ui,
                        lang.tr("edição em grupo", "group edit"),
                        Color32::from_rgb(255, 177, 96),
                    );
                }
            });
        });
}

fn context_badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(
        RichText::new(text)
            .size(9.0)
            .color(color)
            .background_color(color.linear_multiply(0.10)),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_note_page(
    ui: &mut egui::Ui,
    page: note::NoteInspectorSection,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    global_phonemizer_mode: crate::phonemizer::PhonemizerMode,
    selected_note_idx: Option<usize>,
    notes: &mut [UNote],
    selected_indices: &std::collections::HashSet<usize>,
    selected_ruler_alias: Option<&str>,
    selected_resampler: &mut String,
    selected_wavtool: &mut String,
    inherited_flags: &str,
    on_edit_selected_ruler_alias: &mut dyn FnMut(),
) {
    note::draw(
        ui,
        page,
        theme,
        lang,
        voicebank,
        global_phonemizer_mode,
        selected_note_idx,
        notes,
        selected_indices,
        selected_ruler_alias,
        selected_resampler,
        selected_wavtool,
        inherited_flags,
        on_edit_selected_ruler_alias,
    );
}

fn singer_badge(voicebank: Option<&Voicebank>, lang: crate::config::AppLanguage) -> Option<String> {
    voicebank
        .map(|vb| vb.name.clone())
        .or_else(|| Some(lang.tr("padrão", "default").to_string()))
}

fn draw_section(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    section: InspectorSection,
    title: &str,
    description: &str,
    default_open: bool,
    badge: Option<String>,
    active_tab: &mut RightSidebarTab,
    compatibility_tab: RightSidebarTab,
    draw_body: impl FnOnce(&mut egui::Ui),
) {
    let section_id = match section {
        InspectorSection::Singer => "singer",
        InspectorSection::Note => "note",
        InspectorSection::Phonemes => "phonemes",
        InspectorSection::Expressions => "expressions",
        InspectorSection::Envelope => "envelope",
        InspectorSection::Engine => "engine",
    };
    let display_title = badge
        .as_deref()
        .map(|value| format!("{title}  ·  {value}"))
        .unwrap_or_else(|| title.to_string());
    let header_text = RichText::new(display_title)
        .size(11.5)
        .strong()
        .color(theme.text_primary_c32());
    let header = egui::CollapsingHeader::new(header_text)
        .id_salt(("inspector_section", section_id))
        .default_open(default_open)
        .show_background(true)
        .show(ui, |ui| {
            ui.add_space(2.0);
            ui.label(
                RichText::new(description)
                    .size(9.0)
                    .color(theme.text_muted_c32()),
            );
            ui.add_space(5.0);
            draw_body(ui);
            ui.add_space(4.0);
        });

    if header.header_response.clicked() {
        *active_tab = compatibility_tab;
    }

    let _ = lang;
}
