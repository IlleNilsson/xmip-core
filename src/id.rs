//! Every Xmip identifier, and the one generator that mints them.
//!
//! Every Xmip identifier is a **UUIDv7 held as a `u128`**.
//!
//! A UUID is 128 bits, so the newtype and the UUID are the same value in two
//! shapes. The newtype is what stops a `JourneyId` being passed where a
//! `MessageId` belongs; the UUIDv7 is what makes it sort by creation time.
//!
//! v7 leads with a 48-bit Unix millisecond timestamp, so identifiers written in
//! sequence land in sequence. Against the RocksDB-style store in
//! `deployment-model.md` section 7 that is the difference between appending and
//! scattering, and it makes a range scan over identifiers a range scan over
//! time. RFC 9562.
//!
//! `Ord` is derived over the `u128`, which is big-endian by value, so sorting
//! these sorts chronologically. That is deliberate and not incidental.

use alloc::format;
use alloc::string::String;
use core::fmt;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        /// An Xmip identifier: a UUIDv7 held as a `u128`, sorting by when it
        /// was minted, written and read in canonical 8-4-4-4-12 form.
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u128);

        impl $name {
            pub const fn new(value: u128) -> Self {
                Self(value)
            }
            pub const fn value(self) -> u128 {
                self.0
            }
        }

        /// Canonical UUID form, 8-4-4-4-12.
        ///
        /// These are UUIDs, so they are shown as UUIDs. Bare hex would hide
        /// that from anyone matching a log line against a database row.
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_uuid(f, self.0)
            }
        }

        impl core::str::FromStr for $name {
            type Err = String;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                parse_uuid(text).map(Self)
            }
        }

        /// Serialised as the canonical UUID string, never as the `u128`.
        ///
        /// TOML integers are 64-bit signed, so half of a `u128` cannot survive
        /// the trip. JSON has the same problem the moment JavaScript reads it.
        /// The text form round-trips everywhere and matches what `Display`
        /// puts in a log line.
        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.collect_str(self)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let text = String::deserialize(deserializer)?;
                text.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

/// Where the four hyphens of the canonical form stand.
const HYPHENS: [usize; 4] = [8, 13, 18, 23];

/// Write `value` in canonical form straight to the formatter: five groups of
/// hexadecimal, nothing allocated.
fn write_uuid(f: &mut fmt::Formatter<'_>, value: u128) -> fmt::Result {
    write!(
        f,
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        value >> 96,
        (value >> 80) & 0xffff,
        (value >> 64) & 0xffff,
        (value >> 48) & 0xffff,
        value & 0xffff_ffff_ffff
    )
}

/// Read canonical UUID form back to the value behind it.
///
/// Accepts the hyphenated form only, and in it only hexadecimal digits: a
/// sign, a space or a hyphen out of place is refused. A bare 32-character hex
/// string would also parse unambiguously, and is refused on purpose: accepting
/// both means two spellings of one identifier end up in configuration and
/// neither is wrong.
fn parse_uuid(text: &str) -> Result<u128, String> {
    let trimmed = text.trim();
    let bytes = trimmed.as_bytes();

    if bytes.len() != 36 || HYPHENS.iter().any(|&at| bytes[at] != b'-') {
        return Err(format!("'{trimmed}' is not a UUID in 8-4-4-4-12 form"));
    }

    let mut value = 0u128;
    for (at, &byte) in bytes.iter().enumerate() {
        if HYPHENS.contains(&at) {
            continue;
        }
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return Err(format!("'{trimmed}' is not hexadecimal")),
        };
        value = value << 4 | u128::from(digit);
    }

    Ok(value)
}

id_type!(StreamId);
id_type!(MessageId);
id_type!(JourneyId);
id_type!(SectionId);
id_type!(ArtifactId);
id_type!(ExecutionId);
id_type!(AuditId);
id_type!(
    /// An Event's identity (runtime-model section 17, ADR-0065): minted where
    /// the Event is raised, by the one generator, and carried as the wire
    /// event's `id`.
    EventId
);
id_type!(NodeId);
id_type!(ClusterId);
id_type!(PartyId);

/// Produces identifier values.
///
/// **Implementations must return UUIDv7.** The trait cannot enforce it, so it
/// is stated here and satisfied by [`UuidV7Generator`]. An implementation
/// returning random values still compiles and still works — and quietly
/// forfeits the sort locality the whole choice was made for.
pub trait IdGenerator: Send + Sync {
    fn next_u128(&self) -> u128;
}

/// The canonical [`IdGenerator`]. UUIDv7, per RFC 9562.
#[cfg(feature = "uuid-v7")]
#[derive(Clone, Copy, Debug, Default)]
pub struct UuidV7Generator;

#[cfg(feature = "uuid-v7")]
impl IdGenerator for UuidV7Generator {
    fn next_u128(&self) -> u128 {
        uuid::Uuid::now_v7().as_u128()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[cfg(feature = "serde")]
    #[test]
    fn an_identifier_round_trips_as_its_canonical_text() {
        let id = JourneyId::new(0x0198_7cdf_1234_7abc_8def_0123_4567_89ab);

        let json = serde_json::to_string(&id).expect("serialize");
        let back: JourneyId = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(
            json,
            format!("\"{id}\""),
            "must be the text form, not a number"
        );
        assert_eq!(back, id);
    }

    #[test]
    fn a_bare_hex_string_is_refused() {
        // Unambiguous, and still wrong: two spellings of one identifier would
        // both end up in configuration and neither would be the error.
        let result = "01987cdf12347abc8def0123456789ab".parse::<MessageId>();

        assert!(result.is_err(), "got: {result:?}");
    }

    #[test]
    fn a_sign_or_a_stray_character_is_not_hexadecimal() {
        // Stripping the hyphens and handing the rest to from_str_radix let a
        // leading sign through: "+0000000-…" read as zero.
        for text in [
            "+0000000-0000-0000-0000-00000000002a",
            "-0000000-0000-0000-0000-00000000002a",
            "00000000-0000-0000-0000-00000000002g",
            "00000000-0000-0000-0000 00000000002a",
        ] {
            assert!(text.parse::<MessageId>().is_err(), "{text}");
        }
        // Upper case is hexadecimal too.
        assert_eq!(
            "01987CDF-1234-7ABC-8DEF-0123456789AB".parse::<MessageId>(),
            Ok(MessageId::new(0x0198_7cdf_1234_7abc_8def_0123_4567_89ab))
        );
    }

    #[test]
    fn identifiers_are_stable_values() {
        let id = MessageId::new(42);
        assert_eq!(id.value(), 42);
        assert_eq!(id.to_string(), "00000000-0000-0000-0000-00000000002a");

        let every_group = MessageId::new(0x0198_7cdf_1234_7abc_8def_0123_4567_89ab);
        assert_eq!(
            every_group.to_string(),
            "01987cdf-1234-7abc-8def-0123456789ab"
        );
        assert_eq!(every_group.to_string().parse(), Ok(every_group));
    }

    #[cfg(feature = "uuid-v7")]
    #[test]
    fn generated_identifiers_are_uuid_v7_and_sort_by_time() {
        let generator = UuidV7Generator;

        let first = JourneyId::new(generator.next_u128());
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = JourneyId::new(generator.next_u128());

        // Version nibble sits at bits 76..79 — the 13th hex digit.
        let version = (first.value() >> 76) & 0xf;
        assert_eq!(version, 7, "identifiers must be UUIDv7");

        // The reason for v7: later means greater, so Ord is chronological.
        assert!(second > first, "v7 identifiers must sort by creation time");
    }
}
