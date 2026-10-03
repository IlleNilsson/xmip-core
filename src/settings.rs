//! What a technology can be configured with, declared once by the technology.
//!
//! ADR-0064, amendment 2026-09-26: *every technology declares its own
//! settings — names, kinds, defaults, what is required, what each means — in
//! its own crate*, and that one declaration validates a Location's TOML at
//! start, fills the developer's form, documents the technology and is what
//! the technology reads its own settings through. So a technology never
//! parses a setting by hand, and nothing above it — `configure`, the language
//! server, the desktop editor — keeps a list of what a technology takes.
//!
//! The shape is plain data, `const` all the way down, so a technology writes
//! it as a `const` beside its code:
//!
//! ```
//! use xmip_core::settings::{Applies, Fixed, Kind, Presence, Setting, Settings};
//!
//! const SETTINGS: &Settings = &Settings {
//!     technology: "xmip-core-transport-example",
//!     settings: &[Setting {
//!         name: "topic",
//!         kind: Kind::Text,
//!         presence: Presence::Required,
//!         meaning: "The topic a Location reads from or writes to.",
//!         applies: Applies::Both,
//!     }, Setting {
//!         name: "wait",
//!         kind: Kind::Duration,
//!         presence: Presence::Default(Fixed::Text("5s")),
//!         meaning: "How long one receive waits for a record.",
//!         applies: Applies::Receive,
//!     }],
//! };
//! ```
//!
//! The capability a technology belongs to says how it reads them — a
//! transport's `transport::Configured`, a contract's `ContractFactory` — and
//! both read through [`Settings::read`]. It lives here, the lowest crate every
//! capability already reaches, because the shape is the same for every
//! technology of every capability (ADR-0044: shared code goes up).

mod read;
mod refusal;

pub use read::{Given, Read, duration};
pub use refusal::{Refusal, Refused};

/// One technology's settings: every setting a Location may give it, beyond
/// the Location's own `address`. A technology with no settings declares an
/// empty list, and says so by doing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Settings {
    /// The technology's module name, as a Location's configuration names it:
    /// `xmip-core-transport-file`, `xmip-core-contract-json-schema`.
    pub technology: &'static str,
    /// What it takes, in the order a form shows them.
    pub settings: &'static [Setting],
}

/// One setting a technology takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Setting {
    /// Its key in the Location's settings table, `snake_case` as TOML's
    /// keys are in Xmip's documents.
    pub name: &'static str,
    /// What kind of value it is.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub kind: Kind,
    /// Whether it must be given, may be, or falls back to a default.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub presence: Presence,
    /// What it means, in one sentence a developer reads in a form or a hover.
    pub meaning: &'static str,
    /// Which Locations it applies to.
    pub applies: Applies,
}

/// What kind of value a setting holds. Each is written in TOML as the kind
/// it is: a string, an integer or a boolean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "kind", rename_all = "kebab-case")
)]
pub enum Kind {
    /// Any text.
    Text,
    /// A whole number between `minimum` and `maximum`, both included.
    Integer {
        /// The least it may be.
        minimum: i64,
        /// The most it may be.
        maximum: i64,
    },
    /// `true` or `false`.
    Boolean,
    /// A length of time as text: a whole number and its unit, `ms`, `s`, `m`
    /// or `h` — `250ms`, `30s`, `5m`.
    Duration,
    /// Where something is, in the technology's own terms: a host and port, a
    /// URL, a path, a device. Never empty.
    Address,
    /// The name of a secret, never the secret: what `credentials` is on a
    /// Location, for a second one the technology needs. Never empty.
    Secret,
    /// One of the words listed, exactly as written.
    Choice {
        /// The words it may be.
        choices: &'static [&'static str],
    },
}

impl Kind {
    /// The kind's word, as a refusal and a form name it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Integer { .. } => "integer",
            Self::Boolean => "boolean",
            Self::Duration => "duration",
            Self::Address => "address",
            Self::Secret => "secret",
            Self::Choice { .. } => "choice",
        }
    }
}

/// Whether a setting must be given. A required setting has no default, and
/// a setting with a default is never missing: the two cannot both be said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "presence", content = "default", rename_all = "kebab-case")
)]
pub enum Presence {
    /// A Location that leaves it out is refused.
    Required,
    /// A Location may leave it out, and the technology does without it.
    Optional,
    /// A Location that leaves it out has this value.
    Default(Fixed),
}

