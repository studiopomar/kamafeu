pub mod brapa;
pub mod chinese;
pub mod diphone;
pub mod english;
pub mod french;
pub mod japanese;
pub mod korean;
pub mod portuguese;
pub mod portuguese_cvc;
pub mod romaji;
pub mod russian;
pub mod spanish;
pub mod vccv;

/// Upstream OpenUtau revision used by the compatibility corpus and ports.
pub const OPENUTAU_COMPATIBILITY_COMMIT: &str = "13184085f2cee51c174de9ee99163a76ee970e8c";

/// Descrição apresentada pela UI para cada família de fonemizadores.
/// `source` é deliberadamente explícito para não confundir ports com código nativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhonemizerInfo {
    pub mode: PhonemizerMode,
    pub language: &'static str,
    pub name: &'static str,
    pub source: &'static str,
}

/// Status real da implementação usada pelo dispatcher.
///
/// Isto é deliberadamente separado de `PhonemizerMode`: um modo pode existir
/// na interface antes de o port correspondente do OpenUtau estar concluído.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhonemizerImplementation {
    /// Algoritmo portado e validado contra fixtures do OpenUtau.
    OpenUtau,
    /// O programa apenas conserva aliases fornecidos pelo projeto/usuário.
    Passthrough,
    /// Ainda não há port seguro; regras Kamafeu não devem ser apresentadas
    /// como compatibilidade OpenUtau.
    Unsupported,
}

pub use brapa::{BrapaCVCPhonemizer, VccvBrapaPhonemizer};
pub use english::EnglishPhonemizer;
pub use japanese::JapanesePhonemizer as CoreJapanesePhonemizer;
pub use portuguese::PortuguesePhonemizer;

/// Single entry point for phonemization.  Keeping this boundary explicit is
/// important while the individual OpenUtau ports are migrated: callers no
/// longer need to know which legacy module used to implement a mode.
pub struct OpenUtauPhonemizer;

impl OpenUtauPhonemizer {
    pub fn implementation(mode: PhonemizerMode) -> PhonemizerImplementation {
        match mode {
            // Manual mode does not phonemize text. It is a valid passthrough
            // for explicit aliases and therefore does not claim an OpenUtau
            // algorithm.
            PhonemizerMode::None => PhonemizerImplementation::Passthrough,
            // These families have a concrete dispatcher and source-derived
            // alias topology. Golden cases live beside each port; modes not
            // represented here remain visibly unsupported instead of being
            // mislabeled as compatible.
            PhonemizerMode::BasicCV
            | PhonemizerMode::VCV
            | PhonemizerMode::CVVC
            | PhonemizerMode::JapanesePresamp
            | PhonemizerMode::EnglishArpasing
            | PhonemizerMode::EnglishArpasingPlus
            | PhonemizerMode::EnglishVCCV
            | PhonemizerMode::EnglishG2P
            | PhonemizerMode::EnglishXSampa
            | PhonemizerMode::EnglishCpV
            | PhonemizerMode::PortugueseBrazilianCVC
            | PhonemizerMode::PortugueseBrapaVCCV
            | PhonemizerMode::PortugueseBrapaCVC
            | PhonemizerMode::PortugueseCVVC
            | PhonemizerMode::PortugueseVCV
            | PhonemizerMode::PortugueseG2P
            | PhonemizerMode::FrenchG2P
            | PhonemizerMode::FrenchCMUSphinx
            | PhonemizerMode::FrenchCVVC
            | PhonemizerMode::FrenchVCCV
            | PhonemizerMode::RussianG2P
            | PhonemizerMode::RussianCVC
            | PhonemizerMode::RussianVCCV
            | PhonemizerMode::SpanishG2P
            | PhonemizerMode::SpanishMakkusan
            | PhonemizerMode::SpanishSyllable
            | PhonemizerMode::SpanishVCCV
            | PhonemizerMode::ChineseG2P
            | PhonemizerMode::ChineseCVV
            | PhonemizerMode::ChineseCVVPlus
            | PhonemizerMode::CantoneseG2P
            | PhonemizerMode::CantoneseSyo
            | PhonemizerMode::ChineseCVVC
            | PhonemizerMode::ChineseVCV
            | PhonemizerMode::CantoneseCVVC
            | PhonemizerMode::KoreanCV
            | PhonemizerMode::KoreanCVC
            | PhonemizerMode::KoreanVCV
            | PhonemizerMode::KoreanCVVC
            | PhonemizerMode::KoreanCVCCV
            | PhonemizerMode::KoreanCBNN
            | PhonemizerMode::KoreanCVVCStandard
            | PhonemizerMode::GermanG2P
            | PhonemizerMode::GermanVCCV
            | PhonemizerMode::GermanDiphone
            | PhonemizerMode::TurkishG2P
            | PhonemizerMode::TurkishCVVC
            | PhonemizerMode::ArabicG2P
            | PhonemizerMode::IcelandicG2P => PhonemizerImplementation::OpenUtau,
        }
    }

    pub fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        let implementation = Self::implementation(mode);
        if implementation == PhonemizerImplementation::Unsupported {
            crate::terminal_log::warn(
                "Phonemizer",
                format!("mode={mode:?} implementation=unsupported; using compatibility fallback"),
            );
        } else {
            crate::terminal_log::debug(
                "Phonemizer",
                format!("mode={mode:?} implementation={implementation:?}"),
            );
        }
        JapanesePhonemizer::apply_legacy_dispatch(notes, vb, mode)
    }
}

use crate::oto::Voicebank;
use crate::project::model::UNote;
use std::collections::HashMap;
use std::sync::OnceLock;

static OPENUTAU_G2P_DICTIONARIES: OnceLock<HashMap<PhonemizerMode, HashMap<String, Vec<String>>>> =
    OnceLock::new();
static CANTONESE_DICTIONARY: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();

/// Retained as a source-compatible no-op. OpenUtau phonemizers own their
/// normalization and custom text substitutions; applying a second Kamafeu
/// rule layer changes the input before the official algorithm sees it.
pub fn set_custom_rules(_rules: std::collections::HashMap<String, String>) {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum PhonemizerMode {
    #[default]
    None, // Sem Fonemizador (Raw / Manual / Direto)
    BasicCV,                // JA: Japanese CV (Básico - Fonética/Romaji/Kana)
    VCV,                    // JA: Japanese VCV (Completo - Fonética/Romaji/Kana)
    CVVC,                   // JA: Japanese CVVC (Fonética/Romaji/Kana)
    JapanesePresamp,        // JA: Japanese presamp VCV & CVVC
    EnglishArpasing,        // EN: English Arpasing (Fonética Direta)
    EnglishArpasingPlus,    // EN: Arpasing+
    EnglishVCCV,            // EN: English VCCV (Fonética Direta)
    EnglishG2P,             // EN: English G2P (Palavras / Texto -> ARPABET)
    EnglishXSampa,          // EN: X-SAMPA
    EnglishCpV,             // EN: C+V
    PortugueseBrazilianCVC, // PT: Brazilian Portuguese CVC
    PortugueseBrapaVCCV,    // PT: VCCV BRAPA (xiao / PT-BR 3.7)
    PortugueseBrapaCVC,     // PT: BRAPA CVC (Fonética Direta / Tokens BRAPA)
    PortugueseCVVC,         // PT: Portuguese CVVC (Fonética Direta)
    PortugueseVCV,          // PT: Portuguese VCV (Fonética Direta)
    PortugueseG2P,          // PT: Português G2P (Palavras / Texto em Português -> Fonemas)
    FrenchG2P,
    FrenchCMUSphinx,
    FrenchCVVC,
    FrenchVCCV,
    GermanG2P,
    GermanVCCV,
    GermanDiphone,
    RussianG2P,
    RussianCVC,
    RussianVCCV,
    SpanishG2P,
    SpanishMakkusan,
    SpanishSyllable,
    SpanishVCCV,
    TurkishG2P,
    ChineseG2P,
    ChineseCVV,
    ChineseCVVPlus,
    CantoneseG2P,
    CantoneseSyo,
    TurkishCVVC,
    ChineseCVVC,
    ChineseVCV,
    CantoneseCVVC,
    KoreanCV,
    KoreanCVC,
    KoreanVCV,
    KoreanCVVC,
    KoreanCVCCV,
    KoreanCBNN,
    KoreanCVVCStandard,
    ArabicG2P,
    IcelandicG2P,
}

impl PhonemizerMode {
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|mode| format!("{mode:?}") == name)
    }

    pub const ALL: [Self; 49] = [
        Self::BasicCV,
        Self::VCV,
        Self::CVVC,
        Self::JapanesePresamp,
        Self::EnglishArpasing,
        Self::EnglishArpasingPlus,
        Self::EnglishVCCV,
        Self::EnglishG2P,
        Self::EnglishXSampa,
        Self::EnglishCpV,
        Self::PortugueseBrazilianCVC,
        Self::PortugueseBrapaVCCV,
        Self::PortugueseBrapaCVC,
        Self::PortugueseCVVC,
        Self::PortugueseVCV,
        Self::PortugueseG2P,
        Self::TurkishG2P,
        Self::TurkishCVVC,
        Self::FrenchG2P,
        Self::FrenchCMUSphinx,
        Self::FrenchCVVC,
        Self::FrenchVCCV,
        Self::GermanG2P,
        Self::GermanVCCV,
        Self::GermanDiphone,
        Self::RussianG2P,
        Self::RussianCVC,
        Self::RussianVCCV,
        Self::SpanishG2P,
        Self::SpanishMakkusan,
        Self::SpanishSyllable,
        Self::SpanishVCCV,
        Self::ChineseG2P,
        Self::ChineseCVV,
        Self::ChineseCVVPlus,
        Self::ChineseCVVC,
        Self::ChineseVCV,
        Self::CantoneseG2P,
        Self::CantoneseSyo,
        Self::CantoneseCVVC,
        Self::KoreanCV,
        Self::KoreanCVC,
        Self::KoreanVCV,
        Self::KoreanCVVC,
        Self::KoreanCVCCV,
        Self::KoreanCBNN,
        Self::KoreanCVVCStandard,
        Self::ArabicG2P,
        Self::IcelandicG2P,
    ];

    pub fn info(self) -> PhonemizerInfo {
        use PhonemizerMode::*;
        let (language, name, source) = match self {
            None => ("Manual", "Sem fonemizador", "Kamafeu nativo"),
            BasicCV => ("Japonês", "CV / VV", "OpenUtau (validado)"),
            VCV => ("Japonês", "VCV", "OpenUtau (validado)"),
            CVVC => ("Japonês", "CVVC", "OpenUtau (validado)"),
            JapanesePresamp => ("Japonês", "VCV & CVVC Presamp", "OpenUtau (validado)"),
            EnglishArpasing => ("Inglês", "Arpasing", "OpenUtau (portado)"),
            EnglishArpasingPlus => ("Inglês", "Arpasing+", "OpenUtau (portado)"),
            EnglishVCCV => ("Inglês", "VCCV", "OpenUtau (portado)"),
            EnglishG2P => ("Inglês", "G2P", "OpenUtau (portado)"),
            EnglishXSampa => ("Inglês", "X-SAMPA", "OpenUtau (portado)"),
            EnglishCpV => ("Inglês", "C+V", "OpenUtau (portado)"),
            PortugueseBrazilianCVC => ("Português", "Brasileiro CVC", "OpenUtau (portado)"),
            PortugueseBrapaVCCV => ("Português", "BRAPA VCCV", "OpenUtau (portado)"),
            PortugueseBrapaCVC => ("Português", "BRAPA CVC", "OpenUtau (portado)"),
            PortugueseCVVC => ("Português", "CVVC", "OpenUtau (portado)"),
            PortugueseVCV => ("Português", "VCV", "OpenUtau (portado)"),
            PortugueseG2P => ("Português", "G2P", "OpenUtau (portado)"),
            FrenchG2P => ("Francês", "G2P", "OpenUtau (portado)"),
            FrenchCMUSphinx => ("Francês", "CMU Sphinx", "OpenUtau (portado)"),
            FrenchCVVC => ("Francês", "CVVC", "OpenUtau (portado)"),
            FrenchVCCV => ("Francês", "VCCV", "OpenUtau (portado)"),
            GermanG2P => ("Alemão", "G2P", "OpenUtau (dados)"),
            GermanVCCV => ("Alemão", "VCCV", "OpenUtau (dados)"),
            GermanDiphone => ("Alemão", "Diphone", "OpenUtau (dados)"),
            RussianG2P => ("Russo", "G2P", "OpenUtau (portado)"),
            RussianCVC => ("Russo", "CVC", "OpenUtau (portado)"),
            RussianVCCV => ("Russo", "VCCV", "OpenUtau (portado)"),
            SpanishG2P => ("Espanhol", "G2P", "OpenUtau (portado)"),
            SpanishMakkusan => ("Espanhol", "Makkusan", "OpenUtau (portado)"),
            SpanishSyllable => ("Espanhol", "Syllable-Based", "OpenUtau (portado)"),
            SpanishVCCV => ("Espanhol", "VCCV", "OpenUtau (portado)"),
            TurkishG2P => ("Turco", "G2P", "OpenUtau (dados)"),
            ChineseG2P => ("Chinês", "G2P", "OpenUtau (portado)"),
            ChineseCVV => ("Chinês", "CVV", "OpenUtau (portado)"),
            ChineseCVVPlus => ("Chinês", "CVV+", "OpenUtau (portado)"),
            CantoneseG2P => ("Cantonês", "G2P", "OpenUtau (portado)"),
            CantoneseSyo => ("Cantonês", "Syo", "OpenUtau (portado)"),
            TurkishCVVC => ("Turco", "CVVC", "OpenUtau (dados)"),
            ChineseCVVC => ("Chinês", "CVVC", "OpenUtau (portado)"),
            ChineseVCV => ("Chinês", "VCV", "OpenUtau (portado)"),
            CantoneseCVVC => ("Cantonês", "CVVC", "OpenUtau (portado)"),
            KoreanCV => ("Coreano", "CV", "OpenUtau (portado)"),
            KoreanCVC => ("Coreano", "CVC", "OpenUtau (portado)"),
            KoreanVCV => ("Coreano", "VCV", "OpenUtau (portado)"),
            KoreanCVVC => ("Coreano", "CVVC", "OpenUtau (portado)"),
            KoreanCVCCV => ("Coreano", "CVCCV", "OpenUtau (portado)"),
            KoreanCBNN => ("Coreano", "CBNN", "OpenUtau (portado)"),
            KoreanCVVCStandard => ("Coreano", "CVVC padrão", "OpenUtau (portado)"),
            ArabicG2P => (
                "Árabe",
                "DiffSinger G2P",
                "Plugin dorayakito/OpenUtau (dados)",
            ),
            IcelandicG2P => (
                "Islandês",
                "DiffSinger G2P",
                "Plugin dorayakito/OpenUtau (dados)",
            ),
        };
        PhonemizerInfo {
            mode: self,
            language,
            name,
            source,
        }
    }

    pub fn display_name(self) -> String {
        let i = self.info();
        format!(
            "{} — {} · {} [{}]",
            i.language,
            i.name,
            self.author(),
            i.source
        )
    }

    /// Upstream author or maintainer credit shown wherever a phonemizer is
    /// selectable. These names come from the OpenUtau plugin declarations;
    /// modes backed by dictionaries identify the data source explicitly.
    pub fn author(self) -> &'static str {
        use PhonemizerMode::*;
        match self {
            None => "Kamafeu",
            BasicCV => "OpenUtau contributors",
            VCV => "OpenUtau contributors",
            CVVC => "TUBS",
            JapanesePresamp => "Maiko",
            EnglishArpasing => "OpenUtau contributors",
            EnglishArpasingPlus => "Cadlaxa",
            EnglishVCCV => "cubialpha & Mim",
            EnglishG2P => "OpenUtau / Arpabet",
            EnglishXSampa => "Lotte V",
            EnglishCpV => "Cadlaxa",
            PortugueseBrazilianCVC => "HAI-D",
            PortugueseBrapaVCCV | PortugueseBrapaCVC => "xiao / Kamafeu",
            PortugueseCVVC | PortugueseVCV | PortugueseG2P => "OpenUtau contributors",
            FrenchG2P => "OpenUtau / g2p-fr",
            FrenchCMUSphinx => "OpenUtau contributors",
            FrenchCVVC | FrenchVCCV => "Mim",
            GermanG2P => "OpenUtau / g2p-de",
            GermanVCCV => "Lotte V",
            GermanDiphone => "Lotte V",
            RussianG2P => "OpenUtau / g2p-ru",
            RussianCVC | RussianVCCV => "Heiden.BZR",
            SpanishG2P => "OpenUtau / g2p-es",
            SpanishMakkusan | SpanishSyllable | SpanishVCCV => "Lotte V",
            TurkishG2P => "OpenUtau contributors",
            ChineseG2P => "OpenUtau / pinyin",
            ChineseCVV | ChineseCVVC => "OpenUtau contributors",
            ChineseCVVPlus => "2xxbin",
            ChineseVCV => "樗儿",
            CantoneseG2P => "OpenUtau / Jyutping",
            CantoneseSyo | CantoneseCVVC => "Lotte V",
            TurkishCVVC => "ise",
            KoreanCV => "EX3",
            KoreanCVC => "NANA",
            KoreanVCV => "ldc",
            KoreanCVVC => "RYUUSEI & EX3",
            KoreanCVCCV => "RYUUSEI",
            KoreanCBNN => "EX3",
            KoreanCVVCStandard => "RYUUSEI & EX3",
            ArabicG2P => "dorayakito",
            IcelandicG2P => "dorayakito",
        }
    }
}

