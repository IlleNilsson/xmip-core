//! How much a record matters to whoever reads it.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Information,
    Warning,
    Error,
}

impl Severity {
    /// Every severity, least first.
    pub const ALL: [Self; 3] = [Self::Information, Self::Warning, Self::Error];

    /// Its word, as the enum names it: what Xmip Storage's `audit.severity`
    /// column keeps (proposed 2026-10-09).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Information => "Information",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }

    /// The severity `word` names, as [`Severity::word`] writes it.
    #[must_use]
    pub fn worded(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|severity| severity.word() == word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_severity_s_word_is_its_variant_s_name_and_names_it_back() {
        for severity in Severity::ALL {
            assert_eq!(Severity::worded(severity.word()), Some(severity));
        }
        assert_eq!(Severity::Error.word(), "Error");
    }
}
