mod engine;
mod note;
mod phonemes;
mod singer;

use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::theme::ThemeConfig;
use crate::gui::types::RightSidebarTab;
use crate::oto::Voicebank;
use crate::project::model::UNote;
pub use crate::renderer::RenderOptions as VocalModeParams;
use eframe::egui::{self, Frame, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn draw_unified_panel(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    recent_voicebanks: &[PathBuf],
    singers_list: &[crate::oto::SingerInfo],
    active_native_singer: Option<&crate::oto::SingerInfo>,
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
    let compact = !crate::gui::touch_first_layout(ui.ctx());
    ui.vertical(|ui| {
        draw_context_header(
            ui,
            theme,
            lang,
            voicebank,
            active_native_singer,
            selected_note_idx,
            selected_indices,
            selected_ruler_alias,
            compact,
        );
        ui.add_space(6.0);
        draw_group_tabs(ui, theme, lang, active_tab);
        ui.add_space(7.0);

        egui::ScrollArea::vertical()
            .id_salt("unified_inspector_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| match inspector_group(*active_tab) {
                InspectorGroup::Track => {
                    draw_group_description(ui, theme, lang, InspectorGroup::Track, *active_tab);
                    singer::draw(
                        ui,
                        theme,
                        lang,
                        voicebank,
                        recent_voicebanks,
                        singers_list,
                        active_native_singer,
                        singer_search_query,
                        singers_paths,
                        vocal_mode_params,
                        on_load_vb,
                        on_add_singers_dir,
                        on_reload_singers,
                        on_open_gallery,
                    );
                }
                InspectorGroup::Note => {
                    draw_subtabs(ui, theme, lang, active_tab, InspectorGroup::Note);
                    draw_group_description(ui, theme, lang, InspectorGroup::Note, *active_tab);
                    if *active_tab == RightSidebarTab::Phonemes {
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
                    } else {
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
                    }
                }
                InspectorGroup::Expression => {
                    draw_subtabs(ui, theme, lang, active_tab, InspectorGroup::Expression);
                    draw_group_description(
                        ui,
                        theme,
                        lang,
                        InspectorGroup::Expression,
                        *active_tab,
                    );
                    let page = if *active_tab == RightSidebarTab::Envelope {
                        note::NoteInspectorSection::Envelope
                    } else {
                        note::NoteInspectorSection::Expressions
                    };
                    draw_note_page(
                        ui,
                        page,
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
                }
                InspectorGroup::Engine => {
                    draw_group_description(ui, theme, lang, InspectorGroup::Engine, *active_tab);
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
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_context_header(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    active_native_singer: Option<&crate::oto::SingerInfo>,
    selected_note_idx: Option<usize>,
    selected_indices: &std::collections::HashSet<usize>,
    selected_ruler_alias: Option<&str>,
    compact: bool,
) {
    let context = if selected_indices.len() > 1 {
        format!(
            "{} {}",
            selected_indices.len(),
            lang.tr("notas selecionadas", "notes selected")
        )
    } else if selected_note_idx.is_some() {
        lang.tr("Nota selecionada", "Selected note").to_string()
    } else {
        lang.tr("Nenhuma nota selecionada", "No note selected")
            .to_string()
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(lang.tr("INSPETOR", "INSPECTOR"))
                .size(if compact { 9.0 } else { 10.0 })
                .strong()
                .color(theme.text_muted_c32()),
        );
        ui.add_space(5.0);
        ui.label(
            RichText::new(context)
                .size(if compact { 10.0 } else { 11.0 })
                .strong()
                .color(theme.text_primary_c32()),
        );
    });
    ui.add_space(3.0);
    ui.horizontal_wrapped(|ui| {
        let singer = active_native_singer
            .map(|native| native.name.as_str())
            .or_else(|| voicebank.map(|vb| vb.name.as_str()))
            .unwrap_or_else(|| lang.tr("Cantor padrão", "Default singer"));
        ui.label(
            RichText::new(singer)
                .size(9.5)
                .color(theme.text_muted_c32()),
        );
        if let Some(alias) = selected_ruler_alias {
            ui.label(RichText::new("·").size(9.5).color(theme.text_muted_c32()));
            ui.label(
                RichText::new(alias)
                    .size(9.5)
                    .color(theme.accent_soft_c32()),
            );
        }
        if selected_indices.len() > 1 {
            ui.label(RichText::new("·").size(9.5).color(theme.text_muted_c32()));
            ui.label(
                RichText::new(lang.tr("edição em grupo", "group edit"))
                    .size(9.5)
                    .color(theme.text_muted_c32()),
            );
        }
    });
    ui.add_space(5.0);
    ui.separator();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorGroup {
    Track,
    Note,
    Expression,
    Engine,
}

fn inspector_group(section: RightSidebarTab) -> InspectorGroup {
    match section {
        RightSidebarTab::SingerTrack => InspectorGroup::Track,
        RightSidebarTab::Note | RightSidebarTab::Phonemes => InspectorGroup::Note,
        RightSidebarTab::Expressions | RightSidebarTab::Envelope => InspectorGroup::Expression,
        RightSidebarTab::Engine => InspectorGroup::Engine,
    }
}

fn draw_group_tabs(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    active_tab: &mut RightSidebarTab,
) {
    for (left, right) in [
        (InspectorGroup::Track, InspectorGroup::Note),
        (InspectorGroup::Expression, InspectorGroup::Engine),
    ] {
        ui.columns(2, |columns| {
            for (column, group) in [left, right].into_iter().enumerate() {
                let column_ui = &mut columns[column];
                let selected = inspector_group(*active_tab) == group;
                let label = group_label(group, lang);
                let text_color = if selected {
                    theme.text_primary_c32()
                } else {
                    theme.text_muted_c32()
                };
                let response = Frame::none()
                    .inner_margin(egui::Margin::symmetric(7.0, 5.0))
                    .show(column_ui, |ui| {
                        ui.add_sized(
                            [ui.available_width(), 18.0],
                            egui::Label::new(
                                RichText::new(label).size(10.5).strong().color(text_color),
                            )
                            .sense(egui::Sense::click()),
                        )
                    })
                    .inner;
                if selected {
                    let indicator = Rect::from_min_max(
                        Pos2::new(response.rect.left() + 8.0, response.rect.bottom() + 1.0),
                        Pos2::new(response.rect.right() - 8.0, response.rect.bottom() + 2.5),
                    );
                    column_ui.painter().rect_filled(
                        indicator,
                        Rounding::same(1.0),
                        theme.accent_soft_c32(),
                    );
                }
                if response.clicked() {
                    *active_tab = group_default_section(group);
                }
            }
        });
    }
}

fn draw_subtabs(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    active_tab: &mut RightSidebarTab,
    group: InspectorGroup,
) {
    let choices: &[(RightSidebarTab, &str, &str)] = match group {
        InspectorGroup::Note => &[
            (RightSidebarTab::Note, "Propriedades", "Properties"),
            (RightSidebarTab::Phonemes, "Fonemas", "Phonemes"),
        ],
        InspectorGroup::Expression => &[
            (RightSidebarTab::Expressions, "Controles", "Controls"),
            (
                RightSidebarTab::Envelope,
                "Envelope e vibrato",
                "Envelope & vibrato",
            ),
        ],
        _ => return,
    };
    ui.horizontal_wrapped(|ui| {
        for (tab, pt, en) in choices {
            let selected = *active_tab == *tab;
            let label = lang.tr(pt, en);
            let color = if selected {
                theme.text_primary_c32()
            } else {
                theme.text_muted_c32()
            };
            let response =
                ui.selectable_label(selected, RichText::new(label).size(9.5).color(color));
            if response.clicked() {
                *active_tab = *tab;
            }
        }
    });
    ui.add_space(5.0);
    ui.separator();
    ui.add_space(5.0);
}

fn group_label(group: InspectorGroup, lang: crate::config::AppLanguage) -> &'static str {
    match group {
        InspectorGroup::Track => lang.tr("Faixa", "Track"),
        InspectorGroup::Note => lang.tr("Nota", "Note"),
        InspectorGroup::Expression => lang.tr("Expressão", "Expression"),
        InspectorGroup::Engine => lang.tr("Motor", "Engine"),
    }
}

fn group_default_section(group: InspectorGroup) -> RightSidebarTab {
    match group {
        InspectorGroup::Track => RightSidebarTab::SingerTrack,
        InspectorGroup::Note => RightSidebarTab::Note,
        InspectorGroup::Expression => RightSidebarTab::Expressions,
        InspectorGroup::Engine => RightSidebarTab::Engine,
    }
}

fn group_description(
    group: InspectorGroup,
    section: RightSidebarTab,
    lang: crate::config::AppLanguage,
) -> &'static str {
    match (group, section) {
        (InspectorGroup::Track, _) => lang.tr(
            "Voicebank, autor e cantor ativo",
            "Voicebank, author and active singer",
        ),
        (InspectorGroup::Note, RightSidebarTab::Phonemes) => {
            lang.tr("Paleta e edição de alias", "Palette and alias editing")
        }
        (InspectorGroup::Note, _) => lang.tr(
            "Pitch, duração, dinâmica e articulação",
            "Pitch, duration, dynamics and articulation",
        ),
        (InspectorGroup::Expression, RightSidebarTab::Envelope) => lang.tr(
            "Fade, crossfade, portamento e vibrato",
            "Fade, crossfade, portamento and vibrato",
        ),
        (InspectorGroup::Expression, _) => lang.tr(
            "Dinâmica, timbre e articulação",
            "Dynamics, timbre and articulation",
        ),
        (InspectorGroup::Engine, _) => lang.tr(
            "Resampler, wavtool e perfil de áudio",
            "Resampler, wavtool and audio profile",
        ),
    }
}

fn draw_group_description(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    group: InspectorGroup,
    section: RightSidebarTab,
) {
    ui.label(
        RichText::new(group_description(group, section, lang))
            .size(9.0)
            .color(theme.text_muted_c32()),
    );
    ui.add_space(5.0);
}

#[cfg(test)]
mod tests {
    use super::{group_default_section, inspector_group, InspectorGroup};
    use crate::gui::types::RightSidebarTab;

    #[test]
    fn legacy_inspector_sections_map_to_the_four_visual_groups() {
        assert_eq!(
            inspector_group(RightSidebarTab::SingerTrack),
            InspectorGroup::Track
        );
        assert_eq!(inspector_group(RightSidebarTab::Note), InspectorGroup::Note);
        assert_eq!(
            inspector_group(RightSidebarTab::Phonemes),
            InspectorGroup::Note
        );
        assert_eq!(
            inspector_group(RightSidebarTab::Expressions),
            InspectorGroup::Expression
        );
        assert_eq!(
            inspector_group(RightSidebarTab::Envelope),
            InspectorGroup::Expression
        );
        assert_eq!(
            inspector_group(RightSidebarTab::Engine),
            InspectorGroup::Engine
        );
    }

    #[test]
    fn selecting_a_visual_group_opens_its_default_legacy_section() {
        assert_eq!(
            group_default_section(InspectorGroup::Track),
            RightSidebarTab::SingerTrack
        );
        assert_eq!(
            group_default_section(InspectorGroup::Note),
            RightSidebarTab::Note
        );
        assert_eq!(
            group_default_section(InspectorGroup::Expression),
            RightSidebarTab::Expressions
        );
        assert_eq!(
            group_default_section(InspectorGroup::Engine),
            RightSidebarTab::Engine
        );
    }
}
