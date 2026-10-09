//! Where an execution stands.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionPhase {
    Begin,
    Execute,
    Finished,
    Failure,
}

impl ExecutionPhase {
    /// Every phase, in lifecycle order.
    pub const ALL: [Self; 4] = [Self::Begin, Self::Execute, Self::Finished, Self::Failure];

    /// Its word, as the enum names it: what Xmip Storage's `audit.phase`
    /// column keeps (proposed 2026-10-09).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Begin => "Begin",
            Self::Execute => "Execute",
            Self::Finished => "Finished",
            Self::Failure => "Failure",
        }
    }

    /// The phase `word` names, as [`ExecutionPhase::word`] writes it.
    #[must_use]
    pub fn worded(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|phase| phase.word() == word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_phase_s_word_is_its_variant_s_name_and_names_it_back() {
        for phase in ExecutionPhase::ALL {
            assert_eq!(ExecutionPhase::worded(phase.word()), Some(phase));
        }
        assert_eq!(ExecutionPhase::Failure.word(), "Failure");
    }
}
