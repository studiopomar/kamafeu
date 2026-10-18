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
