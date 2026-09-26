//! Why a Location's settings were refused, each naming the technology and
//! the setting, so a developer reads what to change without looking it up.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::Applies;

/// One setting refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The technology declares no setting of this name.
    Unknown {
        technology: &'static str,
        setting: String,
    },
    /// The technology declares it for the other side only.
    OtherSide {
        technology: &'static str,
        setting: &'static str,
        applies: Applies,
    },
    /// The value is not of the declared kind, or not within it.
    Kind {
        technology: &'static str,
        setting: &'static str,
        expected: &'static str,
        reason: String,
    },
    /// A required setting was left out.
    Missing {
        technology: &'static str,
        setting: &'static str,
    },
}

impl Refusal {
    /// The setting refused, as the Location wrote or should have written it.
    #[must_use]
    pub fn setting(&self) -> &str {
        match self {
            Self::Unknown { setting, .. } => setting,
            Self::OtherSide { setting, .. }
            | Self::Kind { setting, .. }
            | Self::Missing { setting, .. } => setting,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown {
                technology,
                setting,
            } => write!(f, "{technology} has no setting {setting:?}"),
            Self::OtherSide {
                technology,
                setting,
                applies,
            } => write!(
                f,
                "{technology} reads {setting:?} on a {} Location only",
                applies.word()
            ),
            Self::Kind {
                technology,
                setting,
                expected,
                reason,
            } => write!(
                f,
                "{technology} reads {setting:?} as {expected}, and {reason}"
            ),
            Self::Missing {
                technology,
                setting,
            } => write!(f, "{technology} requires {setting:?}, and it is missing"),
        }
    }
}

/// Every refusal one reading found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused(pub Vec<Refusal>);

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, refusal) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{refusal}")?;
        }
        Ok(())
    }
}

impl core::error::Error for Refused {}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn every_refusal_names_the_technology_and_the_setting() {
        let refused = Refused(Vec::from([
            Refusal::Missing {
                technology: "xmip-core-transport-kafka",
                setting: "topic",
            },
            Refusal::OtherSide {
                technology: "xmip-core-transport-kafka",
                setting: "group",
                applies: Applies::Receive,
            },
        ]));
        assert_eq!(
            format!("{refused}"),
            "xmip-core-transport-kafka requires \"topic\", and it is missing; \
             xmip-core-transport-kafka reads \"group\" on a receive Location only"
        );
        assert_eq!(refused.0[1].setting(), "group");
    }
}