pub fn consonant_velocity_time_scale(velocity: f64) -> f64 {
    let velocity = if velocity.is_finite() {
        velocity
    } else {
        100.0
    };
    // Negative values are useful for deliberately lengthening consonants.
    2.0f64.powf(1.0 - velocity.clamp(-100.0, 200.0) / 100.0)
}

#[derive(Debug, Clone)]
pub struct RenderPhone {
    pub note_index: usize,
    pub lyric: String,
    pub pitch: String,
    pub position_ms: f64,
    pub duration_ms: f64,
    pub envelope: crate::dsp::envelope::UtauEnvelope,
    pub expressions: crate::project::model::UExpressions,
    pub pitch_bend: crate::project::model::UPitchBend,
    pub vibrato: crate::dsp::pitch::VibratoParam,
    pub flags: String,
    /// Alias resolved against the exact subbank selected for this phone.
    /// When absent, the renderer performs the normal voicebank lookup.
    pub resolved_alias: Option<String>,
    /// Timbre/subbank used to resolve `resolved_alias`, for diagnostics.
    pub resolved_timbre: Option<String>,
    /// Indicates that this phone was borrowed from a CV/VCV syllable to extract
    /// a terminal coda consonant (e.g. borrowing `e sa` for coda `s`), requiring
    /// truncation of the trailing vowel in rendering.
    pub coda_truncation: bool,
}

impl RenderPhone {
    pub fn midi_key(&self) -> u8 {
        crate::dsp::pitch::note_name_to_midi(&self.pitch).unwrap_or(60)
    }

    pub fn find_entry<'a>(
        &'a self,
        voicebank: &'a crate::oto::Voicebank,
    ) -> Option<&'a crate::oto::OtoEntry> {
        if let Some(alias) = self.resolved_alias.as_deref() {
            // A frozen alias is an authoritative phonemizer decision. Never
            // fall back to PrefixMap here: doing so can apply a second
            // prefix/suffix and silently switch subbanks.
            return voicebank.find_exact_entry(alias);
        }
        voicebank.find_mapped_entry(&self.lyric, &self.pitch)
    }
}

pub struct JapanesePhonemizer;

