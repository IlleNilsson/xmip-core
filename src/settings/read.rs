//! A Location's settings, read through the technology's declaration.
//!
//! The one reading: `configure` calls it to validate a Location at start,
//! and the technology calls it to build itself, so what is refused and what
//! a technology is given cannot disagree.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::time::Duration;

use super::{Applies, Fixed, Kind, Presence, Refusal, Refused, Settings};

/// A value as a Location wrote it, before it is read: what TOML holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Given {
    /// A string.
    Text(String),
    /// An integer.
    Integer(i64),
    /// A boolean.
    Boolean(bool),
    /// Anything else TOML can say — an array, a table, a float, a date —
    /// by the word for it. No setting is one, so it is always refused.
    Other(&'static str),
}

impl Given {
    fn word(&self) -> &'static str {
        match self {
            Self::Text(_) => "string",
            Self::Integer(_) => "integer",
            Self::Boolean(_) => "boolean",
            Self::Other(word) => word,
        }
    }
}

/// One value, read and held as its kind.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Held {
    Text(String),
    Integer(i64),
    Boolean(bool),
    Duration(Duration),
}

/// A Location's settings, read: every value the declaration let through, as
/// its kind, and every default the Location left to the declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    technology: &'static str,
    values: Vec<(&'static str, Held)>,
}

impl Settings {
    /// Read what a Location on `side` gave (`Applies::Both` for the
    /// technology's own view), refusing every setting the declaration does
    /// not have, has for the other side only, or has as another kind, and
    /// every required one left out — each naming this technology and the
    /// setting. Defaults are filled in.
    ///
    /// # Errors
    /// Every refusal at once, in the order found.
    pub fn read(&self, side: Applies, given: &[(String, Given)]) -> Result<Read, Refused> {
        let mut refusals = Vec::new();
        let mut values = Vec::new();

        for (name, value) in given {
            let Some(setting) = self.setting(name) else {
                refusals.push(Refusal::Unknown {
                    technology: self.technology,
                    setting: name.clone(),
                });
                continue;
            };
            if !setting.applies.covers(side) {
                refusals.push(Refusal::OtherSide {
                    technology: self.technology,
                    setting: setting.name,
                    applies: setting.applies,
                });
                continue;
            }
            match held(setting.kind, value) {
                Ok(held) => values.push((setting.name, held)),
                Err(reason) => refusals.push(Refusal::Kind {
                    technology: self.technology,
                    setting: setting.name,
                    expected: setting.kind.word(),
                    reason,
                }),
            }
        }

        for setting in self.applying(side) {
            if values.iter().any(|(name, _)| *name == setting.name)
                || given.iter().any(|(name, _)| name == setting.name)
            {
                continue;
            }
            match setting.presence {
                Presence::Required => refusals.push(Refusal::Missing {
                    technology: self.technology,
                    setting: setting.name,
                }),
                Presence::Optional => {}
                Presence::Default(fixed) => match held(setting.kind, &fixed.given()) {
                    Ok(held) => values.push((setting.name, held)),
                    // A default its own kind refuses is the declaration's
                    // fault; each technology's test reads its defaults.
                    Err(reason) => refusals.push(Refusal::Kind {
                        technology: self.technology,
                        setting: setting.name,
                        expected: setting.kind.word(),
                        reason,
                    }),
                },
            }
        }

        if refusals.is_empty() {
            Ok(Read {
                technology: self.technology,
                values,
            })
        } else {
            Err(Refused(refusals))
        }
    }
}

impl Settings {
    /// What is wrong with the declaration itself, each said in a sentence:
    /// a name that is not `snake_case` or is declared twice, a meaning that
    /// is not a sentence, a range that is empty, a choice of nothing, a
    /// default its own kind refuses. Empty for a sound declaration; every
    /// technology's tests hold its own to it.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (index, setting) in self.settings.iter().enumerate() {
            let name = setting.name;
            let snake = !name.is_empty()
                && name.starts_with(|c: char| c.is_ascii_lowercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !snake {
                problems.push(alloc::format!("{name:?} is not a snake_case name"));
            }
            if self.settings[..index].iter().any(|s| s.name == name) {
                problems.push(alloc::format!("{name:?} is declared twice"));
            }
            if !setting.meaning.ends_with('.') || setting.meaning.len() < 8 {
                problems.push(alloc::format!("{name:?} has no one-sentence meaning"));
            }
            match setting.kind {
                Kind::Integer { minimum, maximum } if minimum > maximum => {
                    problems.push(alloc::format!("{name:?} has an empty range"));
                }
                Kind::Choice { choices: [] } => {
                    problems.push(alloc::format!("{name:?} is a choice of nothing"));
                }
                _ => {}
            }
            if let Presence::Default(fixed) = setting.presence
                && let Err(reason) = held(setting.kind, &fixed.given())
            {
                problems.push(alloc::format!(
                    "{name:?} defaults to what it refuses: {reason}"
                ));
            }
        }
        problems
    }
}

