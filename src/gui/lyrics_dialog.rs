use crate::gui::theme::ThemeConfig;
use crate::project::model::UNote;
use crate::config::AppLanguage;
use eframe::egui::{self, Color32, Frame, RichText, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitMode {
    Spaces,
    HyphensAndSpaces,
    JapaneseSyllables,
    PortugueseSyllables,
}

impl SplitMode {
    pub fn label(&self, lang: AppLanguage) -> &'static str {
        match self {
            SplitMode::Spaces => lang.tr("Espaços (Palavra por Nota)", "Spaces (Word per Note)"),
            SplitMode::HyphensAndSpaces => lang.tr("Hífens e Espaços (ex: ka-ma-feu)", "Hyphens and Spaces (e.g. ka-ma-feu)"),
            SplitMode::JapaneseSyllables => lang.tr("Japonês (Romaji / Kana por sílaba)", "Japanese (Romaji / Kana per syllable)"),
            SplitMode::PortugueseSyllables => lang.tr("Português (Divisão Silábica Automática)", "Portuguese (Automatic Syllable Division)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LyricsDialogState {
    pub is_open: bool,
    pub input_text: String,
    pub split_mode: SplitMode,
    pub target_mode: LyricsTargetMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsTargetMode {
    SelectedNotes,
    FromFirstSelected,
    FromPlayhead,
}

impl LyricsTargetMode {
    pub fn label(&self, lang: AppLanguage) -> &'static str {
        match self {
            LyricsTargetMode::SelectedNotes => lang.tr("Somente Notas Selecionadas", "Selected Notes Only"),
            LyricsTargetMode::FromFirstSelected => lang.tr("A partir da primeira nota selecionada", "From First Selected Note"),
            LyricsTargetMode::FromPlayhead => lang.tr("A partir da posição do Playhead", "From Playhead Position"),
        }
    }
}

impl Default for LyricsDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            input_text: String::new(),
            split_mode: SplitMode::HyphensAndSpaces,
            target_mode: LyricsTargetMode::SelectedNotes,
        }
    }
}

/// Splits input text according to the selected mode
pub fn split_lyrics(text: &str, mode: SplitMode) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    match mode {
        SplitMode::Spaces => trimmed
            .split_whitespace()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect(),

        SplitMode::HyphensAndSpaces => {
            let mut result = Vec::new();
            for word in trimmed.split_whitespace() {
                for syllable in word.split('-') {
                    let s = syllable.trim();
                    if !s.is_empty() {
                        result.push(s.to_string());
                    }
                }
            }
            result
        }

        SplitMode::JapaneseSyllables => {
            // Tokenize japanese kana or romaji syllables
            let mut result = Vec::new();
            for word in trimmed.split_whitespace() {
                let chars: Vec<char> = word.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    let c = chars[i];
                    // Check compound kana (e.g. kya, sho, tsu small kana ゃ ゅ ょ ぁ ぃ ぅ ぇ ぉ ゎ ヵ ヶ)
                    if i + 1 < chars.len()
                        && matches!(
                            chars[i + 1],
                            'ゃ' | 'ゅ'
                                | 'ょ'
                                | 'ぁ'
                                | 'ぃ'
                                | 'ぅ'
                                | 'ぇ'
                                | 'ぉ'
                                | 'ゎ'
                                | 'ャ'
                                | 'ュ'
                                | 'ョ'
                                | 'ァ'
                                | 'ィ'
                                | 'ゥ'
                                | 'ェ'
                                | 'ォ'
                                | 'ヮ'
                        )
                    {
                        let s: String = chars[i..=i + 1].iter().collect();
                        result.push(s);
                        i += 2;
                        continue;
                    }

                    // Check if it's romaji (latin characters)
                    if c.is_ascii_alphabetic() {
                        let lower_c = c.to_ascii_lowercase();
                        if matches!(lower_c, 'a' | 'e' | 'i' | 'o' | 'u') {
                            result.push(c.to_string());
                            i += 1;
                            continue;
                        }

                        // Consonant cluster followed by vowel (or coda 'n')
                        let mut end = i + 1;
                        while end < chars.len() && chars[end].is_ascii_alphabetic() {
                            let next_c = chars[end].to_ascii_lowercase();
                            if matches!(next_c, 'a' | 'e' | 'i' | 'o' | 'u') {
                                end += 1;
                                break;
                            }
                            if lower_c == 'n' {
                                break;
                            }
                            end += 1;
                        }
                        let s: String = chars[i..end].iter().collect();
                        result.push(s);
                        i = end;
                        continue;
                    }

                    // Regular single kana or punctuation
                    result.push(c.to_string());
                    i += 1;
                }
            }
            result
        }

        SplitMode::PortugueseSyllables => {
            // Intelligent Portuguese syllabification
            let mut result = Vec::new();
            for word in trimmed.split_whitespace() {
                let syllables = syllabify_portuguese(word);
                for syl in syllables {
                    if !syl.is_empty() {
                        result.push(syl);
                    }
                }
            }
            result
        }
    }
}