impl JapanesePhonemizer {
    pub fn extract_vowel(lyric: &str) -> Option<&'static str> {
        CoreJapanesePhonemizer::extract_vowel(lyric)
    }

    pub fn extract_consonant(lyric: &str) -> Option<&'static str> {
        CoreJapanesePhonemizer::extract_consonant(lyric)
    }

    pub fn preprocess_plus_notes(notes: &[UNote]) -> Vec<UNote> {
        let mut result: Vec<UNote> = Vec::new();
        for note in notes {
            let lyric_trimmed = note.lyric.trim();
            let is_plus = lyric_trimmed.starts_with('+');

            if is_plus {
                if let Some(last) = result.last_mut() {
                    let last_end = last.position_ms + last.duration_ms;
                    if (note.position_ms - last_end).abs() <= 2.0 {
                        last.duration_ms += note.duration_ms;
                        continue;
                    } else {
                        // Unconnected plus note: borrow lyric from preceding note
                        let mut fallback_note = note.clone();
                        fallback_note.lyric = last.lyric.clone();
                        result.push(fallback_note);
                        continue;
                    }
                }
            }

            result.push(note.clone());
        }
        result
    }

    pub fn apply_phonemizer(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        OpenUtauPhonemizer::apply(notes, vb, mode)
    }

    fn apply_legacy_dispatch(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        Self::apply_legacy_dispatch_with_cvvc_port(notes, vb, mode, true)
    }

    fn apply_legacy_dispatch_with_cvvc_port(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
        allow_cvvc_port: bool,
    ) -> Vec<RenderPhone> {
        if notes.iter().any(|note| note.voicebank_pitch.is_some()) {
            let lookup_notes: Vec<UNote> = notes
                .iter()
                .map(|note| {
                    let mut lookup = note.clone();
                    if let Some(pitch) = &note.voicebank_pitch {
                        lookup.pitch = pitch.clone();
                    }
                    lookup.voicebank_pitch = None;
                    lookup
                })
                .collect();
            let mut phones = Self::apply_legacy_dispatch_with_cvvc_port(
                &lookup_notes,
                vb,
                mode,
                allow_cvvc_port,
            );
            for phone in &mut phones {
                if let Some(note) = notes.get(phone.note_index) {
                    phone.pitch = note.pitch.clone();
                }
            }
            return phones;
        }
        if notes.iter().any(|note| note.timbre.is_some()) {
            // Build the phoneme topology once, without a selected PrefixMap
            // color. Resolving the complete phrase once prevents each color
            // pass from producing a subtly different VCV/CVVC structure.
            let mut neutral = notes.to_vec();
            for note in &mut neutral {
                note.timbre = None;
            }
            let mut topology_bank = vb.clone();
            // Use one deterministic neutral color only to choose the
            // phoneme topology. The selected suffix is removed below before
            // resolving each phone against its effective color bank.
            let topology_color = topology_bank
                .prefix_map
                .colors()
                .find(|color| color.is_empty())
                .or_else(|| {
                    topology_bank
                        .prefix_map
                        .colors()
                        .find(|color| !color.is_empty())
                })
                .map(str::to_owned)
                .unwrap_or_default();
            topology_bank.prefix_map.select_color(&topology_color);
            let topology =
                Self::apply_legacy_dispatch_with_cvvc_port(&neutral, &topology_bank, mode, false);

            let mut colors = std::collections::BTreeSet::new();
            colors.extend(notes.iter().map(|note| note.timbre.clone()));
            let mut mixed = Vec::new();
            for color in colors {
                let mut bank = vb.clone();
                if let Some(color) = &color {
                    bank.prefix_map.select_color(color);
                }
                for mut phone in topology.iter().cloned() {
                    let source_note = notes.get(phone.note_index);
                    // Japanese CVVC places the outgoing VC fragment at the
                    // end of the current note while it actually belongs to
                    // the following note's transition. Resolve that fragment
                    // with the destination color, just like OpenUtau.
                    let transition_target = source_note.and_then(|note| {
                        let note_end = note.position_ms + note.duration_ms;
                        let phone_end = phone.position_ms + phone.duration_ms;
                        if phone.duration_ms + 0.001 < note.duration_ms
                            && (phone_end - note_end).abs() <= 0.01
                        {
                            notes.get(phone.note_index + 1)
                        } else {
                            None
                        }
                    });
                    let effective_note = transition_target.or(source_note);
                    if effective_note.and_then(|note| note.timbre.as_ref()) != color.as_ref() {
                        continue;
                    }
                    // Freeze the mapped alias, including manual phonemizers,
                    // before the renderer sees the shared default voicebank.
                    let lookup_lyric = topology_bank
                        .prefix_map
                        .get_prefix_suffix(&phone.pitch)
                        .map(|(prefix, suffix)| {
                            phone
                                .lyric
                                .strip_prefix(prefix)
                                .unwrap_or(&phone.lyric)
                                .strip_suffix(suffix)
                                .unwrap_or_else(|| {
                                    phone
                                        .lyric
                                        .strip_prefix(prefix.trim())
                                        .unwrap_or(&phone.lyric)
                                })
                                .trim()
                                .to_string()
                        })
                        .unwrap_or_else(|| phone.lyric.clone());
                    if let Some(entry) = bank
                        .find_mapped_entry(&lookup_lyric, &phone.pitch)
                        .or_else(|| bank.find_exact_entry(&phone.lyric))
                    {
                        phone.lyric = entry.alias.clone();
                        phone.resolved_alias = Some(entry.alias.clone());
                        phone.resolved_timbre = transition_target
                            .and_then(|note| note.timbre.clone())
                            .or_else(|| color.clone());
                    }
                    mixed.push(phone);
                }
            }
            mixed.sort_by(|a, b| a.position_ms.total_cmp(&b.position_ms));
            return mixed;
        }
        if notes.iter().any(|note| note.phonemizer_override.is_some()) {
            let mut mixed = Vec::new();
            for (index, note) in notes.iter().enumerate() {
                let note_mode = note
                    .phonemizer_override
                    .as_deref()
                    .and_then(PhonemizerMode::from_name)
                    .unwrap_or(mode);
                // A phonemizer override changes the algorithm for this note, not
                // its musical neighbourhood. Keep the adjacent notes in the
                // request so VCCV/CVVC phonemizers can still see the outgoing
                // vowel, next consonant, pitch and phrase boundary.
                let window_start = index.saturating_sub(1);
                let window_end = (index + 2).min(notes.len());
                let center = index - window_start;
                let mut context = notes[window_start..window_end].to_vec();
                for context_note in &mut context {
                    context_note.phonemizer_override = None;
                }
                for mut phone in Self::apply_legacy_dispatch(&context, vb, note_mode)
                    .into_iter()
                    .filter(|phone| phone.note_index == center)
                {
                    phone.note_index = index;
                    mixed.push(phone);
                }
            }
            if !mixed.is_empty() {
                return mixed;
            }
        }
        let mut normalized_notes: Vec<(usize, UNote)> = Vec::new();
        for (orig_idx, note) in notes.iter().enumerate() {
            let lyric_trimmed = note.lyric.trim();
            let is_plus = lyric_trimmed.starts_with('+');

            if is_plus {
                if let Some((_last_orig_idx, last_note)) = normalized_notes.last_mut() {
                    let last_end = last_note.position_ms + last_note.duration_ms;
                    if (note.position_ms - last_end).abs() <= 2.0 {
                        last_note.duration_ms += note.duration_ms;
                        continue;
                    } else {
                        let mut fallback_note = note.clone();
                        fallback_note.lyric = last_note.lyric.clone();
                        normalized_notes.push((orig_idx, fallback_note));
                        continue;
                    }
                }
            }

            // Intra-note multi-phoneme split (ex: "ko.e.s", "ko;e;s", "ko,e,s")
            if mode != PhonemizerMode::None
                && (lyric_trimmed.contains('.')
                    || lyric_trimmed.contains(';')
                    || lyric_trimmed.contains(','))
            {
                let parts: Vec<&str> = lyric_trimmed
                    .split(['.', ';', ','])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();

                if parts.len() > 1 {
                    let num_parts = parts.len();
                    let durations = if note.phoneme_durations_ms.len() == num_parts {
                        note.resolved_phoneme_durations(num_parts)
                    } else {
                        let last_part = parts[num_parts - 1];
                        let is_last_coda = last_part.len() <= 2
                            && ![
                                "a", "i", "u", "e", "o", "n", "nn", "あ", "い", "う", "え", "お",
                                "ん",
                            ]
                            .contains(&last_part);

                        let coda_dur = if is_last_coda {
                            (note.duration_ms * 0.25).clamp(40.0, 90.0)
                        } else {
                            note.duration_ms / num_parts as f64
                        };

                        let remaining_dur = (note.duration_ms
                            - if is_last_coda { coda_dur } else { 0.0 })
                        .max(10.0);
                        let prior_count = if is_last_coda {
                            num_parts - 1
                        } else {
                            num_parts
                        };
                        let per_prior = remaining_dur / prior_count.max(1) as f64;

                        let mut res = vec![per_prior; num_parts];
                        if is_last_coda {
                            res[num_parts - 1] = coda_dur;
                        }
                        res
                    };

                    let mut sub_pos = note.position_ms;
                    for (i, part) in parts.into_iter().enumerate() {
                        let mut sub_note = note.clone();
                        sub_note.lyric = part.to_string();
                        sub_note.position_ms = sub_pos;
                        sub_note.duration_ms = durations[i];
                        normalized_notes.push((orig_idx, sub_note));
                        sub_pos += durations[i];
                    }
                    continue;
                }
            }

            normalized_notes.push((orig_idx, note.clone()));
        }

        let temp_notes: Vec<UNote> = normalized_notes.iter().map(|(_, n)| n.clone()).collect();
        let orig_indices: Vec<usize> = normalized_notes.iter().map(|(idx, _)| *idx).collect();

        let mut phones = match mode {
            PhonemizerMode::None => Self::apply_raw_passthrough(&temp_notes, vb),
            PhonemizerMode::PortugueseBrapaVCCV | PhonemizerMode::PortugueseBrapaCVC => {
                // BRAPA is an explicit phonetic family, not a word G2P. The
                // existing port implements its transition cascade and keeps
                // authored aliases when a bank does not contain a candidate.
                brapa::VccvBrapaPhonemizer::apply_phonemizer(&temp_notes, vb)
            }
            PhonemizerMode::JapanesePresamp => {
                CoreJapanesePhonemizer::apply_presamp(&temp_notes, vb)
            }
            PhonemizerMode::EnglishArpasingPlus => {
                Self::apply_english(&temp_notes, vb, PhonemizerMode::EnglishArpasingPlus)
            }
            PhonemizerMode::EnglishXSampa | PhonemizerMode::EnglishCpV => {
                // These families are explicit symbol sets. Preserve a USTX
                // phonetic hint verbatim instead of sending it through the
                // English word fallback, while retaining the shared
                // phrase/VC timing contract.
                Self::apply_english(&temp_notes, vb, mode)
            }
            PhonemizerMode::KoreanCV
            | PhonemizerMode::KoreanCVC
            | PhonemizerMode::KoreanVCV
            | PhonemizerMode::KoreanCVVC
            | PhonemizerMode::KoreanCVCCV
            | PhonemizerMode::KoreanCBNN
            | PhonemizerMode::KoreanCVVCStandard => korean::apply(&temp_notes, vb, mode),
            PhonemizerMode::EnglishVCCV => {
                let mut expanded = temp_notes.clone();
                for note in &mut expanded {
                    let lyric = note.lyric.trim();
                    if lyric.is_empty()
                        || lyric == "+"
                        || lyric.eq_ignore_ascii_case("r")
                        || lyric.chars().any(char::is_whitespace)
                        || vccv::is_explicit_symbol(lyric)
                        || note.phonetic_hint.is_some()
                    {
                        continue;
                    }
                    if let Some(tokens) =
                        Self::openutau_dictionary_tokens(lyric, PhonemizerMode::EnglishG2P)
                    {
                        note.lyric = tokens.join(" ");
                    }
                }
                vccv::apply(&expanded, vb)
            }
            PhonemizerMode::EnglishArpasing => Self::apply_arpasing(&temp_notes, vb),
            PhonemizerMode::EnglishG2P => Self::apply_english(&temp_notes, vb, mode),
            PhonemizerMode::PortugueseCVVC | PhonemizerMode::PortugueseVCV => {
                Self::apply_portuguese(&temp_notes, vb, mode)
            }
            PhonemizerMode::PortugueseBrazilianCVC => portuguese_cvc::apply(&temp_notes, vb),
            PhonemizerMode::RussianCVC => russian::apply(&temp_notes, vb),
            PhonemizerMode::RussianVCCV => russian::apply_vccv(&temp_notes, vb),
            PhonemizerMode::SpanishSyllable => {
                spanish::apply(&temp_notes, vb, PhonemizerMode::SpanishSyllable)
            }
            PhonemizerMode::SpanishMakkusan | PhonemizerMode::SpanishVCCV => {
                spanish::apply(&temp_notes, vb, mode)
            }
            PhonemizerMode::FrenchCMUSphinx => diphone::apply(&temp_notes, vb, mode),
            PhonemizerMode::FrenchCVVC | PhonemizerMode::FrenchVCCV => {
                french::apply(&temp_notes, vb, mode)
            }
            PhonemizerMode::ChineseCVV
            | PhonemizerMode::ChineseCVVC
            | PhonemizerMode::ChineseVCV
            | PhonemizerMode::CantoneseCVVC => chinese::apply(&temp_notes, vb, mode),
            PhonemizerMode::CantoneseSyo => chinese::apply_cantonese_syo(&temp_notes, vb),
            PhonemizerMode::ChineseCVVPlus => chinese::apply_cvv_plus(&temp_notes, vb),
            PhonemizerMode::PortugueseG2P => Self::apply_rule_g2p(&temp_notes, vb, mode),
            PhonemizerMode::FrenchG2P
            | PhonemizerMode::GermanG2P
            | PhonemizerMode::GermanVCCV
            | PhonemizerMode::GermanDiphone
            | PhonemizerMode::RussianG2P
            | PhonemizerMode::SpanishG2P
            | PhonemizerMode::TurkishG2P
            | PhonemizerMode::ChineseG2P
            | PhonemizerMode::CantoneseG2P
            | PhonemizerMode::TurkishCVVC
            | PhonemizerMode::ArabicG2P
            | PhonemizerMode::IcelandicG2P => Self::apply_rule_g2p(&temp_notes, vb, mode),
            PhonemizerMode::BasicCV => Self::apply_openutau_cv(&temp_notes, vb),
            PhonemizerMode::VCV => Self::apply_openutau_vcv(&temp_notes, vb),
            PhonemizerMode::CVVC if allow_cvvc_port => Self::apply_openutau_cvvc(&temp_notes, vb),
            _ => Self::apply_japanese(&temp_notes, vb, mode),
        };

        for p in &mut phones {
            if let Some(&real_idx) = orig_indices.get(p.note_index) {
                p.note_index = real_idx;
            }
        }

        for (orig_idx, note) in notes.iter().enumerate() {
            if !note.phoneme_durations_ms.is_empty() {
                let note_phone_indices: Vec<usize> = phones
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.note_index == orig_idx)
                    .map(|(idx, _)| idx)
                    .collect();

                if note_phone_indices.len() == note.phoneme_durations_ms.len() {
                    let initial_offset = phones[note_phone_indices[0]].position_ms;
                    let mut cur_pos = initial_offset;
                    for (i, &phone_idx) in note_phone_indices.iter().enumerate() {
                        let dur = note.phoneme_durations_ms[i];
                        phones[phone_idx].position_ms = cur_pos;
                        phones[phone_idx].duration_ms = dur;
                        cur_pos += dur;
                    }
                }
            }

            if !note.phoneme_overrides.is_empty() {
                let note_phone_indices: Vec<usize> = phones
                    .iter()
                    .enumerate()
                    .filter(|(_, phone)| phone.note_index == orig_idx)
                    .map(|(phone_idx, _)| phone_idx)
                    .collect();
                for phoneme_override in &note.phoneme_overrides {
                    let Some(&phone_idx) = note_phone_indices.get(phoneme_override.index) else {
                        continue;
                    };
                    let phone = &mut phones[phone_idx];
                    if let Some(alias) = phoneme_override
                        .phoneme
                        .as_ref()
                        .filter(|alias| !alias.trim().is_empty())
                    {
                        phone.lyric = alias.clone();
                    }
                    if let Some(alternate) = phoneme_override.alternate {
                        let alternate_alias = format!("{}{}", phone.lyric, alternate);
                        if let Some(entry) = vb.find_mapped_entry(&alternate_alias, &phone.pitch) {
                            phone.lyric = entry.alias.clone();
                        }
                    }
                    if let Some(flags) = phoneme_override
                        .flags
                        .as_deref()
                        .filter(|flags| !flags.trim().is_empty())
                    {
                        phone.flags = crate::drivers::resampler_driver::merge_utau_flags(&[
                            phone.flags.as_str(),
                            flags,
                        ]);
                    }
                    if let Some(offset) = phoneme_override.offset_ms.filter(|v| v.is_finite()) {
                        phone.position_ms += offset;
                    }
                    if let Some(delta) = phoneme_override
                        .preutter_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.preutter_offset_ms += delta;
                    }
                    if let Some(delta) = phoneme_override
                        .overlap_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.overlap_offset_ms += delta;
                    }
                    if let Some(delta) = phoneme_override
                        .attack_time_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.attack_time_delta_ms += delta;
                    }
                    if let Some(delta) = phoneme_override
                        .release_time_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.release_time_delta_ms += delta;
                    }
                    if let Some(value) = phoneme_override
                        .consonant_timing_offset_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.consonant_timing_offset_ms = value;
                    }
                    if let Some(value) = phoneme_override.velocity {
                        phone.expressions.velocity = value;
                        phone.expressions.consonant_velocity = value;
                    }
                    if let Some(value) = phoneme_override.volume {
                        phone.expressions.volume = value;
                    }
                    if let Some(value) = phoneme_override.attack {
                        phone.expressions.attack = value;
                    }
                    if let Some(value) = phoneme_override.decay {
                        phone.expressions.decay = value;
                    }
                    if let Some(value) = phoneme_override.breathiness {
                        phone.expressions.breathiness = value;
                    }
                    if let Some(value) = phoneme_override.gender {
                        phone.expressions.gender = value;
                    }
                    if let Some(value) = phoneme_override.modulation {
                        phone.expressions.modulation = value;
                    }
                    if let Some(value) = phoneme_override.pitch_delta {
                        phone.expressions.pitch_delta = value;
                    }
                    if let Some(value) = phoneme_override.dynamics {
                        phone.expressions.dynamics = value;
                    }
                    if let Some(envelope) = &phoneme_override.envelope {
                        phone.envelope = envelope.clone();
                    }
                    if let Some(vibrato) = &phoneme_override.vibrato {
                        phone.vibrato = vibrato.clone();
                    }
                    if let Some(pitch_bend) = &phoneme_override.pitch_bend {
                        phone.pitch_bend = pitch_bend.clone();
                    }
                }
            }
        }

        if notes.iter().any(|note| !note.phoneme_overrides.is_empty()) {
            phones.sort_by(|left, right| left.position_ms.total_cmp(&right.position_ms));
            for index in 0..phones.len().saturating_sub(1) {
                let current_note = phones[index].note_index;
                let next_note = phones[index + 1].note_index;
                let same_phrase = current_note == next_note
                    || notes
                        .get(current_note)
                        .zip(notes.get(next_note))
                        .is_some_and(|(current, next)| {
                            (current.position_ms + current.duration_ms - next.position_ms).abs()
                                <= 0.001
                        });
                let duration = phones[index + 1].position_ms - phones[index].position_ms;
                if same_phrase && duration > 0.0 {
                    phones[index].duration_ms = duration;
                }
            }
        }

        phones
    }

    fn apply_raw_passthrough(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut global_prev_vowel: Option<String> = None;
        for (i, note) in notes.iter().enumerate() {
            let raw_lyric = note.lyric.trim();
            if raw_lyric.is_empty() || raw_lyric == "R" || raw_lyric == "r" || raw_lyric == "+" {
                continue;
            }

            let is_multi = raw_lyric.contains(['.', ';', ',', '|', '/']);

            if is_multi {
                let parts: Vec<&str> = raw_lyric
                    .split(['.', ';', ',', '|', '/'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "+" && *s != "R" && *s != "r")
                    .collect();

                if !parts.is_empty() {
                    let num_parts = parts.len();
                    let durations = if note.phoneme_durations_ms.len() == num_parts {
                        note.resolved_phoneme_durations(num_parts)
                    } else {
                        // Manual aliases describe phoneme positions, not equal slices of a
                        // musical note. Resolve every following alias from its oto.ini
                        // preutterance, backwards from the note end. This is the same
                        // invariant used by OpenUtau phonemizers and prevents a short VC
                        // bridge from receiving half of a long sustained note.
                        let mut resolved = vec![0.0; num_parts];
                        let mut remaining = note.duration_ms.max(0.0);
                        for part_idx in (1..num_parts).rev() {
                            let alias = parts[part_idx];
                            let stretch =
                                consonant_velocity_time_scale(note.expressions.consonant_velocity);
                            let requested = vb
                                .find_mapped_entry(alias, &note.pitch)
                                .map(|oto| {
                                    let preutter = oto.preutterance.max(0.0) * stretch;
                                    let overlap = oto.overlap * stretch;
                                    if overlap < 0.0 {
                                        preutter - overlap
                                    } else {
                                        preutter
                                    }
                                })
                                .unwrap_or(60.0);
                            let duration = requested.max(5.0).min((remaining * 0.5).max(5.0));
                            resolved[part_idx] = duration;
                            remaining = (remaining - duration).max(0.0);
                        }
                        resolved[0] = remaining;
                        resolved
                    };
                    let mut sub_position_ms = note.position_ms;
                    let mut prev_vowel: Option<String> = None;
                    for (part_idx, part) in parts.into_iter().enumerate() {
                        let sub_duration_ms = durations[part_idx];
                        let mut sub_envelope = note.envelope.clone();
                        // Smooth crossfade transitions between intra-note phonemes:
                        // Preserve user-authored attack on the first phone and release on the last phone,
                        // while ensuring interior junctions have seamless crossfade boundaries.
                        if part_idx > 0 {
                            sub_envelope.p1 = 0.0;
                            sub_envelope.p2 = 5.0;
                            sub_envelope.v1 = 0.0;
                            sub_envelope.v2 = 100.0;
                        }
                        if part_idx < num_parts - 1 {
                            sub_envelope.p4 = 0.0;
                            sub_envelope.p5 = 35.0;
                            sub_envelope.v4 = 100.0;
                            sub_envelope.v5 = 0.0;
                        }

                        // Resolver contra o voicebank e encadear transições VCV se existirem
                        let mut resolved_alias = None;
                        let mut coda_trunc = false;
                        let mut phone_lyric = part.to_string();

                        // 1. Tentar VCV se houver vogal anterior
                        if let Some(pv) = prev_vowel.as_deref() {
                            let vcv_cands = [
                                format!("{pv} {part}"),
                                format!("{pv}_{part}"),
                                format!("* {part}"),
                                part.to_string(),
                            ];
                            for cand in &vcv_cands {
                                for spelling in crate::phonemizer::romaji::lyric_candidates(cand) {
                                    if let Some(entry) = vb.find_mapped_entry(&spelling, &note.pitch) {
                                        resolved_alias = Some(entry.alias.clone());
                                        phone_lyric = entry.alias.clone();
                                        break;
                                    }
                                }
                                if resolved_alias.is_some() {
                                    break;
                                }
                            }
                        }

                        // 2. Tentar candidatos diretos ou romaji/kana
                        if resolved_alias.is_none() {
                            for cand in [part, &format!("- {part}")] {
                                for spelling in crate::phonemizer::romaji::lyric_candidates(cand) {
                                    if let Some(entry) = vb.find_mapped_entry(&spelling, &note.pitch) {
                                        resolved_alias = Some(entry.alias.clone());
                                        phone_lyric = entry.alias.clone();
                                        break;
                                    }
                                }
                                if resolved_alias.is_some() {
                                    break;
                                }
                            }
                        }

                        // 3. Fallback inteligente de coda com corte de vogal posterior (coda truncation)
                        if resolved_alias.is_none() {
                            let pv = prev_vowel.as_deref().unwrap_or("");
                            if let Some((entry, is_trunc)) = Self::find_coda_fallback_entry(vb, pv, part, &note.pitch) {
                                resolved_alias = Some(entry.alias.clone());
                                coda_trunc = is_trunc;
                                phone_lyric = if is_trunc && !pv.is_empty() {
                                    format!("{pv} {part}")
                                } else {
                                    entry.alias.clone()
                                };
                            }
                        }

                        prev_vowel = Self::extract_vowel(part).map(str::to_owned);

                        phones.push(RenderPhone {
                            note_index: i,
                            lyric: phone_lyric,
                            pitch: note.pitch.clone(),
                            position_ms: sub_position_ms,
                            duration_ms: sub_duration_ms,
                            envelope: sub_envelope,
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                            resolved_alias,
                            resolved_timbre: note.timbre.clone(),
                            coda_truncation: coda_trunc,
                        });
                        sub_position_ms += sub_duration_ms;
                    }
                    continue;
                }
            }

            let mut resolved_alias = None;
            let mut coda_trunc = false;
            let mut phone_lyric = raw_lyric.to_string();

            // 1. Tentar VCV se houver vogal anterior
            if let Some(pv) = global_prev_vowel.as_deref() {
                let vcv_cands = [
                    format!("{pv} {raw_lyric}"),
                    format!("{pv}_{raw_lyric}"),
                    format!("* {raw_lyric}"),
                ];
                for cand in &vcv_cands {
                    for spelling in crate::phonemizer::romaji::lyric_candidates(cand) {
                        if let Some(entry) = vb.find_mapped_entry(&spelling, &note.pitch) {
                            resolved_alias = Some(entry.alias.clone());
                            phone_lyric = entry.alias.clone();
                            break;
                        }
                    }
                    if resolved_alias.is_some() {
                        break;
                    }
                }
            }

            // 2. Tentar candidatos diretos ou romaji/kana (ex: "- こ" ou "こ" para "ko")
            if resolved_alias.is_none() {
                for cand in [raw_lyric, &format!("- {raw_lyric}")] {
                    for spelling in crate::phonemizer::romaji::lyric_candidates(cand) {
                        if let Some(entry) = vb.find_mapped_entry(&spelling, &note.pitch) {
                            resolved_alias = Some(entry.alias.clone());
                            phone_lyric = entry.alias.clone();
                            break;
                        }
                    }
                    if resolved_alias.is_some() {
                        break;
                    }
                }
            }

            // 3. Fallback inteligente de Coda com corte de vogal posterior
            if resolved_alias.is_none() {
                let pv = global_prev_vowel.as_deref().unwrap_or("");
                if let Some((entry, is_trunc)) = Self::find_coda_fallback_entry(vb, pv, raw_lyric, &note.pitch) {
                    resolved_alias = Some(entry.alias.clone());
                    coda_trunc = is_trunc;
                    phone_lyric = if is_trunc && !pv.is_empty() {
                        format!("{pv} {raw_lyric}")
                    } else {
                        entry.alias.clone()
                    };
                }
            }

            global_prev_vowel = Self::extract_vowel(raw_lyric).map(str::to_owned);

            phones.push(RenderPhone {
                note_index: i,
                lyric: phone_lyric,
                pitch: note.pitch.clone(),
                position_ms: note.position_ms,
                duration_ms: note.duration_ms,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
                resolved_alias,
                resolved_timbre: note.timbre.clone(),
                coda_truncation: coda_trunc,
            });
        }
        phones
    }

    fn apply_rule_g2p(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        let arabic = mode == PhonemizerMode::ArabicG2P;
        let icelandic = mode == PhonemizerMode::IcelandicG2P;
        let chinese_like = matches!(
            mode,
            PhonemizerMode::ChineseG2P | PhonemizerMode::CantoneseG2P
        );
        let mapping: std::collections::HashMap<String, Vec<String>> =
            serde_json::from_str(if arabic {
                include_str!("data/arabic-mapping.json")
            } else {
                include_str!("data/icelandic-mapping.json")
            })
            .unwrap_or_default();
        let mut phones: Vec<RenderPhone> = Vec::new();
        let cvvc = matches!(
            mode,
            PhonemizerMode::TurkishCVVC
                | PhonemizerMode::ChineseCVVC
                | PhonemizerMode::CantoneseCVVC
                | PhonemizerMode::FrenchCVVC
                | PhonemizerMode::FrenchVCCV
                | PhonemizerMode::GermanVCCV
                | PhonemizerMode::GermanDiphone
                | PhonemizerMode::RussianVCCV
                | PhonemizerMode::SpanishVCCV
        );
        let mut previous_vowel: Option<String> = None;
        for (note_index, note) in notes.iter().enumerate() {
            let word = note.lyric.trim();
            if word.is_empty() || word.eq_ignore_ascii_case("r") {
                previous_vowel = None;
                continue;
            }
            let tokens = if arabic {
                Self::arabic_tokens(word)
            } else if icelandic {
                Self::icelandic_tokens(word)
            } else if chinese_like {
                Self::latin_language_tokens(word, mode)
            } else {
                Self::openutau_dictionary_tokens(word, mode).unwrap_or_else(|| {
                    crate::terminal_log::debug(
                        "Phonemizer",
                        format!("mode={mode:?} word={word:?} fallback=passthrough reason=missing_dictionary_entry"),
                    );
                    // An unknown word is not converted by a Kamafeu-specific
                    // grapheme table. Keep the user's token intact until an
                    // official OpenUtau G2P dictionary/provider resolves it.
                    EnglishPhonemizer::phonetic_tokens(word)
                })
            };
            if tokens.is_empty() {
                continue;
            }
            let dur = note.duration_ms / tokens.len() as f64;
            for (index, token) in tokens.iter().enumerate() {
                if cvvc && index == 0 {
                    if let Some(vowel) = previous_vowel.as_deref() {
                        if !Self::is_rule_vowel(token) {
                            let vc = format!("{} {}", vowel, token);
                            if let Some(entry) = vb.find_mapped_entry(&vc, &note.pitch) {
                                let transition_duration =
                                    dur.min((note.duration_ms * 0.35).max(20.0));
                                phones.push(RenderPhone {
                                    note_index,
                                    lyric: entry.alias.clone(),
                                    pitch: note.pitch.clone(),
                                    position_ms: note.position_ms - transition_duration,
                                    duration_ms: transition_duration,
                                    envelope: note.envelope.clone(),
                                    expressions: note.expressions.clone(),
                                    pitch_bend: note.pitch_bend.clone(),
                                    vibrato: note.vibrato.clone(),
                                    flags: note.flags.clone(),
                                    resolved_alias: None,
                                    resolved_timbre: None,
                                    coda_truncation: false,
                                });
                            }
                        }
                    }
                }
                let aliases = if arabic || icelandic {
                    mapping
                        .get(token)
                        .cloned()
                        .unwrap_or_else(|| vec![token.clone()])
                } else {
                    vec![token.clone()]
                };
                let mut alias = aliases.first().cloned().unwrap_or_else(|| token.clone());
                if let Some(entry) = vb.find_mapped_entry(&alias, &note.pitch) {
                    alias = entry.alias.clone();
                }
                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: note.position_ms + index as f64 * dur,
                    duration_ms: dur,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                    resolved_alias: None,
                    resolved_timbre: None,
                    coda_truncation: false,
                });
            }
            if cvvc {
                previous_vowel = tokens
                    .iter()
                    .rev()
                    .find(|token| Self::is_rule_vowel(token))
                    .cloned();
            }
        }
        phones
    }

    fn is_rule_vowel(token: &str) -> bool {
        token.chars().next().is_some_and(|c| {
            matches!(
                c,
                'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'A' | 'E' | 'I' | 'O' | 'U' | 'Y'
            )
        })
    }

    fn arabic_tokens(word: &str) -> Vec<String> {
        let clean: String = word
            .trim()
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
            .collect();
        let chars: Vec<char> = clean.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let token = {
                let c = chars[i];
                i += 1;
                match c {
                    'ا' | 'أ' | 'إ' | 'آ' => "a:",
                    'ب' | 'b' => "b",
                    'ت' | 't' => "t",
                    'ث' => "th",
                    'ج' | 'j' => "j",
                    'ح' | '7' => "H",
                    'خ' => "kh",
                    'د' | 'd' => "d",
                    'ذ' => "dh",
                    'ر' | 'r' => "r",
                    'ز' | 'z' => "z",
                    'س' | 's' => "s",
                    'ش' => "sh",
                    'ص' | '9' => "s.",
                    'ض' => "d.",
                    'ط' => "t.",
                    'ظ' => "dh.",
                    'ع' | '3' => "3",
                    'غ' => "gh",
                    'ف' | 'f' => "f",
                    'ق' | 'q' => "q",
                    'ك' | 'k' => "k",
                    'ل' | 'l' => "l",
                    'م' | 'm' => "m",
                    'ن' | 'n' => "n",
                    'ه' | 'h' => "h",
                    'و' | 'w' => "w",
                    'ي' | 'ى' | 'y' => "y",
                    'ة' => "a",
                    'ء' | '2' => "?",
                    _ => continue,
                }
            };
            out.push(token.to_string());
        }
        out
    }

    fn icelandic_tokens(word: &str) -> Vec<String> {
        let lower = word.to_lowercase();
        let chars: Vec<char> = lower.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            let token = match (c, next) {
                ('a', Some('u')) => {
                    i += 2;
                    "au"
                }
                ('e', Some('i')) => {
                    i += 2;
                    "ei"
                }
                ('o', Some('u')) => {
                    i += 2;
                    "ou"
                }
                ('p', Some('h')) => {
                    i += 2;
                    "p_h"
                }
                ('t', Some('h')) => {
                    i += 2;
                    "t_h"
                }
                ('k', Some('h')) => {
                    i += 2;
                    "k_h"
                }
                ('þ', _) => {
                    i += 1;
                    "T"
                }
                ('ð', _) => {
                    i += 1;
                    "D"
                }
                ('æ', _) => {
                    i += 1;
                    "E"
                }
                ('ö', _) => {
                    i += 1;
                    "9"
                }
                ('á', _) => {
                    i += 1;
                    "ai"
                }
                ('é', _) => {
                    i += 1;
                    "ei"
                }
                ('í', _) => {
                    i += 1;
                    "i"
                }
                ('ó', _) => {
                    i += 1;
                    "ou"
                }
                ('ú', _) => {
                    i += 1;
                    "u"
                }
                _ => {
                    i += 1;
                    match c {
                        'a' | 'e' | 'i' | 'o' | 'u' => &lower[i - 1..i],
                        _ => match c {
                            'p' | 'b' => "p",
                            't' | 'd' => "t",
                            'k' | 'g' | 'c' => "k",
                            'f' | 'v' => "f",
                            's' => "s",
                            'j' => "j",
                            'h' => "h",
                            'm' => "m",
                            'n' => "n",
                            'l' => "l",
                            'r' => "r",
                            _ => continue,
                        },
                    }
                }
            };
            out.push(token.to_string());
        }
        out
    }

    fn latin_language_tokens(word: &str, mode: PhonemizerMode) -> Vec<String> {
        let lower = word.to_lowercase();
        if matches!(
            mode,
            PhonemizerMode::ChineseG2P
                | PhonemizerMode::ChineseCVV
                | PhonemizerMode::ChineseCVVPlus
                | PhonemizerMode::ChineseVCV
                | PhonemizerMode::CantoneseG2P
                | PhonemizerMode::CantoneseSyo
                | PhonemizerMode::ChineseCVVC
                | PhonemizerMode::CantoneseCVVC
        ) && lower
            .chars()
            .any(|c| !c.is_ascii() && !c.is_ascii_punctuation())
        {
            let romanized = if matches!(
                mode,
                PhonemizerMode::ChineseG2P
                    | PhonemizerMode::ChineseCVV
                    | PhonemizerMode::ChineseCVVPlus
                    | PhonemizerMode::ChineseVCV
                    | PhonemizerMode::ChineseCVVC
            ) {
                pinyin::to_pinyin_vec(&lower, pinyin::Pinyin::with_tone_num_end)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            } else {
                Self::cantonese_tokens(&lower)
            };
            if !romanized.is_empty() {
                return romanized;
            }
        }
        let mut out = Vec::new();
        let chars: Vec<char> = lower.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_whitespace() || chars[i].is_ascii_punctuation() {
                i += 1;
                continue;
            }
            let pair = chars
                .get(i..(i + 2).min(chars.len()))
                .unwrap_or(&[])
                .iter()
                .collect::<String>();
            let token = match mode {
                PhonemizerMode::FrenchG2P => match pair.as_str() {
                    "ch" => {
                        i += 2;
                        "sh"
                    }
                    "gn" => {
                        i += 2;
                        "ny"
                    }
                    "ou" => {
                        i += 2;
                        "u"
                    }
                    "oi" => {
                        i += 2;
                        "wa"
                    }
                    _ => Self::latin_char_token(&chars, &mut i),
                },
                PhonemizerMode::GermanG2P => {
                    if i + 3 <= chars.len() && chars[i..i + 3].iter().collect::<String>() == "sch" {
                        i += 3;
                        "sh"
                    } else {
                        match pair.as_str() {
                            "ch" => {
                                i += 2;
                                "x"
                            }
                            "ei" | "ai" => {
                                i += 2;
                                "ay"
                            }
                            "eu" | "äu" => {
                                i += 2;
                                "oy"
                            }
                            _ => Self::latin_char_token(&chars, &mut i),
                        }
                    }
                }
                PhonemizerMode::RussianG2P => Self::cyrillic_token(&chars, &mut i),
                PhonemizerMode::SpanishG2P => match pair.as_str() {
                    "ll" => {
                        i += 2;
                        "y"
                    }
                    "ch" => {
                        i += 2;
                        "ch"
                    }
                    "rr" => {
                        i += 2;
                        "r"
                    }
                    _ => Self::latin_char_token(&chars, &mut i),
                },
                PhonemizerMode::TurkishG2P | PhonemizerMode::TurkishCVVC => {
                    if matches!(chars[i], 'ş' | 'ç' | 'ğ') {
                        let c = chars[i];
                        i += 1;
                        match c {
                            'ş' => "sh",
                            'ç' => "ch",
                            _ => "g",
                        }
                    } else {
                        match pair.as_str() {
                            "ae" | "eu" | "oe" | "ue" => {
                                i += 2;
                                match pair.as_str() {
                                    "ae" => "ae",
                                    "eu" => "eu",
                                    "oe" => "oe",
                                    _ => "ue",
                                }
                            }
                            "ch" => {
                                i += 2;
                                "ch"
                            }
                            "sh" => {
                                i += 2;
                                "sh"
                            }
                            "ng" => {
                                i += 2;
                                "ng"
                            }
                            "rr" => {
                                i += 2;
                                "rr"
                            }
                            _ => Self::latin_char_token(&chars, &mut i),
                        }
                    }
                }
                PhonemizerMode::ChineseG2P
                | PhonemizerMode::CantoneseG2P
                | PhonemizerMode::ChineseCVVC
                | PhonemizerMode::CantoneseCVVC => {
                    // OpenUtau's Chinese/Cantonese phonemizers consume pinyin or
                    // jyutping syllables. Preserve an already romanized syllable
                    // (including its tone number) as one phoneme unit.
                    if chars[i].is_ascii_alphabetic() {
                        let start = i;
                        while i < chars.len()
                            && (chars[i].is_ascii_alphabetic() || chars[i].is_ascii_digit())
                        {
                            i += 1;
                        }
                        let syllable: String = chars[start..i].iter().collect();
                        out.push(syllable);
                        continue;
                    }
                    i += 1;
                    match chars[i - 1] {
                        '你' => out.push(
                            if matches!(
                                mode,
                                PhonemizerMode::ChineseG2P | PhonemizerMode::ChineseCVVC
                            ) {
                                "ni3".to_string()
                            } else {
                                "nei5".to_string()
                            },
                        ),
                        '好' => out.push(
                            if matches!(
                                mode,
                                PhonemizerMode::ChineseG2P | PhonemizerMode::ChineseCVVC
                            ) {
                                "hao3".to_string()
                            } else {
                                "hou2".to_string()
                            },
                        ),
                        '世' => out.push("shi4".to_string()),
                        '界' => out.push(
                            if matches!(
                                mode,
                                PhonemizerMode::ChineseG2P | PhonemizerMode::ChineseCVVC
                            ) {
                                "jie4".to_string()
                            } else {
                                "gaai3".to_string()
                            },
                        ),
                        _ => out.push("a".to_string()),
                    }
                    continue;
                }
                _ => Self::latin_char_token(&chars, &mut i),
            };
            out.push(token.to_string());
        }
        out
    }

    fn cantonese_tokens(word: &str) -> Vec<String> {
        let dictionary = CANTONESE_DICTIONARY.get_or_init(|| {
            let mut map = HashMap::new();
            for line in include_str!("data/cccedict-canto-readings.txt").lines() {
                if line.starts_with('#') {
                    continue;
                }
                let Some((head, reading)) = line.split_once('{') else {
                    continue;
                };
                let Some(key) = head.split_whitespace().next() else {
                    continue;
                };
                let Some(reading) = reading.split('}').next() else {
                    continue;
                };
                let syllables: Vec<String> =
                    reading.split_whitespace().map(str::to_string).collect();
                if !syllables.is_empty() {
                    map.entry(key.to_string()).or_insert(syllables);
                }
            }
            map
        });
        if let Some(syllables) = dictionary.get(word) {
            return syllables.clone();
        }
        let mut out = Vec::new();
        for c in word.chars() {
            if let Some(syllables) = dictionary.get(&c.to_string()) {
                out.extend(syllables.iter().cloned());
                continue;
            }
            let token = match c {
                '你' => "nei5",
                '好' => "hou2",
                '世' => "sai3",
                '界' => "gaai3",
                '我' => "ngo5",
                '係' => "hai6",
                '佢' => "keoi5",
                '人' => "jan4",
                '中' => "zung1",
                '國' | '国' => "gwok3",
                '學' | '学' => "hok6",
                '生' => "saang1",
                '廣' | '广' => "gwong2",
                '東' | '东' => "dung1",
                _ if c.is_ascii_alphabetic() => continue,
                _ => "a1",
            };
            out.push(token.to_string());
        }
        out
    }

    fn openutau_dictionary_tokens(word: &str, mode: PhonemizerMode) -> Option<Vec<String>> {
        let clean = word.trim().to_lowercase();
        let dictionaries = OPENUTAU_G2P_DICTIONARIES.get_or_init(|| {
            let mut all = HashMap::new();
            for (modes, dictionary) in [
                (
                    &[PhonemizerMode::EnglishG2P][..],
                    include_str!("data/openutau/en.dict"),
                ),
                (
                    &[
                        PhonemizerMode::FrenchG2P,
                        PhonemizerMode::FrenchCMUSphinx,
                        PhonemizerMode::FrenchCVVC,
                        PhonemizerMode::FrenchVCCV,
                    ][..],
                    include_str!("data/openutau/fr.dict"),
                ),
                (
                    &[
                        PhonemizerMode::GermanG2P,
                        PhonemizerMode::GermanVCCV,
                        PhonemizerMode::GermanDiphone,
                    ][..],
                    include_str!("data/openutau/de.dict"),
                ),
                (
                    &[
                        PhonemizerMode::RussianG2P,
                        PhonemizerMode::RussianCVC,
                        PhonemizerMode::RussianVCCV,
                    ][..],
                    include_str!("data/openutau/ru.dict"),
                ),
                (
                    &[
                        PhonemizerMode::SpanishG2P,
                        PhonemizerMode::SpanishMakkusan,
                        PhonemizerMode::SpanishSyllable,
                        PhonemizerMode::SpanishVCCV,
                    ][..],
                    include_str!("data/openutau/es.dict"),
                ),
                (
                    &[
                        PhonemizerMode::PortugueseG2P,
                        PhonemizerMode::PortugueseBrazilianCVC,
                    ][..],
                    include_str!("data/openutau/pt.dict"),
                ),
            ] {
                let mut table = HashMap::new();
                for line in dictionary.lines() {
                    let mut fields = line.split_whitespace();
                    let Some(key) = fields.next() else { continue };
                    if key.starts_with(';') || key.starts_with("###") {
                        continue;
                    }
                    let phones = fields
                        .map(|phone| {
                            phone
                                .trim_end_matches(|c: char| c.is_ascii_digit())
                                .to_string()
                        })
                        .collect::<Vec<_>>();
                    if phones.is_empty() {
                        continue;
                    }
                    table.insert(key.to_lowercase(), phones);
                }
                for mode in modes {
                    let normalized = table
                        .iter()
                        .map(|(key, phones)| {
                            let phones = if *mode == PhonemizerMode::EnglishG2P {
                                phones.iter().map(|p| p.to_lowercase()).collect()
                            } else {
                                phones.clone()
                            };
                            (key.clone(), phones)
                        })
                        .collect();
                    all.insert(*mode, normalized);
                }
            }
            all
        });
        dictionaries.get(&mode).and_then(|table| {
            table.get(&clean).cloned().or_else(|| {
                table
                    .iter()
                    .find(|(key, _)| key.starts_with(&format!("{}(", clean)))
                    .map(|(_, phones)| phones.clone())
            })
        })
    }

    /// Loads the singer-local Arpasing dictionary used by OpenUtau's
    /// `ArpasingPhonemizer`.  Arpasing dictionaries are intentionally read
    /// from the voicebank, rather than replaced by a built-in English
    /// grapheme heuristic.  This keeps the phonemizer authoritative for the
    /// singer and makes missing data observable to the caller.
    fn openutau_arpasing_tokens(word: &str, vb: &Voicebank) -> Option<Vec<String>> {
        let key = word.trim().to_lowercase();
        if let Ok(content) = std::fs::read_to_string(vb.root_path.join("arpasing.yaml")) {
            if let Ok(document) = yaml_serde::from_str::<yaml_serde::Value>(&content) {
                let value = document.get(&key).or_else(|| {
                    document.as_mapping().and_then(|mapping| {
                        mapping.iter().find_map(|(candidate, value)| {
                            (candidate.as_str()?.to_lowercase() == key).then_some(value)
                        })
                    })
                });
                let parse_tokens = |value: &yaml_serde::Value| -> Option<Vec<String>> {
                    let mut tokens = match value {
                        yaml_serde::Value::String(value) => value
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<Vec<_>>(),
                        yaml_serde::Value::Sequence(values) => values
                            .iter()
                            .filter_map(|value| value.as_str().map(str::to_owned))
                            .collect::<Vec<_>>(),
                        _ => Vec::new(),
                    };
                    tokens.retain(|token| !token.is_empty());
                    (!tokens.is_empty()).then_some(tokens)
                };
                if let Some(tokens) = value.and_then(parse_tokens) {
                    return Some(tokens);
                }
                if let Some(tokens) = document
                    .get("entries")
                    .and_then(|entries| entries.as_sequence())
                    .and_then(|entries| {
                        entries.iter().find_map(|entry| {
                            let grapheme = entry.get("grapheme")?.as_str()?;
                            (grapheme.to_lowercase() == key)
                                .then(|| entry.get("phonemes"))
                                .flatten()
                                .and_then(parse_tokens)
                        })
                    })
                {
                    return Some(tokens);
                }
            }
        }

        // OpenUtau's Arpasing phonemizer falls back to Arpabet G2P when the
        // singer-local dictionary has no entry. The bundled OpenUtau/CMU
        // dictionary is the deterministic equivalent available to Kamafeu.
        Self::openutau_dictionary_tokens(word, PhonemizerMode::EnglishG2P)
    }

    /// Vowel substitutions declared by OpenUtau's Arpasing phonemizer.
    /// These are alias lookup fallbacks only; the generated phoneme remains
    /// the original Arpabet token for timing and diagnostics.
    fn arpasing_vowel_fallbacks(phone: &str) -> &'static [&'static str] {
        match phone {
            "aa" => &["ah", "ae"],
            "ae" => &["ah", "aa"],
            "ah" => &["aa", "ae"],
            "ao" => &["ow"],
            "ow" => &["ao"],
            "eh" => &["ae"],
            "ih" => &["iy"],
            "iy" => &["ih"],
            "uh" => &["uw"],
            "uw" => &["uh"],
            "aw" => &["ao"],
            _ => &[],
        }
    }

    fn latin_char_token(chars: &[char], i: &mut usize) -> &'static str {
        let c = chars[*i];
        *i += 1;
        match c {
            'a' | 'á' | 'à' | 'â' | 'ä' | 'ã' => "a",
            'e' | 'é' | 'è' | 'ê' | 'ë' => "e",
            'i' | 'í' | 'ì' | 'î' | 'ï' => "i",
            'o' | 'ó' | 'ò' | 'ô' | 'ö' | 'õ' => "o",
            'u' | 'ú' | 'ù' | 'û' | 'ü' => "u",
            'b' => "b",
            'c' => "k",
            'd' => "d",
            'f' => "f",
            'g' => "g",
            'h' => "h",
            'j' => "j",
            'k' => "k",
            'l' => "l",
            'm' => "m",
            'n' | 'ñ' => "n",
            'p' => "p",
            'q' => "k",
            'r' => "r",
            's' => "s",
            't' => "t",
            'v' => "v",
            'w' => "w",
            'x' => "x",
            'y' => "y",
            'z' => "z",
            _ => "a",
        }
    }

    fn cyrillic_token(chars: &[char], i: &mut usize) -> &'static str {
        let c = chars[*i];
        *i += 1;
        match c {
            'а' | 'я' => "a",
            'э' | 'е' | 'ё' => "e",
            'и' | 'й' => "i",
            'о' => "o",
            'у' | 'ю' => "u",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'д' => "d",
            'ж' => "zh",
            'з' => "z",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'ф' => "f",
            'х' => "kh",
            'ц' => "ts",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "sh",
            _ => "a",
        }
    }

    fn apply_japanese(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        CoreJapanesePhonemizer::apply_japanese(notes, vb, mode)
    }

    pub fn find_coda_fallback_entry<'a>(
        vb: &'a Voicebank,
        prev_vowel: &str,
        consonant: &str,
        pitch_name: &str,
    ) -> Option<(&'a crate::oto::OtoEntry, bool)> {
        let cons = consonant.trim();
        let pv = prev_vowel.trim();
        if cons.is_empty() {
            return None;
        }
        let pv_kana = crate::phonemizer::romaji::romaji_to_hiragana(pv);

        // 1. Candidatos diretos de coda
        let direct_candidates = if !pv.is_empty() {
            vec![
                format!("{pv} {cons}"),
                format!("{pv}_{cons}"),
                format!("{pv}-{cons}"),
                format!("{pv} {cons}・"),
                format!("{pv_kana} {cons}"),
                format!("{pv_kana}_{cons}"),
                format!("{pv_kana}-{cons}"),
                format!("_{cons}"),
                cons.to_string(),
            ]
        } else {
            vec![cons.to_string(), format!("- {cons}")]
        };

        for cand in &direct_candidates {
            if let Some(entry) = vb.find_mapped_entry(cand, pitch_name) {
                return Some((entry, false));
            }
        }

        // 2. Fallback de Coda Truncation para voicebanks sem alias VC puro
        let syllables: &[&str] = match cons {
            "s" => &["sa", "se", "su", "so", "si", "さ", "せ", "す", "そ", "し"],
            "k" => &["ka", "ke", "ku", "ko", "ki", "か", "け", "く", "こ", "き"],
            "t" => &["ta", "te", "to", "tsu", "chi", "た", "て", "と", "つ", "ち"],
            "p" => &["pa", "pe", "pu", "po", "pi", "ぱ", "ぺ", "ぷ", "ぽ", "pi", "ぴ"],
            "f" | "h" => &["ha", "he", "fu", "ho", "hi", "fa", "fe", "fo", "fi", "は", "へ", "ふ", "ほ", "ひ"],
            "m" => &["ma", "me", "mu", "mo", "mi", "ま", "me", "mu", "mo", "mi", "ま", "め", "む", "も", "み"],
            "n" => &["na", "ne", "nu", "no", "ni", "n", "ん", "な", "ね", "ぬ", "の", "に"],
            "r" | "l" => &["ra", "re", "ru", "ro", "ri", "la", "le", "lu", "lo", "li", "ら", "れ", "る", "ろ", "り"],
            "z" | "j" => &["za", "ze", "zu", "zo", "ji", "ざ", "ぜ", "ず", "ぞ", "じ"],
            "b" => &["ba", "be", "bu", "bo", "bi", "ば", "べ", "ぶ", "ぼ", "び"],
            "d" => &["da", "de", "do", "だ", "で", "ど"],
            "g" => &["ga", "ge", "gu", "go", "gi", "が", "げ", "ぐ", "ご", "ぎ"],
            "sh" => &["sha", "she", "shu", "sho", "shi", "しゃ", "しぇ", "しゅ", "しょ", "し"],
            "ch" => &["cha", "che", "chu", "cho", "chi", "ちゃ", "ちぇ", "ちゅ", "ちょ", "ち"],
            "ts" => &["tsa", "tse", "tsu", "tso", "tsi", "つ"],
            _ => &[],
        };

        if !pv.is_empty() {
            for syl in syllables {
                let vcv_cands = [
                    format!("{pv} {syl}"),
                    format!("{pv}_{syl}"),
                    format!("{pv}-{syl}"),
                    format!("{pv_kana} {syl}"),
                    format!("{pv_kana}_{syl}"),
                    format!("{pv_kana}-{syl}"),
                ];
                for cand in &vcv_cands {
                    if let Some(entry) = vb.find_mapped_entry(cand, pitch_name) {
                        return Some((entry, true));
                    }
                }
            }
        }

        for syl in syllables {
            if let Some(entry) = vb.find_mapped_entry(syl, pitch_name) {
                return Some((entry, true));
            }
        }

        None
    }

    /// Port of OpenUtau's legacy `JapaneseVCVPhonemizer.Process` candidate
    /// order.  This intentionally does not use the larger Kamafeu presamp
    /// resolver: presamp is a different OpenUtau phonemizer and must not
    /// silently change the legacy VCV contract.
    fn apply_openutau_vcv(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut previous_vowel: Option<String> = None;
        let mut previous_end_ms: Option<f64> = None;

        for (note_index, note) in notes.iter().enumerate() {
            let lyric = note
                .phonetic_hint
                .as_deref()
                .unwrap_or(note.lyric.as_str())
                .trim();
            if lyric.is_empty() || lyric == "R" {
                previous_vowel = None;
                previous_end_ms = None;
                continue;
            }
            if lyric.starts_with('+') {
                if let Some(last) = phones.last_mut() {
                    last.duration_ms += note.duration_ms;
                }
                previous_end_ms = Some(note.position_ms + note.duration_ms);
                continue;
            }

            let phrase_start = previous_end_ms
                .map(|end| note.position_ms > end + 0.001)
                .unwrap_or(true);
            let mut candidates = vec![format!("- {lyric}"), lyric.to_string()];
            if !phrase_start {
                if let Some(vowel) = previous_vowel.as_deref() {
                    candidates = vec![
                        format!("{vowel} {lyric}"),
                        format!("{vowel}_{lyric}"),
                        format!("* {lyric}"),
                        lyric.to_string(),
                        format!("- {lyric}"),
                    ];
                }
            }

            let mut coda_trunc = false;
            let mut entry = candidates.iter().find_map(|candidate| {
                for spelling in crate::phonemizer::romaji::lyric_candidates(candidate) {
                    if let Some(entry) = vb.find_mapped_entry(&spelling, &note.pitch) {
                        return Some(entry);
                    }
                }
                None
            });

            if entry.is_none() {
                if let Some(pv) = previous_vowel.as_deref() {
                    if let Some((fallback, is_trunc)) =
                        Self::find_coda_fallback_entry(vb, pv, lyric, &note.pitch)
                    {
                        entry = Some(fallback);
                        coda_trunc = is_trunc;
                    }
                } else if let Some((fallback, is_trunc)) =
                    Self::find_coda_fallback_entry(vb, "", lyric, &note.pitch)
                {
                    entry = Some(fallback);
                    coda_trunc = is_trunc;
                }
            }

            let alias = entry
                .map(|entry| entry.alias.clone())
                .unwrap_or_else(|| lyric.to_string());
            let current_vowel = Self::extract_vowel(lyric).map(str::to_owned);
            phones.push(RenderPhone {
                note_index,
                lyric: if coda_trunc && previous_vowel.is_some() {
                    format!("{} {}", previous_vowel.as_deref().unwrap(), lyric)
                } else {
                    alias.clone()
                },
                pitch: note.pitch.clone(),
                position_ms: note.position_ms,
                duration_ms: note.duration_ms,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
                resolved_alias: entry.map(|entry| entry.alias.clone()),
                resolved_timbre: note.timbre.clone(),
                coda_truncation: coda_trunc,
            });
            previous_vowel = current_vowel;
            previous_end_ms = Some(note.position_ms + note.duration_ms);
        }
        phones
    }

    /// OpenUtau's basic Japanese CV-VV lookup: explicit phonetic hints are
    /// authoritative; consonant notes use CV aliases, while two adjacent
    /// vowel notes may use a VV alias.  This is deliberately not VCV: a
    /// consonant note must never inherit the preceding vowel here.
    fn apply_openutau_cv(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut previous_vowel: Option<String> = None;
        let mut previous_end_ms: Option<f64> = None;
        for (note_index, note) in notes.iter().enumerate() {
            let lyric = note
                .phonetic_hint
                .as_deref()
                .unwrap_or(note.lyric.as_str())
                .trim();
            if lyric.is_empty() || lyric == "R" {
                previous_vowel = None;
                previous_end_ms = None;
                continue;
            }
            if lyric.starts_with('+') {
                if let Some(last) = phones.last_mut() {
                    last.duration_ms += note.duration_ms;
                }
                previous_end_ms = Some(note.position_ms + note.duration_ms);
                continue;
            }

            let phrase_start = previous_end_ms
                .map(|end| note.position_ms > end + 0.001)
                .unwrap_or(true);
            // `extract_vowel("か")` is `a`, but `か` is still a CV syllable.
            // VV is valid only for an isolated vowel lyric, never for a CV
            // note whose vowel is merely its nucleus.
            let current_vowel = Self::extract_vowel(lyric)
                .filter(|_| Self::extract_consonant(lyric).is_none())
                .map(str::to_owned);
            let candidates = if note.phonetic_hint.is_some() {
                vec![lyric.to_string()]
            } else if !phrase_start {
                if let (Some(previous_vowel), Some(_current_vowel)) =
                    (previous_vowel.as_deref(), current_vowel.as_deref())
                {
                    // Japanese CV-VV banks conventionally spell the vowel
                    // handoff as `previous_vowel current_vowel`.  Keep the
                    // plain CV fallback for banks without that recording.
                    vec![
                        format!("{previous_vowel} {lyric}"),
                        format!("{previous_vowel}_{lyric}"),
                        lyric.to_string(),
                    ]
                } else {
                    vec![format!("- {lyric}"), lyric.to_string()]
                }
            } else {
                vec![format!("- {lyric}"), lyric.to_string()]
            };
            let entry = candidates
                .iter()
                .find_map(|candidate| vb.find_mapped_entry(candidate, &note.pitch));
            let alias = entry
                .map(|entry| entry.alias.clone())
                .unwrap_or_else(|| lyric.to_string());
            phones.push(RenderPhone {
                note_index,
                lyric: alias.clone(),
                pitch: note.pitch.clone(),
                position_ms: note.position_ms,
                duration_ms: note.duration_ms,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
                resolved_alias: entry.map(|entry| entry.alias.clone()),
                resolved_timbre: note.timbre.clone(),
                coda_truncation: false,
            });
            previous_vowel = current_vowel;
            previous_end_ms = Some(note.position_ms + note.duration_ms);
        }
        phones
    }

    /// Port of the legacy OpenUtau Japanese CVVC contract. The main CV is
    /// kept on the current note and the optional V-C fragment is placed at
    /// the end of that same note, using the next note's oto timing.
    fn apply_openutau_cvvc(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        fn make_phone(
            note_index: usize,
            note: &UNote,
            alias: String,
            position_ms: f64,
            duration_ms: f64,
        ) -> RenderPhone {
            RenderPhone {
                note_index,
                lyric: alias.clone(),
                pitch: note.pitch.clone(),
                position_ms,
                duration_ms,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
                resolved_alias: Some(alias),
                resolved_timbre: note.timbre.clone(),
                coda_truncation: false,
            }
        }

        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut previous_end_ms: Option<f64> = None;

        for (note_index, note) in notes.iter().enumerate() {
            let lyric = note
                .phonetic_hint
                .as_deref()
                .unwrap_or(note.lyric.as_str())
                .trim();
            if lyric.is_empty() || lyric == "R" {
                previous_end_ms = None;
                continue;
            }
            if lyric.starts_with('+') {
                if let Some(last) = phones.last_mut() {
                    last.duration_ms += note.duration_ms;
                }
                previous_end_ms = Some(note.position_ms + note.duration_ms);
                continue;
            }

            let phrase_start = previous_end_ms
                .map(|end| note.position_ms > end + 0.001)
                .unwrap_or(true);
            let mut candidates = if note.phonetic_hint.is_some() {
                vec![lyric.to_string()]
            } else if phrase_start {
                vec![format!("- {lyric}"), lyric.to_string()]
            } else {
                // CVVC has a separate outgoing VC phone.  The main phone is
                // therefore always the current CV, never a VCV alias that
                // would repeat the same consonant transition.
                vec![format!("* {lyric}"), lyric.to_string()]
            };
            let main_entry = candidates
                .drain(..)
                .find_map(|candidate| vb.find_mapped_entry(&candidate, &note.pitch));
            let main_alias = main_entry
                .map(|entry| entry.alias.clone())
                .unwrap_or_else(|| lyric.to_string());

            let next = notes.get(note_index + 1).filter(|next| {
                let next_lyric = next
                    .phonetic_hint
                    .as_deref()
                    .unwrap_or(next.lyric.as_str())
                    .trim();
                !next_lyric.is_empty()
                    && next_lyric != "R"
                    && !next_lyric.starts_with('+')
                    && (next.position_ms - (note.position_ms + note.duration_ms)).abs() <= 0.001
            });

            let mut vc: Option<(String, f64)> = None;
            if let Some(next) = next {
                let next_lyric = next
                    .phonetic_hint
                    .as_deref()
                    .unwrap_or(next.lyric.as_str())
                    .trim();
                // A CV lyric such as `さ` has vowel nucleus `a`, but it is
                // still a consonant onset and therefore needs the outgoing
                // VC fragment from the preceding note.
                let next_is_plain_vowel = Self::extract_consonant(next_lyric).is_none();
                if !next_is_plain_vowel {
                    let vowel = Self::extract_vowel(lyric).unwrap_or("");
                    let consonant = Self::extract_consonant(next_lyric).unwrap_or("");
                    if !vowel.is_empty() && !consonant.is_empty() {
                        let substitute = match consonant {
                            "ch" | "ty" | "ts" => Some("t"),
                            "j" | "dy" => Some("d"),
                            "gy" => Some("g"),
                            "ky" => Some("k"),
                            "py" => Some("p"),
                            "ny" => Some("n"),
                            "ry" => Some("r"),
                            "my" => Some("m"),
                            "hy" | "f" => Some("h"),
                            "by" | "v" => Some("b"),
                            "dz" => Some("z"),
                            "l" => Some("r"),
                            "ly" => Some("l"),
                            _ => None,
                        };
                        let mut vc_candidates = vec![format!("{vowel} {consonant}")];
                        if let Some(substitute) = substitute {
                            vc_candidates.push(format!("{vowel} {substitute}"));
                        }
                        if let Some(entry) = vc_candidates
                            .iter()
                            .find_map(|candidate| vb.find_mapped_entry(candidate, &note.pitch))
                        {
                            let next_entry = vb.find_mapped_entry(next_lyric, &next.pitch);
                            let authored_ms = next_entry
                                .map(|entry| {
                                    if entry.overlap < 0.0 {
                                        entry.preutterance - entry.overlap
                                    } else {
                                        entry.preutterance
                                    }
                                })
                                .unwrap_or(120.0);
                            let scaled = authored_ms
                                * consonant_velocity_time_scale(
                                    next.expressions.consonant_velocity,
                                );
                            let vc_len = scaled.max(0.0).min((note.duration_ms * 0.5).max(0.0));
                            vc = Some((entry.alias.clone(), vc_len));
                        }
                    }
                }
            }

            let main_duration = vc
                .as_ref()
                .map(|(_, length)| (note.duration_ms - *length).max(0.0))
                .unwrap_or(note.duration_ms);
            phones.push(make_phone(
                note_index,
                note,
                main_alias,
                note.position_ms,
                main_duration,
            ));
            if let Some((vc_alias, vc_len)) = vc {
                phones.push(make_phone(
                    note_index,
                    note,
                    vc_alias,
                    note.position_ms + note.duration_ms - vc_len,
                    vc_len,
                ));
            }
            previous_end_ms = Some(note.position_ms + note.duration_ms);
        }
        phones
    }

    fn apply_english(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut prev_vowel: Option<String> = None;
        let mut prev_note_end_ms: Option<f64> = None;

        for (note_index, note) in notes.iter().enumerate() {
            if note.lyric.trim().is_empty() || note.lyric.trim() == "R" {
                prev_vowel = None;
                prev_note_end_ms = None;
                continue;
            }
            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => note.position_ms > end_ms + 0.001,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            let word_phones = if let Some(hint) = note.phonetic_hint.as_deref() {
                hint.split([',', ' '])
                    .filter(|token| !token.is_empty())
                    .map(|token| {
                        if mode == PhonemizerMode::EnglishXSampa {
                            token.to_string()
                        } else {
                            token.to_lowercase()
                        }
                    })
                    .collect::<Vec<_>>()
            } else {
                match mode {
                PhonemizerMode::EnglishArpasing | PhonemizerMode::EnglishArpasingPlus => {
                    if let Some(tokens) = Self::openutau_arpasing_tokens(&note.lyric, vb) {
                        tokens
                    } else {
                        // Arpasing is an explicit-phone mode when the singer
                        // does not ship its dictionary. Do not silently turn
                        // an English word into Kamafeu-specific graphemes.
                        crate::terminal_log::debug(
                            "Phonemizer",
                            format!("mode=EnglishArpasing word={:?} fallback=passthrough reason=missing_arpasing_yaml", note.lyric),
                        );
                        EnglishPhonemizer::phonetic_tokens(&note.lyric)
                    }
                }
                PhonemizerMode::EnglishG2P => Self::openutau_dictionary_tokens(&note.lyric, mode)
                    .unwrap_or_else(|| {
                        crate::terminal_log::debug(
                            "Phonemizer",
                            format!("mode=EnglishG2P word={:?} fallback=passthrough reason=missing_dictionary_entry", note.lyric),
                        );
                        EnglishPhonemizer::phonetic_tokens(&note.lyric)
                    }),
                _ => EnglishPhonemizer::phonetic_tokens(&note.lyric),
                }
            };
            if word_phones.is_empty() {
                continue;
            }

            let sub_dur = note.duration_ms / word_phones.len() as f64;
            for (idx, p) in word_phones.iter().enumerate() {
                let sub_pos = note.position_ms + (idx as f64 * sub_dur);
                let mut alias = p.clone();

                if is_phrase_start && idx == 0 {
                    let head_try = format!("- {}", p);
                    if let Some(entry) = vb.find_mapped_entry(&head_try, &note.pitch) {
                        alias = entry.alias.clone();
                    }
                } else if idx == 0 && !is_phrase_start {
                    if let Some(ref pv) = prev_vowel {
                        let vc_try = format!("{} {}", pv, p);
                        if let Some(entry) = vb.find_mapped_entry(&vc_try, &note.pitch) {
                            let stretch =
                                consonant_velocity_time_scale(note.expressions.consonant_velocity);
                            let authored = if entry.overlap < 0.0 {
                                entry.preutterance - entry.overlap
                            } else {
                                entry.preutterance
                            } * stretch;
                            let vc_dur = authored.max(5.0).min((note.duration_ms * 0.5).max(5.0));
                            if let Some(last) = phones.last_mut() {
                                let borrow = vc_dur.min((last.duration_ms - 20.0).max(0.0));
                                if borrow > 0.0 {
                                    last.duration_ms -= borrow;
                                    phones.push(RenderPhone {
                                        note_index,
                                        lyric: entry.alias.clone(),
                                        pitch: note.pitch.clone(),
                                        position_ms: sub_pos - borrow,
                                        duration_ms: borrow,
                                        envelope: crate::dsp::envelope::UtauEnvelope::default(),
                                        expressions: note.expressions.clone(),
                                        pitch_bend: crate::project::model::UPitchBend::default(),
                                        vibrato: crate::dsp::pitch::VibratoParam::default(),
                                        flags: note.flags.clone(),
                                        resolved_alias: None,
                                        resolved_timbre: None,
                                        coda_truncation: false,
                                    });
                                }
                            }
                        }
                    }
                } else if let Some(ref pv) = prev_vowel {
                    let vc_try = format!("{} {}", pv, p);
                    if let Some(entry) = vb.find_mapped_entry(&vc_try, &note.pitch) {
                        alias = entry.alias.clone();
                    }
                }

                if EnglishPhonemizer::is_vowel(p)
                    || (mode == PhonemizerMode::EnglishXSampa
                        && matches!(
                            p.as_str(),
                            "a" | "A"
                                | "@"
                                | "{"
                                | "V"
                                | "O"
                                | "E"
                                | "3"
                                | "eI"
                                | "I"
                                | "i"
                                | "oU"
                                | "OI"
                                | "U"
                                | "u"
                                | "Q"
                                | "Ol"
                                | "Q@"
                                | "2"
                                | "6"
                                | "7"
                                | "8"
                                | "9"
                                | "&"
                                | "}"
                                | "1"
                                | "e"
                                | "o"
                                | "i:"
                                | "u:"
                                | "O:"
                        ))
                {
                    prev_vowel = Some(p.clone());
                } else {
                    prev_vowel = None;
                }

                let resolved_entry = if matches!(
                    mode,
                    PhonemizerMode::EnglishArpasing | PhonemizerMode::EnglishArpasingPlus
                ) {
                    std::iter::once(alias.as_str())
                        .chain(Self::arpasing_vowel_fallbacks(p).iter().copied())
                        .find_map(|candidate| vb.find_mapped_entry(candidate, &note.pitch))
                } else {
                    vb.find_mapped_entry(&alias, &note.pitch)
                };
                if let Some(entry) = resolved_entry {
                    alias = entry.alias.clone();
                }

                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: sub_pos,
                    duration_ms: sub_dur,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                    resolved_alias: None,
                    resolved_timbre: None,
                    coda_truncation: false,
                });
            }

            prev_note_end_ms = Some(note.position_ms + note.duration_ms);
        }

        phones
    }

    /// OpenUtau's `ArpasingPhonemizer` is a Latin diphone phonemizer: aliases
    /// are selected from the previous symbol and the current symbol, with a
    /// leading `-` at a phrase boundary.  The old Kamafeu path tracked only
    /// the previous vowel, which lost clusters such as `s t ae`.  Keep this
    /// implementation separate from the generic G2P path so its alias graph
    /// can be compared directly with OpenUtau's `LatinDiphonePhonemizer`.
    fn apply_arpasing(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        let mut phones = Vec::new();
        let mut previous_symbol: Option<String> = None;

        for (note_index, note) in notes.iter().enumerate() {
            let lyric = note.lyric.trim();
            if lyric.is_empty() || lyric.eq_ignore_ascii_case("r") {
                previous_symbol = None;
                continue;
            }

            let phrase_start = note_index == 0
                || notes
                    .get(note_index.saturating_sub(1))
                    .is_none_or(|previous| {
                        previous.position_ms + previous.duration_ms < note.position_ms - 0.001
                    });
            if phrase_start {
                previous_symbol = None;
            }

            let symbols = if let Some(hint) = note.phonetic_hint.as_deref() {
                hint.split_whitespace()
                    .filter(|symbol| !symbol.is_empty())
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
            } else {
                Self::openutau_arpasing_tokens(lyric, vb).unwrap_or_else(|| {
                    crate::terminal_log::debug(
                        "Phonemizer",
                        format!("mode=EnglishArpasing word={lyric:?} fallback=passthrough reason=missing_arpasing_yaml"),
                    );
                    EnglishPhonemizer::phonetic_tokens(lyric)
                })
            };
            if symbols.is_empty() {
                continue;
            }

            let phrase_end = notes.get(note_index + 1).is_none_or(|next| {
                next.lyric.trim().is_empty()
                    || next.lyric.eq_ignore_ascii_case("r")
                    || next.position_ms > note.position_ms + note.duration_ms + 0.001
            });
            let symbol_count = (symbols.len() + usize::from(phrase_end)) as f64;
            let duration = note.duration_ms / symbol_count;
            for (symbol_index, symbol) in symbols.iter().enumerate() {
                let previous = if symbol_index == 0 {
                    previous_symbol.as_deref().unwrap_or("-")
                } else {
                    symbols[symbol_index - 1].as_str()
                };
                let alias = Self::resolve_arpasing_alias(previous, symbol, vb, note);
                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: note.position_ms + symbol_index as f64 * duration,
                    duration_ms: duration,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                    resolved_alias: None,
                    resolved_timbre: note.timbre.clone(),
                    coda_truncation: false,
                });
            }

            if phrase_end {
                let previous = symbols.last().map(String::as_str).unwrap_or("-");
                let alias = Self::resolve_arpasing_alias(previous, "-", vb, note);
                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: note.position_ms + symbols.len() as f64 * duration,
                    duration_ms: duration,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                    resolved_alias: None,
                    resolved_timbre: note.timbre.clone(),
                    coda_truncation: false,
                });
            }

            previous_symbol = symbols.last().cloned();
        }
        phones
    }

    fn resolve_arpasing_alias(
        previous: &str,
        symbol: &str,
        vb: &Voicebank,
        note: &UNote,
    ) -> String {
        let candidate = format!("{previous} {symbol}");
        if let Some(entry) = vb.find_mapped_entry(&candidate, &note.pitch) {
            return entry.alias.clone();
        }
        for fallback in Self::arpasing_vowel_fallbacks(symbol) {
            let candidate = format!("{previous} {fallback}");
            if let Some(entry) = vb.find_mapped_entry(&candidate, &note.pitch) {
                return entry.alias.clone();
            }
        }
        if let Some(entry) = vb.find_mapped_entry(&format!("- {symbol}"), &note.pitch) {
            return entry.alias.clone();
        }
        format!("{previous} {symbol}")
    }

    fn apply_portuguese(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut prev_vowel: Option<String> = None;
        let mut prev_note_end_ms: Option<f64> = None;

        for (note_index, note) in notes.iter().enumerate() {
            if note.lyric.trim().is_empty() || note.lyric.trim() == "R" {
                prev_vowel = None;
                prev_note_end_ms = None;
                continue;
            }
            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => note.position_ms > end_ms + 0.001,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            let syllables = match mode {
                PhonemizerMode::PortugueseG2P => {
                    PortuguesePhonemizer::word_to_phonemes(&note.lyric)
                }
                _ => PortuguesePhonemizer::phonetic_tokens(&note.lyric),
            };
            if syllables.is_empty() {
                continue;
            }

            let sub_dur = note.duration_ms / syllables.len() as f64;
            for (idx, syl) in syllables.iter().enumerate() {
                let sub_pos = note.position_ms + (idx as f64 * sub_dur);
                let mut alias = syl.clone();

                if is_phrase_start && idx == 0 {
                    let head_try = format!("- {}", syl);
                    if let Some(entry) = vb.find_mapped_entry(&head_try, &note.pitch) {
                        alias = entry.alias.clone();
                    }
                } else if mode == PhonemizerMode::PortugueseVCV {
                    if let Some(ref pv) = prev_vowel {
                        let vcv_try = format!("{} {}", pv, syl);
                        if let Some(entry) = vb.find_mapped_entry(&vcv_try, &note.pitch) {
                            alias = entry.alias.clone();
                        }
                    }
                } else if mode == PhonemizerMode::PortugueseCVVC {
                    if let (Some(ref pv), Some(cc)) = (
                        prev_vowel.as_ref(),
                        PortuguesePhonemizer::extract_consonant(syl),
                    ) {
                        let vc_try = format!("{} {}", pv, cc);
                        if let Some(entry) = vb.find_mapped_entry(&vc_try, &note.pitch) {
                            let stretch =
                                consonant_velocity_time_scale(note.expressions.consonant_velocity);
                            let authored = if entry.overlap < 0.0 {
                                entry.preutterance - entry.overlap
                            } else {
                                entry.preutterance
                            } * stretch;
                            let vc_dur = authored.max(5.0).min((sub_dur * 0.5).max(5.0));
                            if let Some(last) = phones.last_mut() {
                                let borrow = vc_dur.min((last.duration_ms - 20.0).max(0.0));
                                if borrow > 0.0 {
                                    last.duration_ms -= borrow;
                                    phones.push(RenderPhone {
                                        note_index,
                                        lyric: entry.alias.clone(),
                                        pitch: note.pitch.clone(),
                                        position_ms: sub_pos - borrow,
                                        duration_ms: borrow,
                                        envelope: crate::dsp::envelope::UtauEnvelope::default(),
                                        expressions: note.expressions.clone(),
                                        pitch_bend: crate::project::model::UPitchBend::default(),
                                        vibrato: crate::dsp::pitch::VibratoParam::default(),
                                        flags: note.flags.clone(),
                                        resolved_alias: None,
                                        resolved_timbre: None,
                                        coda_truncation: false,
                                    });
                                }
                            }
                        }
                    }
                }

                if let Some(v) = PortuguesePhonemizer::extract_vowel(syl) {
                    prev_vowel = Some(v.to_string());
                }

                if let Some(entry) = vb.find_mapped_entry(&alias, &note.pitch) {
                    alias = entry.alias.clone();
                }

                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: sub_pos,
                    duration_ms: sub_dur,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                    resolved_alias: None,
                    resolved_timbre: None,
                    coda_truncation: false,
                });
            }

            prev_note_end_ms = Some(note.position_ms + note.duration_ms);
        }

        phones
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_voicebank_with_aliases(aliases: &[&str]) -> (tempfile::TempDir, Voicebank) {
        let directory = tempfile::tempdir().unwrap();
        let mut voicebank = Voicebank::new(directory.path()).unwrap();
        for alias in aliases {
            voicebank.entries.insert(
                (*alias).to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    (*alias).to_string(),
                    0.0,
                    80.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        (directory, voicebank)
    }

    #[test]
    fn basic_cv_uses_vv_only_between_adjacent_vowels() {
        let (_directory, voicebank) =
            test_voicebank_with_aliases(&["- あ", "a い", "あ", "か", "a か"]);
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("い", "C4", 400.0, 400.0),
        ];
        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &voicebank, PhonemizerMode::BasicCV);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- あ", "a い"]
        );

        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
        ];
        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &voicebank, PhonemizerMode::BasicCV);
        assert_eq!(phones[1].lyric, "か");
    }

    #[test]
    fn cvvc_keeps_cv_main_phone_and_adds_only_outgoing_vc() {
        let (_directory, voicebank) = test_voicebank_with_aliases(&["- あ", "a さ", "a s", "さ"]);
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &voicebank, PhonemizerMode::CVVC);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- あ", "a s", "さ"]
        );
    }

    #[test]
    fn dispatcher_reports_supported_openutau_families_explicitly() {
        assert_eq!(
            OpenUtauPhonemizer::implementation(PhonemizerMode::None),
            PhonemizerImplementation::Passthrough
        );
        assert_eq!(
            OpenUtauPhonemizer::implementation(PhonemizerMode::VCV),
            PhonemizerImplementation::OpenUtau
        );
        assert_eq!(PhonemizerMode::VCV.info().source, "OpenUtau (validado)");
        for mode in [
            PhonemizerMode::BasicCV,
            PhonemizerMode::CVVC,
            PhonemizerMode::JapanesePresamp,
        ] {
            assert_eq!(
                OpenUtauPhonemizer::implementation(mode),
                PhonemizerImplementation::OpenUtau
            );
            assert_eq!(mode.info().source, "OpenUtau (validado)");
        }
        assert_eq!(
            OpenUtauPhonemizer::implementation(PhonemizerMode::EnglishVCCV),
            PhonemizerImplementation::OpenUtau
        );
    }

    #[test]
    fn japanese_presamp_dispatch_uses_voicebank_presamp_ini() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("presamp.ini"),
            "[VOWEL]\na=あ=あ\n[CONSONANT]\nk=k,か\n[ALIAS]\nVC=a k\nVCV=%v% %CV%\nBEGINNINGCV=- %CV%\n",
        )
        .unwrap();
        let mut voicebank = Voicebank::new(directory.path()).unwrap();
        for alias in ["- あ", "a か"] {
            voicebank.entries.insert(
                alias.to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    80.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }

        let notes = vec![
            UNote::new("あ", "C4", 0.0, 120.0),
            UNote::new("か", "C4", 120.0, 120.0),
        ];
        let phones = OpenUtauPhonemizer::apply(&notes, &voicebank, PhonemizerMode::JapanesePresamp);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/japanese_presamp.json"
        ))
        .unwrap();
        assert_eq!(fixture["upstream"]["class"], "JapanesePresampPhonemizer");
        let expected = fixture["cases"][0]["expected_aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn arpasing_uses_the_voicebank_openutau_dictionary() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("arpasing.yaml"),
            "symbols: [{symbol: hh, type: aspirate}]\nentries:\n  - grapheme: hello\n    phonemes: [hh, eh, l, ow]\n  - grapheme: WORLD\n    phonemes: [w, er, l, d]\n",
        )
        .unwrap();
        let vb = Voicebank::new(dir.path()).unwrap();

        assert_eq!(
            JapanesePhonemizer::openutau_arpasing_tokens("hello", &vb).unwrap(),
            vec!["hh", "eh", "l", "ow"]
        );
        assert_eq!(
            JapanesePhonemizer::openutau_arpasing_tokens("world", &vb).unwrap(),
            vec!["w", "er", "l", "d"]
        );
        assert_eq!(
            JapanesePhonemizer::openutau_arpasing_tokens("can", &vb).unwrap(),
            vec!["k", "ae", "n"]
        );
        assert_eq!(JapanesePhonemizer::arpasing_vowel_fallbacks("ao"), &["ow"]);
        assert!(JapanesePhonemizer::openutau_arpasing_tokens("zzzznotaword", &vb).is_none());
    }

    #[test]
    fn arpasing_uses_the_previous_symbol_for_consonant_clusters() {
        let (_directory, voicebank) =
            test_voicebank_with_aliases(&["- s", "s t", "t ae", "ae n", "n -", "- ao", "- ah"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 100.0);
        first.phonetic_hint = Some("s".to_string());
        let mut second = UNote::new("ignored", "C4", 100.0, 300.0);
        second.phonetic_hint = Some("t ae n".to_string());
        let phones = JapanesePhonemizer::apply_phonemizer(
            &[first, second],
            &voicebank,
            PhonemizerMode::EnglishArpasing,
        );

        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/english_arpasing.json"
        ))
        .unwrap();
        let expected = fixture["cases"][0]["expected_aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    use std::collections::HashMap;

    #[test]
    fn per_note_timbre_preserves_vcv_context_and_default() {
        let dir = tempfile::tempdir().unwrap();
        let mut vb = Voicebank::new(dir.path()).unwrap();
        vb.prefix_map = crate::oto::PrefixMap::parse_yaml_str("subbanks:\n  - color: ''\n    suffix: _normal\n    tone_ranges: [C4-C5]\n  - color: Soft\n    suffix: _soft\n    tone_ranges: [C4-C5]\n");
        for alias in ["- あ_normal", "a か_soft", "a か_normal"] {
            vb.entries.insert(
                alias.to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    50.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        let mut notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
            UNote::new("か", "C4", 800.0, 400.0),
        ];
        notes[1].timbre = Some("Soft".to_string());
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(
            phones.iter().map(|p| p.lyric.as_str()).collect::<Vec<_>>(),
            ["- あ_normal", "a か_soft", "a か_normal"]
        );
        assert_eq!(
            phones.iter().map(|p| p.note_index).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(phones[1].position_ms, 400.0);
        assert_eq!(
            phones
                .iter()
                .map(|p| p.resolved_alias.as_deref())
                .collect::<Vec<_>>(),
            [Some("- あ_normal"), Some("a か_soft"), Some("a か_normal")]
        );
        assert_eq!(phones[1].resolved_timbre.as_deref(), Some("Soft"));
        assert_eq!(vb.prefix_map.selected_color(), "");

        // Repeated per-note timbre resolution must not leak the temporary
        // color into the shared PrefixMap state.
        notes[0].timbre = Some("Soft".to_string());
        notes[1].timbre = None;
        let swapped = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(swapped[0].resolved_timbre.as_deref(), Some("Soft"));
        assert_eq!(swapped[1].resolved_timbre, None);
        assert_eq!(vb.prefix_map.selected_color(), "");
        // Existing projects without the new field still inherit the default.
        let legacy: UNote =
            serde_json::from_str(r#"{"lyric":"a","pitch":"C4","position_ms":0,"duration_ms":400}"#)
                .unwrap();
        assert_eq!(legacy.timbre, None);
    }

    #[test]
    fn cvvc_outgoing_transition_uses_destination_timbre() {
        let dir = tempfile::tempdir().unwrap();
        let mut vb = Voicebank::new(dir.path()).unwrap();
        vb.prefix_map = crate::oto::PrefixMap::parse_yaml_str(
            "subbanks:\n  - color: ''\n    suffix: _normal\n    tone_ranges: [C4-C5]\n  - color: Soft\n    suffix: _soft\n    tone_ranges: [C4-C5]\n",
        );
        for alias in [
            "- あ_normal",
            "a k_normal",
            "a k_soft",
            "か_soft",
            "か_normal",
        ] {
            vb.entries.insert(
                alias.to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    50.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        let mut notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
        ];
        notes[1].timbre = Some("Soft".to_string());
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::CVVC);
        let transition = phones.iter().find(|phone| phone.lyric == "a k_soft");
        assert!(transition.is_some(), "phones: {:?}", phones);
        assert_eq!(transition.unwrap().resolved_timbre.as_deref(), Some("Soft"));
    }

    #[test]
    fn frozen_alias_does_not_fall_back_to_prefix_map() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("oto.ini"),
            "a.wav=a_normal,0,50,-200,40,0\n",
        )
        .unwrap();
        let vb = Voicebank::new(dir.path()).unwrap();
        let phone = RenderPhone {
            note_index: 0,
            lyric: "a".to_string(),
            pitch: "C4".to_string(),
            position_ms: 0.0,
            duration_ms: 100.0,
            envelope: crate::dsp::envelope::UtauEnvelope::default(),
            expressions: crate::project::model::UExpressions::default(),
            pitch_bend: crate::project::model::UPitchBend::default(),
            vibrato: crate::dsp::pitch::VibratoParam::default(),
            flags: String::new(),
            resolved_alias: Some("a_missing".to_string()),
            resolved_timbre: Some("Soft".to_string()),
            coda_truncation: false,
        };
        assert!(phone.find_entry(&vb).is_none());
    }

    #[test]
    fn test_phrase_start_head_alias() {
        let mut entries = HashMap::new();
        entries.insert(
            "- か".to_string(),
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- か".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a き".to_string(),
            crate::oto::OtoEntry::new(
                "ki.wav".to_string(),
                "a き".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("ki", "C4", 400.0, 400.0),
            UNote::new("ka", "C4", 2000.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "- か");
        assert_eq!(phones[1].lyric, "a き");
        assert_eq!(phones[2].lyric, "- か");
    }

    #[test]
    fn test_english_phonemizer() {
        let phones = EnglishPhonemizer::word_to_arpabet("hello");
        assert_eq!(phones, vec!["hh", "ah", "l", "ow"]);

        let sing = EnglishPhonemizer::word_to_arpabet("sing");
        assert_eq!(sing, vec!["s", "ih", "ng"]);
    }

    #[test]
    fn test_portuguese_phonemizer() {
        let phones = PortuguesePhonemizer::word_to_phonemes("canto");
        assert_eq!(phones, vec!["ka", "n", "to"]);

        let amor = PortuguesePhonemizer::word_to_phonemes("amor");
        assert_eq!(amor, vec!["a", "mo", "r"]);
    }

    #[test]
    fn test_none_phonemizer_passthrough() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k_a", "C4", 0.0, 400.0),
            UNote::new("R", "C4", 400.0, 200.0),
            UNote::new("- sa", "D4", 600.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "k_a");
        assert_eq!(phones[0].position_ms, 0.0);
        assert_eq!(phones[1].lyric, "- sa");
        assert_eq!(phones[1].position_ms, 600.0);
    }

    #[test]
    fn phoneme_override_applies_alias_and_advanced_vocal_controls() {
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: std::collections::HashMap::new(),
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };
        let mut note = UNote::new("original", "C4", 0.0, 400.0);
        note.phoneme_overrides
            .push(crate::project::model::UPhonemeOverride {
                index: 0,
                phoneme: Some("k a".to_string()),
                envelope: Some(crate::dsp::envelope::UtauEnvelope {
                    p2: 19.0,
                    ..Default::default()
                }),
                vibrato: Some(crate::dsp::pitch::VibratoParam {
                    length_pct: 33.0,
                    ..Default::default()
                }),
                pitch_bend: Some(crate::project::model::UPitchBend {
                    portamento_length_ms: 77.0,
                    ..Default::default()
                }),
                consonant_timing_offset_ms: Some(14.0),
                ..Default::default()
            });

        let phones = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 1);
        assert_eq!(phones[0].lyric, "k a");
        assert_eq!(phones[0].envelope.p2, 19.0);
        assert_eq!(phones[0].vibrato.length_pct, 33.0);
        assert_eq!(phones[0].pitch_bend.portamento_length_ms, 77.0);
        assert_eq!(phones[0].expressions.consonant_timing_offset_ms, 14.0);
    }

    #[test]
    fn test_universal_plus_continuation_in_any_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k aa", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];

        let phones_none = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones_none.len(), 1);
        assert_eq!(phones_none[0].lyric, "k aa");
        assert_eq!(phones_none[0].duration_ms, 1000.0); // 500 + 250 + 250

        let notes_en = vec![
            UNote::new("sing", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];
        let phones_en =
            JapanesePhonemizer::apply_phonemizer(&notes_en, &vb, PhonemizerMode::EnglishArpasing);
        let total_dur: f64 = phones_en.iter().map(|p| p.duration_ms).sum();
        assert_eq!(total_dur, 1000.0);
    }

    #[test]
    fn test_manual_mode_semicolon_and_comma_sub_phonemes() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![UNote::new("-k;k ae;ae n", "C4", 0.0, 300.0)];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "-k");
        assert_eq!(phones[0].duration_ms, 180.0);
        assert_eq!(phones[1].lyric, "k ae");
        assert_eq!(phones[1].duration_ms, 60.0);
        assert_eq!(phones[2].lyric, "ae n");
        assert_eq!(phones[2].duration_ms, 60.0);

        let notes_comma = vec![UNote::new("k ae, ae n", "C4", 0.0, 300.0)];
        let phones_comma =
            JapanesePhonemizer::apply_phonemizer(&notes_comma, &vb, PhonemizerMode::None);
        assert_eq!(phones_comma.len(), 2);
        assert_eq!(phones_comma[0].lyric, "k ae");
        assert_eq!(phones_comma[0].duration_ms, 240.0);
        assert_eq!(phones_comma[1].lyric, "ae n");
        assert_eq!(phones_comma[1].duration_ms, 60.0);

        let mut resized_note = UNote::new("k ae.ae n.", "B3", 0.0, 480.0);
        resized_note.set_phoneme_boundary(2, 1, 360.0);
        let resized =
            JapanesePhonemizer::apply_phonemizer(&[resized_note], &vb, PhonemizerMode::None);
        assert_eq!(resized.len(), 2);
        assert_eq!(resized[0].lyric, "k ae");
        assert_eq!(resized[0].position_ms, 0.0);
        assert_eq!(resized[0].duration_ms, 360.0);
        assert_eq!(resized[1].lyric, "ae n");
        assert_eq!(resized[1].position_ms, 360.0);
        assert_eq!(resized[1].duration_ms, 120.0);

        let notes_dot = vec![UNote::new("m an. an d. d eh. eh l. l a.", "C4", 0.0, 500.0)];
        let phones_dot =
            JapanesePhonemizer::apply_phonemizer(&notes_dot, &vb, PhonemizerMode::None);
        assert_eq!(phones_dot.len(), 5);
        assert_eq!(phones_dot[0].lyric, "m an");
        assert_eq!(phones_dot[0].duration_ms, 260.0);
        assert_eq!(phones_dot[1].lyric, "an d");
        assert_eq!(phones_dot[1].duration_ms, 60.0);
        assert_eq!(phones_dot[2].lyric, "d eh");
        assert_eq!(phones_dot[2].duration_ms, 60.0);
        assert_eq!(phones_dot[3].lyric, "eh l");
        assert_eq!(phones_dot[3].duration_ms, 60.0);
        assert_eq!(phones_dot[4].lyric, "l a");
        assert_eq!(phones_dot[4].duration_ms, 60.0);
    }

    #[test]
    fn manual_composite_uses_oto_preutterance_instead_of_equal_slices() {
        let mut entries = HashMap::new();
        entries.insert(
            "a rh-".to_string(),
            crate::oto::OtoEntry::new(
                "a-rh.wav".to_string(),
                "a rh-".to_string(),
                0.0,
                268.0,
                -147.0,
                122.0,
                90.0,
            ),
        );
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };
        let phones = JapanesePhonemizer::apply_phonemizer(
            &[UNote::new("t a.a rh-", "B3", 0.0, 1200.0)],
            &vb,
            PhonemizerMode::None,
        );
        assert_eq!(phones.len(), 2);
        assert!((phones[0].duration_ms - 1078.0).abs() < 1e-6);
        assert!((phones[1].position_ms - 1078.0).abs() < 1e-6);
        assert!((phones[1].duration_ms - 122.0).abs() < 1e-6);
    }

    #[test]
    fn test_g2p_vs_phonetic_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes_word = vec![UNote::new("can", "C4", 0.0, 300.0)];
        let phones_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones_g2p.len(), 3);
        assert_eq!(phones_g2p[0].lyric, "k");
        assert_eq!(phones_g2p[1].lyric, "ae");
        assert_eq!(phones_g2p[2].lyric, "n");

        let phones_direct =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishArpasing);
        assert_eq!(phones_direct.len(), 4);
        assert_eq!(
            phones_direct
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- k", "k ae", "ae n", "n -"]
        );

        let notes_pt = vec![UNote::new("sol", "C4", 0.0, 300.0)];
        let phones_pt_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_pt, &vb, PhonemizerMode::PortugueseG2P);
        assert_eq!(phones_pt_g2p.len(), 3);
        assert_eq!(phones_pt_g2p[0].lyric, "s");
        assert_eq!(phones_pt_g2p[1].lyric, "O");
        assert_eq!(phones_pt_g2p[2].lyric, "w");

        let notes_brapa = vec![UNote::new("canto", "C4", 0.0, 400.0)];
        let phones_brapa = JapanesePhonemizer::apply_phonemizer(
            &notes_brapa,
            &vb,
            PhonemizerMode::PortugueseBrapaVCCV,
        );
        assert!(!phones_brapa.is_empty());
    }

    #[test]
    fn test_vccv_brapa_phonemizer_full_cascade() {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            "- ka".to_string(),
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- ka".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a s".to_string(),
            crate::oto::OtoEntry::new(
                "as.wav".to_string(),
                "a s".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "s a".to_string(),
            crate::oto::OtoEntry::new(
                "sa.wav".to_string(),
                "s a".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "BRAPA VB".to_string(),
            author: "Xiao".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("sa", "C4", 400.0, 400.0),
        ];

        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::PortugueseBrapaVCCV);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- ka", "a s", "s a"]
        );
        assert_eq!(phones[0].position_ms, 0.0);
        assert_eq!(phones[1].position_ms, 375.0);
    }

    #[test]
    fn test_custom_phoneme_durations_across_all_phonemizers() {
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: HashMap::new(),
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let mut note = UNote::new("can", "C4", 100.0, 600.0);
        note.phoneme_durations_ms = vec![100.0, 350.0, 150.0];

        let phones = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].duration_ms, 100.0);
        assert_eq!(phones[0].position_ms, 100.0);
        assert_eq!(phones[1].duration_ms, 350.0);
        assert_eq!(phones[1].position_ms, 200.0);
        assert_eq!(phones[2].duration_ms, 150.0);
        assert_eq!(phones[2].position_ms, 550.0);
    }

    #[test]
    fn rule_g2p_handles_arabic_and_icelandic_scripts() {
        let arabic = JapanesePhonemizer::arabic_tokens("شمس");
        assert_eq!(arabic, vec!["sh", "m", "s"]);
        let icelandic = JapanesePhonemizer::icelandic_tokens("þú");
        assert_eq!(icelandic, vec!["T", "u"]);
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("chanson", PhonemizerMode::FrenchG2P),
            vec!["sh", "a", "n", "s", "o", "n"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("шаг", PhonemizerMode::RussianG2P),
            vec!["sh", "a", "g"]
        );
        assert_eq!(
            JapanesePhonemizer::openutau_dictionary_tokens("-ce", PhonemizerMode::FrenchG2P)
                .unwrap(),
            vec!["ss"]
        );
        assert_eq!(
            JapanesePhonemizer::openutau_dictionary_tokens("can", PhonemizerMode::EnglishG2P)
                .unwrap(),
            vec!["k", "ae", "n"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("ni3 hao3", PhonemizerMode::ChineseG2P),
            vec!["ni3", "hao3"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("你好", PhonemizerMode::ChineseG2P),
            vec!["ni3", "hao3"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("你好", PhonemizerMode::CantoneseG2P),
            vec!["nei5", "hou2"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("廣東", PhonemizerMode::CantoneseG2P),
            vec!["gwong2", "dung1"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("şeu", PhonemizerMode::TurkishCVVC),
            vec!["sh", "eu"]
        );
    }

    #[test]
    fn catalog_contains_requested_languages_and_origins() {
        let languages: std::collections::BTreeSet<_> = PhonemizerMode::ALL
            .iter()
            .map(|mode| mode.info().language)
            .collect();
        for language in [
            "Inglês",
            "Japonês",
            "Francês",
            "Português",
            "Alemão",
            "Russo",
            "Espanhol",
            "Turco",
            "Chinês",
            "Cantonês",
            "Árabe",
            "Islandês",
        ] {
            assert!(languages.contains(language), "missing language {language}");
        }
        assert_eq!(
            PhonemizerMode::ArabicG2P.info().source,
            "Plugin dorayakito/OpenUtau (dados)"
        );
        assert_eq!(
            PhonemizerMode::IcelandicG2P.info().source,
            "Plugin dorayakito/OpenUtau (dados)"
        );
        assert_eq!(
            PhonemizerMode::FrenchG2P.info().source,
            "OpenUtau (portado)"
        );
    }

    #[test]
    fn test_intra_note_split_ko_e_s_with_coda_truncation() {
        let mut entries = std::collections::HashMap::new();
        // Em um voicebank VCV tradicional japonês, "e s" não existe, mas "e sa" existe
        entries.insert(
            "- ko".to_string(),
            crate::oto::OtoEntry {
                alias: "- ko".to_string(),
                wav_filename: "ko.wav".to_string(),
                offset: 10.0,
                consonant: 50.0,
                cutoff: -120.0,
                preutterance: 30.0,
                overlap: 15.0,
                ..Default::default()
            },
        );
        entries.insert(
            "o e".to_string(),
            crate::oto::OtoEntry {
                alias: "o e".to_string(),
                wav_filename: "oe.wav".to_string(),
                offset: 10.0,
                consonant: 40.0,
                cutoff: -100.0,
                preutterance: 25.0,
                overlap: 10.0,
                ..Default::default()
            },
        );
        entries.insert(
            "e sa".to_string(),
            crate::oto::OtoEntry {
                alias: "e sa".to_string(),
                wav_filename: "esa.wav".to_string(),
                offset: 10.0,
                consonant: 60.0,
                cutoff: -150.0,
                preutterance: 35.0,
                overlap: 12.0,
                ..Default::default()
            },
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "VCV Voicebank".to_string(),
            author: "Author".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let note = UNote::new("ko.e.s", "C4", 1000.0, 480.0);
        let phones = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::VCV);

        assert_eq!(phones.len(), 3, "Deveria decompor ko.e.s em 3 fonemas intra-nota: {:?}", phones);
        assert_eq!(phones[0].note_index, 0);
        assert_eq!(phones[0].lyric, "- ko");

        assert_eq!(phones[1].note_index, 0);
        assert_eq!(phones[1].lyric, "o e");

        assert_eq!(phones[2].note_index, 0);
        assert_eq!(phones[2].lyric, "e s");
        assert_eq!(phones[2].resolved_alias.as_deref(), Some("e sa"));
        assert!(phones[2].coda_truncation, "Deveria ter coda_truncation ativado");
    }

    #[test]
    fn test_intra_note_split_ko_e_s_with_hiragana_bank_and_mode_none() {
        let mut entries = std::collections::HashMap::new();
        // Banco no formato japonês tradicional com Hiragana (ex: Yamine Renri VCV)
        entries.insert(
            "- こ".to_string(),
            crate::oto::OtoEntry {
                alias: "- こ".to_string(),
                wav_filename: "ko.wav".to_string(),
                offset: 10.0,
                consonant: 50.0,
                cutoff: -120.0,
                preutterance: 30.0,
                overlap: 15.0,
                ..Default::default()
            },
        );
        entries.insert(
            "o え".to_string(),
            crate::oto::OtoEntry {
                alias: "o え".to_string(),
                wav_filename: "oe.wav".to_string(),
                offset: 10.0,
                consonant: 40.0,
                cutoff: -100.0,
                preutterance: 25.0,
                overlap: 10.0,
                ..Default::default()
            },
        );
        entries.insert(
            "e さ".to_string(),
            crate::oto::OtoEntry {
                alias: "e さ".to_string(),
                wav_filename: "esa.wav".to_string(),
                offset: 10.0,
                consonant: 60.0,
                cutoff: -150.0,
                preutterance: 35.0,
                overlap: 12.0,
                ..Default::default()
            },
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Renri Style Bank".to_string(),
            author: "Author".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let note = UNote::new("ko.e.s", "C4", 1000.0, 480.0);
        // Testando com PhonemizerMode::None (o padrão ao abrir o software!)
        let phones_none = JapanesePhonemizer::apply_phonemizer(&[note.clone()], &vb, PhonemizerMode::None);
        assert_eq!(phones_none.len(), 3);
        assert_eq!(phones_none[0].resolved_alias.as_deref(), Some("- こ"));
        assert_eq!(phones_none[1].resolved_alias.as_deref(), Some("o え"));
        assert_eq!(phones_none[2].resolved_alias.as_deref(), Some("e さ"));
        assert!(phones_none[2].coda_truncation);

        // Testando com PhonemizerMode::VCV
        let phones_vcv = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::VCV);
        assert_eq!(phones_vcv.len(), 3);
        assert_eq!(phones_vcv[0].resolved_alias.as_deref(), Some("- こ"));
        assert_eq!(phones_vcv[1].resolved_alias.as_deref(), Some("o え"));
        assert_eq!(phones_vcv[2].resolved_alias.as_deref(), Some("e さ"));
        assert!(phones_vcv[2].coda_truncation);
    }
}