/// A default, written as TOML would write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(untagged))]
pub enum Fixed {
    /// A string: text, an address, a choice.
    Text(&'static str),
    /// An integer.
    Integer(i64),
    /// A boolean.
    Boolean(bool),
    /// A duration, as the technology's own constant holds it, so the
    /// default is written once; it crosses as a duration's text, `5s`.
    #[cfg_attr(feature = "serde", serde(serialize_with = "duration_text"))]
    Duration(core::time::Duration),
}

#[cfg(feature = "serde")]
fn duration_text<S: serde::Serializer>(
    duration: &core::time::Duration,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&read::duration_written(*duration))
}

/// Which Locations a setting applies to: a Receive Location's, a Send
/// Location's, or both. One technology serves both directions (ADR-0010), and
/// a setting only one of them reads is refused on the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "kebab-case")
)]
pub enum Applies {
    /// A Receive Location's.
    Receive,
    /// A Send Location's.
    Send,
    /// Either.
    Both,
}

impl Applies {
    /// Whether a setting that applies `self` is read on a Location that is
    /// `side`. `Both` as the side reads every setting: the technology's own
    /// view, before it knows which end it is.
    #[must_use]
    pub const fn covers(self, side: Self) -> bool {
        matches!(
            (self, side),
            (Self::Both, _)
                | (_, Self::Both)
                | (Self::Receive, Self::Receive)
                | (Self::Send, Self::Send)
        )
    }

    /// The side's word, as a refusal names it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Receive => "receive",
            Self::Send => "send",
            Self::Both => "both",
        }
    }
}

impl Settings {
    /// A technology that takes nothing beyond the Location's address.
    #[must_use]
    pub const fn none(technology: &'static str) -> Self {
        Self {
            technology,
            settings: &[],
        }
    }

    /// The setting called `name`, where the technology declares one.
    #[must_use]
    pub fn setting(&self, name: &str) -> Option<&'static Setting> {
        self.settings.iter().find(|setting| setting.name == name)
    }

    /// The settings a Location on `side` may give, in declared order.
    pub fn applying(&self, side: Applies) -> impl Iterator<Item = &'static Setting> {
        self.settings
            .iter()
            .filter(move |setting| setting.applies.covers(side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &Settings = &Settings {
        technology: "xmip-core-transport-example",
        settings: &[
            Setting {
                name: "topic",
                kind: Kind::Text,
                presence: Presence::Required,
                meaning: "The topic.",
                applies: Applies::Both,
            },
            Setting {
                name: "group",
                kind: Kind::Text,
                presence: Presence::Optional,
                meaning: "The consumer group.",
                applies: Applies::Receive,
            },
        ],
    };

    #[test]
    fn a_side_sees_what_applies_to_it() {
        let receive: alloc::vec::Vec<_> = EXAMPLE.applying(Applies::Receive).collect();
        assert_eq!(receive.len(), 2);
        let send: alloc::vec::Vec<_> = EXAMPLE.applying(Applies::Send).collect();
        assert_eq!(send.len(), 1);
        assert_eq!(send[0].name, "topic");
    }

    #[test]
    fn the_declaration_crosses_as_flat_json() {
        let port = Setting {
            name: "port",
            kind: Kind::Integer {
                minimum: 1,
                maximum: 65_535,
            },
            presence: Presence::Default(Fixed::Integer(9092)),
            meaning: "The port.",
            applies: Applies::Send,
        };
        let json = serde_json::to_string(&port).expect("serialises");
        assert_eq!(
            json,
            "{\"name\":\"port\",\"kind\":\"integer\",\"minimum\":1,\"maximum\":65535,\
             \"presence\":\"default\",\"default\":9092,\"meaning\":\"The port.\",\
             \"applies\":\"send\"}"
        );
        let json = serde_json::to_string(EXAMPLE).expect("serialises");
        assert!(json.contains("\"presence\":\"required\""), "{json}");
    }

    #[test]
    fn a_setting_is_found_by_its_name() {
        assert_eq!(EXAMPLE.setting("group").map(|s| s.kind), Some(Kind::Text));
        assert!(EXAMPLE.setting("nothing").is_none());
        assert!(Settings::none("x").settings.is_empty());
    }
}