/// Simple rule-based Portuguese syllable splitter
fn syllabify_portuguese(word: &str) -> Vec<String> {
    if word.contains('-') {
        return word.split('-').map(|s| s.to_string()).collect();
    }

    let chars: Vec<char> = word.chars().collect();
    if chars.len() <= 3 {
        return vec![word.to_string()];
    }

    let is_vowel = |c: char| -> bool {
        matches!(
            c.to_ascii_lowercase(),
            'a' | 'e'
                | 'i'
                | 'o'
                | 'u'
                | 'á'
                | 'é'
                | 'í'
                | 'ó'
                | 'ú'
                | 'â'
                | 'ê'
                | 'ô'
                | 'ã'
                | 'õ'
                | 'à'
                | 'ü'
        )
    };

    let mut result = Vec::new();
    let mut cur = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        cur.push(c);

        if is_vowel(c) {
            // Lookahead for next vowel or consonant cluster
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if !is_vowel(next) {
                    if i + 2 < chars.len() && is_vowel(chars[i + 2]) {
                        // VCV pattern -> break before the consonant
                        result.push(cur.clone());
                        cur.clear();
                    } else if i + 2 < chars.len() && !is_vowel(chars[i + 2]) {
                        // VCCV pattern -> check digraphs like ch, lh, nh, qu, gu, tr, pr, bl, cl, etc.
                        let pair: String = [next, chars[i + 2]].iter().collect();
                        let pair_lower = pair.to_lowercase();
                        let inseparable = matches!(
                            pair_lower.as_str(),
                            "ch" | "lh"
                                | "nh"
                                | "qu"
                                | "gu"
                                | "pr"
                                | "br"
                                | "tr"
                                | "dr"
                                | "cr"
                                | "gr"
                                | "fr"
                                | "vr"
                                | "pl"
                                | "bl"
                                | "cl"
                                | "gl"
                                | "fl"
                        );
                        if inseparable {
                            result.push(cur.clone());
                            cur.clear();
                        } else {
                            cur.push(next);
                            result.push(cur.clone());
                            cur.clear();
                            i += 1;
                        }
                    }
                } else {
                    // VV pattern (hiatus vs diphthong)
                    let v_pair = format!("{}{}", c.to_ascii_lowercase(), next.to_ascii_lowercase());
                    let is_hiatus = matches!(
                        v_pair.as_str(),
                        "aa" | "ee" | "oo" | "oa" | "eo" | "oe" | "ao"
                    );
                    if is_hiatus {
                        result.push(cur.clone());
                        cur.clear();
                    }
                }
            }
        }
        i += 1;
    }

    if !cur.is_empty() {
        if let Some(last) = result.last_mut() {
            if !cur.chars().any(is_vowel) {
                last.push_str(&cur);
            } else {
                result.push(cur);
            }
        } else {
            result.push(cur);
        }
    }

    if result.is_empty() {
        vec![word.to_string()]
    } else {
        result
    }
}

