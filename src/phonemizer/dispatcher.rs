use super::{JapanesePhonemizer, PhonemizerImplementation, PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;

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