impl Fixed {
    /// The default as a Location would have written it.
    #[must_use]
    pub fn given(self) -> Given {
        match self {
            Self::Text(text) => Given::Text(text.to_string()),
            Self::Integer(value) => Given::Integer(value),
            Self::Boolean(value) => Given::Boolean(value),
            Self::Duration(duration) => Given::Text(duration_written(duration)),
        }
    }
}

/// A duration as a Location writes one, in the largest unit that holds it
/// whole: `2h`, `5m`, `30s`, `250ms`.
#[must_use]
pub(super) fn duration_written(duration: Duration) -> String {
    let millis = duration.as_millis();
    let (number, unit) = match millis {
        0 => (0, "s"),
        _ if millis.is_multiple_of(3_600_000) => (millis / 3_600_000, "h"),
        _ if millis.is_multiple_of(60_000) => (millis / 60_000, "m"),
        _ if millis.is_multiple_of(1000) => (millis / 1000, "s"),
        _ => (millis, "ms"),
    };
    alloc::format!("{number}{unit}")
}

/// `value` as `kind` holds it, or why it is not one.
fn held(kind: Kind, value: &Given) -> Result<Held, String> {
    let wrong = || alloc::format!("a {} was given", value.word());
    match (kind, value) {
        (Kind::Text, Given::Text(text)) => Ok(Held::Text(text.clone())),
        (Kind::Address | Kind::Secret, Given::Text(text)) => {
            if text.trim().is_empty() {
                Err("it is empty".to_string())
            } else {
                Ok(Held::Text(text.clone()))
            }
        }
        (Kind::Choice { choices }, Given::Text(text)) => {
            if choices.contains(&text.as_str()) {
                Ok(Held::Text(text.clone()))
            } else {
                Err(alloc::format!(
                    "{text:?} is not one of {}",
                    choices.join(", ")
                ))
            }
        }
        (Kind::Duration, Given::Text(text)) => duration(text).map(Held::Duration),
        (Kind::Integer { minimum, maximum }, Given::Integer(number)) => {
            if (minimum..=maximum).contains(number) {
                Ok(Held::Integer(*number))
            } else {
                Err(alloc::format!(
                    "{number} is not from {minimum} to {maximum}"
                ))
            }
        }
        (Kind::Boolean, Given::Boolean(flag)) => Ok(Held::Boolean(*flag)),
        _ => Err(wrong()),
    }
}

/// A duration's text, a whole number and its unit: `250ms`, `30s`, `5m`,
/// `1h`. The one reading of a configured duration: a setting's, and a
/// Send Port's retry backoff in `xmip-core-configure`.
///
/// # Errors
/// The text in words, when it is not a whole number and one of the units.
pub fn duration(text: &str) -> Result<Duration, String> {
    let digits = text.trim().bytes().take_while(u8::is_ascii_digit).count();
    let (number, unit) = text.trim().split_at(digits);
    let refused = || alloc::format!("{text:?} is not a number and ms, s, m or h");
    let number: u64 = number.parse().map_err(|_| refused())?;
    match unit {
        "ms" => Ok(Duration::from_millis(number)),
        "s" => Ok(Duration::from_secs(number)),
        "m" => Ok(Duration::from_secs(number.saturating_mul(60))),
        "h" => Ok(Duration::from_secs(number.saturating_mul(3600))),
        _ => Err(refused()),
    }
}

impl Read {
    fn value(&self, name: &str) -> Option<&Held> {
        self.values
            .iter()
            .find(|(held, _)| *held == name)
            .map(|(_, value)| value)
    }

    fn absent(&self, name: &str) -> ! {
        panic!(
            "{} reads {name:?} as always there; its declaration must make it \
             required or give it a default",
            self.technology
        )
    }

