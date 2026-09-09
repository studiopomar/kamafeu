use crate::config::AppLanguage;

impl AppLanguage {
    /// Selects localized text based on the active language.
    #[inline]
    pub fn tr<'a>(&self, pt: &'a str, en: &'a str) -> &'a str {
        match self {
            AppLanguage::PtBr => pt,
            AppLanguage::EnUs => en,
        }
    }

    #[inline]
    pub fn is_en(&self) -> bool {
        matches!(self, AppLanguage::EnUs)
    }

    #[inline]
    pub fn is_pt(&self) -> bool {
        matches!(self, AppLanguage::PtBr)
    }
}