pub fn draw_lyrics_dialog(
    ctx: &egui::Context,
    lang: AppLanguage,
    theme: &ThemeConfig,
    state: &mut LyricsDialogState,
    notes: &mut [UNote],
    selected_indices: &std::collections::HashSet<usize>,
    playhead_ms: f64,
    on_applied: &mut dyn FnMut(),
) {
    if !state.is_open {
        return;
    }

    let mut window_open = state.is_open;
    let mut apply_clicked = false;
    let mut close_clicked = false;

    Window::new(RichText::new(lang.tr("Distribuidor Inteligente de Letras", "Smart Lyrics Distributor")).strong().color(theme.accent_c32()))
        .open(&mut window_open)
        .resizable(true)
        .default_width(440.0)
        .default_height(360.0)
        .frame(Frame::window(&ctx.style()).fill(theme.bg_panel_c32()).stroke(theme.card_stroke()))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(lang.tr("Cole a letra ou texto da música para distribuir pelas notas:", "Paste song lyrics or text to distribute across notes:")).size(11.0).color(theme.text_muted_c32()));
                ui.add_space(4.0);

                ui.add(
                    egui::TextEdit::multiline(&mut state.input_text)
                        .hint_text(lang.tr(
                            "Exemplo: O sol bri-lha no ho-ri-zon-te azul\nou: a-ri-ga-to-u go-za-i-ma-su",
                            "Example: The sun shi-nes in the blue ho-ri-zon\nor: a-ri-ga-to-u go-za-i-ma-su",
                        ))
                        .desired_rows(4)
                        .desired_width(ui.available_width()),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(lang.tr("Modo de Divisão:", "Split Mode:")).size(10.5).color(theme.text_muted_c32()));
                    egui::ComboBox::from_id_salt("lyrics_split_mode_combo")
                        .selected_text(state.split_mode.label(lang))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.split_mode, SplitMode::HyphensAndSpaces, SplitMode::HyphensAndSpaces.label(lang));
                            ui.selectable_value(&mut state.split_mode, SplitMode::Spaces, SplitMode::Spaces.label(lang));
                            ui.selectable_value(&mut state.split_mode, SplitMode::PortugueseSyllables, SplitMode::PortugueseSyllables.label(lang));
                            ui.selectable_value(&mut state.split_mode, SplitMode::JapaneseSyllables, SplitMode::JapaneseSyllables.label(lang));
                        });
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(lang.tr("Aplicar em:", "Apply to:")).size(10.5).color(theme.text_muted_c32()));
                    egui::ComboBox::from_id_salt("lyrics_target_mode_combo")
                        .selected_text(state.target_mode.label(lang))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.target_mode, LyricsTargetMode::SelectedNotes, LyricsTargetMode::SelectedNotes.label(lang));
                            ui.selectable_value(&mut state.target_mode, LyricsTargetMode::FromFirstSelected, LyricsTargetMode::FromFirstSelected.label(lang));
                            ui.selectable_value(&mut state.target_mode, LyricsTargetMode::FromPlayhead, LyricsTargetMode::FromPlayhead.label(lang));
                        });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                let tokens = split_lyrics(&state.input_text, state.split_mode);

                // Target note calculation
                let target_indices: Vec<usize> = match state.target_mode {
                    LyricsTargetMode::SelectedNotes => {
                        let mut indices: Vec<usize> = selected_indices.iter().copied().collect();
                        indices.sort_by_key(|&idx| {
                            notes.get(idx).map(|n| n.position_ms as i64).unwrap_or(0)
                        });
                        indices
                    }
                    LyricsTargetMode::FromFirstSelected => {
                        let first = selected_indices.iter().copied().min_by_key(|&idx| {
                            notes.get(idx).map(|n| n.position_ms as i64).unwrap_or(0)
                        }).unwrap_or(0);
                        (first..notes.len()).collect()
                    }
                    LyricsTargetMode::FromPlayhead => {
                        notes
                            .iter()
                            .enumerate()
                            .filter(|(_, n)| n.position_ms >= playhead_ms)
                            .map(|(i, _)| i)
                            .collect()
                    }
                };

                ui.label(RichText::new(format!(
                    "{} ({} {} -> {} {})",
                    lang.tr("Prévia da Distribuição", "Distribution Preview"),
                    tokens.len(),
                    lang.tr("sílabas", "syllables"),
                    target_indices.len().min(tokens.len()),
                    lang.tr("notas afetadas", "affected notes")
                )).strong().size(11.0).color(theme.accent_c32()));
                ui.add_space(4.0);

                Frame::none()
                    .fill(theme.card_bg_c32())
                    .rounding(theme.ui_rounding())
                    .stroke(theme.card_stroke())
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("lyrics_preview_scroll")
                            .max_height(100.0)
                            .show(ui, |ui| {
                                if tokens.is_empty() {
                                    ui.label(RichText::new(lang.tr("Digite um texto acima para ver a prévia das sílabas.", "Type lyrics above to preview syllables.")).size(9.5).italics().color(theme.text_muted_c32()));
                                } else {
                                    ui.horizontal_wrapped(|ui| {
                                        for (idx, token) in tokens.iter().enumerate() {
                                            let has_note = idx < target_indices.len();
                                            let badge_color = if has_note {
                                                theme.accent_c32()
                                             } else {
                                                theme.text_muted_c32()
                                            };
                                            ui.label(
                                                RichText::new(format!("{}: [{}]", idx + 1, token))
                                                    .size(10.0)
                                                    .strong()
                                                    .color(badge_color),
                                            );
                                        }
                                    });
                                }
                            });
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let can_apply = !tokens.is_empty() && !target_indices.is_empty();
                    ui.add_enabled_ui(can_apply, |ui| {
                        if ui.button(RichText::new(lang.tr("Aplicar Letras às Notas", "Apply Lyrics to Notes")).strong().size(12.0).color(if can_apply { Color32::WHITE } else { theme.text_muted_c32() })).clicked() {
                            apply_clicked = true;
                        }
                    });

                    if ui.button(RichText::new(lang.tr("Fechar", "Close")).size(11.0)).clicked() {
                        close_clicked = true;
                    }
                });

                if apply_clicked {
                    on_applied();
                    for (token_idx, &note_idx) in target_indices.iter().enumerate() {
                        if let Some(token) = tokens.get(token_idx) {
                            if note_idx < notes.len() {
                                notes[note_idx].lyric = token.clone();
                            }
                        }
                    }
                    close_clicked = true;
                }
            });
        });

    state.is_open = window_open && !close_clicked;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_lyrics_spaces() {
        let text = "o sol brilha";
        let tokens = split_lyrics(text, SplitMode::Spaces);
        assert_eq!(tokens, vec!["o", "sol", "brilha"]);
    }

    #[test]
    fn test_split_lyrics_hyphens() {
        let text = "ka-ma-feu stu-dio";
        let tokens = split_lyrics(text, SplitMode::HyphensAndSpaces);
        assert_eq!(tokens, vec!["ka", "ma", "feu", "stu", "dio"]);
    }

    #[test]
    fn test_split_lyrics_japanese() {
        let text = "arigatou";
        let tokens = split_lyrics(text, SplitMode::JapaneseSyllables);
        assert_eq!(tokens, vec!["a", "ri", "ga", "to", "u"]);
    }

    #[test]
    fn test_split_lyrics_portuguese() {
        let text = "kamafeu cantar";
        let tokens = split_lyrics(text, SplitMode::PortugueseSyllables);
        assert!(!tokens.is_empty());
    }
}