    /// A text, address, secret or choice setting, where it has a value.
    #[must_use]
    pub fn optional_text(&self, name: &str) -> Option<&str> {
        match self.value(name) {
            Some(Held::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// A text, address, secret or choice setting that is required or has a
    /// default.
    ///
    /// # Panics
    /// When the declaration lets it be absent: a defect in the technology,
    /// which its test of its own declaration finds.
    #[must_use]
    pub fn text(&self, name: &str) -> &str {
        self.optional_text(name)
            .unwrap_or_else(|| self.absent(name))
    }

    /// An integer setting, where it has a value.
    #[must_use]
    pub fn optional_integer(&self, name: &str) -> Option<i64> {
        match self.value(name) {
            Some(Held::Integer(number)) => Some(*number),
            _ => None,
        }
    }

    /// An integer setting that is required or has a default.
    ///
    /// # Panics
    /// As [`Read::text`].
    #[must_use]
    pub fn integer(&self, name: &str) -> i64 {
        self.optional_integer(name)
            .unwrap_or_else(|| self.absent(name))
    }

    /// A boolean setting, where it has a value.
    #[must_use]
    pub fn optional_boolean(&self, name: &str) -> Option<bool> {
        match self.value(name) {
            Some(Held::Boolean(flag)) => Some(*flag),
            _ => None,
        }
    }

    /// A boolean setting that is required or has a default.
    ///
    /// # Panics
    /// As [`Read::text`].
    #[must_use]
    pub fn boolean(&self, name: &str) -> bool {
        self.optional_boolean(name)
            .unwrap_or_else(|| self.absent(name))
    }

    /// A duration setting, where it has a value.
    #[must_use]
    pub fn optional_duration(&self, name: &str) -> Option<Duration> {
        match self.value(name) {
            Some(Held::Duration(duration)) => Some(*duration),
            _ => None,
        }
    }

    /// A duration setting that is required or has a default.
    ///
    /// # Panics
    /// As [`Read::text`].
    #[must_use]
    pub fn duration(&self, name: &str) -> Duration {
        self.optional_duration(name)
            .unwrap_or_else(|| self.absent(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Setting;

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
                name: "port",
                kind: Kind::Integer {
                    minimum: 1,
                    maximum: 65_535,
                },
                presence: Presence::Default(Fixed::Integer(9092)),
                meaning: "The port.",
                applies: Applies::Both,
            },
            Setting {
                name: "wait",
                kind: Kind::Duration,
                presence: Presence::Default(Fixed::Text("5s")),
                meaning: "How long a receive waits.",
                applies: Applies::Receive,
            },
            Setting {
                name: "acks",
                kind: Kind::Choice {
                    choices: &["none", "leader", "all"],
                },
                presence: Presence::Optional,
                meaning: "Who acknowledges a send.",
                applies: Applies::Send,
            },
        ],
    };

    fn given(pairs: &[(&str, Given)]) -> Vec<(String, Given)> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn what_is_given_is_read_and_defaults_are_filled() {
        let read = EXAMPLE
            .read(
                Applies::Receive,
                &given(&[("topic", Given::Text("orders".into()))]),
            )
            .expect("reads");
        assert_eq!(read.text("topic"), "orders");
        assert_eq!(read.integer("port"), 9092);
        assert_eq!(read.duration("wait"), Duration::from_secs(5));
        assert_eq!(read.optional_text("acks"), None);
    }

    #[test]
    fn unknown_wrong_kind_and_missing_are_each_refused_by_name() {
        let refused = EXAMPLE
            .read(
                Applies::Send,
                &given(&[
                    ("colour", Given::Text("lime".into())),
                    ("port", Given::Text("9092".into())),
                    ("wait", Given::Text("1s".into())),
                    ("acks", Given::Text("most".into())),
                ]),
            )
            .expect_err("refused");
        let said = alloc::format!("{refused}");
        assert_eq!(refused.0.len(), 5, "{said}");
        for word in [
            "colour",
            "port",
            "wait",
            "acks",
            "topic",
            "xmip-core-transport-example",
        ] {
            assert!(said.contains(word), "names {word}: {said}");
        }
    }

    #[test]
    fn a_sound_declaration_has_no_problems_and_an_unsound_one_says_why() {
        assert!(EXAMPLE.problems().is_empty(), "{:?}", EXAMPLE.problems());
        let unsound = Settings {
            technology: "x",
            settings: &[
                Setting {
                    name: "Wait",
                    kind: Kind::Duration,
                    presence: Presence::Default(Fixed::Text("soon")),
                    meaning: "",
                    applies: Applies::Both,
                },
                Setting {
                    name: "Wait",
                    kind: Kind::Choice { choices: &[] },
                    presence: Presence::Optional,
                    meaning: "Which one it is.",
                    applies: Applies::Both,
                },
            ],
        };
        assert_eq!(unsound.problems().len(), 6, "{:?}", unsound.problems());
    }

    #[test]
    fn a_duration_default_is_written_in_its_largest_whole_unit() {
        assert_eq!(duration_written(Duration::from_secs(7200)), "2h");
        assert_eq!(duration_written(Duration::from_secs(300)), "5m");
        assert_eq!(duration_written(Duration::from_millis(1500)), "1500ms");
        let written = duration_written(Duration::from_secs(30));
        assert_eq!(duration(&written), Ok(Duration::from_secs(30)));
    }

    #[test]
    fn a_duration_is_a_number_and_its_unit() {
        assert_eq!(duration("250ms"), Ok(Duration::from_millis(250)));
        assert_eq!(duration("2m"), Ok(Duration::from_secs(120)));
        assert!(duration("5").is_err());
        assert!(duration("five s").is_err());
    }

    #[test]
    fn a_range_and_an_empty_address_are_refused() {
        assert!(
            held(
                Kind::Integer {
                    minimum: 1,
                    maximum: 9
                },
                &Given::Integer(10)
            )
            .is_err()
        );
        assert!(held(Kind::Address, &Given::Text(" ".into())).is_err());
        assert!(held(Kind::Boolean, &Given::Other("array")).is_err());
    }
}
