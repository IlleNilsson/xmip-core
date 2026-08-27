#![forbid(unsafe_code)]

//! Core Xmip identifiers, shared types and stable public contracts.

pub mod identity;

pub use identity::{
    mechanism, Arriving, Assurance, CredentialRef, Departing, Established, IdentityClass,
    IdentityContext, Layer, Mechanism, Purpose,
};

use core::fmt;

/// Every Xmip identifier is a **UUIDv7 held as a `u128`**.
///
/// A UUID is 128 bits, so the newtype and the UUID are the same value in two
/// shapes. The newtype is what stops a `JourneyId` being passed where a
/// `MessageId` belongs; the UUIDv7 is what makes it sort by creation time.
///
/// v7 leads with a 48-bit Unix millisecond timestamp, so identifiers written in
/// sequence land in sequence. Against the RocksDB-style store in
/// `deployment-model.md` section 7 that is the difference between appending and
/// scattering, and it makes a range scan over identifiers a range scan over
/// time. RFC 9562.
///
/// `Ord` is derived over the `u128`, which is big-endian by value, so sorting
/// these sorts chronologically. That is deliberate and not incidental.
macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u128);

        impl $name {
            pub const fn new(value: u128) -> Self { Self(value) }
            pub const fn value(self) -> u128 { self.0 }
        }

        /// Canonical UUID form, 8-4-4-4-12.
        ///
        /// These are UUIDs, so they are shown as UUIDs. Bare hex would hide
        /// that from anyone matching a log line against a database row.
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let hex = format!("{:032x}", self.0);
                write!(
                    f,
                    "{}-{}-{}-{}-{}",
                    &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32]
                )
            }
        }

        impl std::str::FromStr for $name {
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

/// Read canonical UUID form back to the value behind it.
///
/// Accepts the hyphenated form only. A bare 32-character hex string would also
/// parse unambiguously, and is refused on purpose: accepting both means two
/// spellings of one identifier end up in configuration and neither is wrong.
fn parse_uuid(text: &str) -> Result<u128, String> {
    let trimmed = text.trim();

    let shaped = trimmed.len() == 36
        && trimmed.as_bytes()[8] == b'-'
        && trimmed.as_bytes()[13] == b'-'
        && trimmed.as_bytes()[18] == b'-'
        && trimmed.as_bytes()[23] == b'-';

    if !shaped {
        return Err(format!("'{trimmed}' is not a UUID in 8-4-4-4-12 form"));
    }

    let hex: String = trimmed.chars().filter(|c| *c != '-').collect();

    u128::from_str_radix(&hex, 16).map_err(|_| format!("'{trimmed}' is not hexadecimal"))
}

id_type!(StreamId);
id_type!(MessageId);
id_type!(JourneyId);
id_type!(SectionId);
id_type!(ArtifactId);
id_type!(ExecutionId);
id_type!(AuditId);
id_type!(NodeId);
id_type!(ClusterId);
id_type!(PartyId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Information,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionPhase {
    Begin,
    Execute,
    Finished,
    Failure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactRef {
    pub artifact_id: ArtifactId,
    pub artifact_type: &'static str,
    pub name: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionScope {
    pub execution_id: ExecutionId,
    pub journey_id: JourneyId,
    pub message_id: MessageId,
    pub artifact: ArtifactRef,
    pub node_id: Option<NodeId>,
    pub cluster_id: Option<ClusterId>,
}

pub trait Clock: Send + Sync {
    fn unix_timestamp_nanos(&self) -> i128;
}

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

    #[cfg(feature = "serde")]
    #[test]
    fn an_identifier_round_trips_as_its_canonical_text() {
        let id = JourneyId::new(0x0198_7cdf_1234_7abc_8def_0123_4567_89ab);

        let json = serde_json::to_string(&id).expect("serialize");
        let back: JourneyId = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(json, format!("\"{id}\""), "must be the text form, not a number");
        assert_eq!(back, id);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_bare_hex_string_is_refused() {
        // Unambiguous, and still wrong: two spellings of one identifier would
        // both end up in configuration and neither would be the error.
        let result = "01987cdf12347abc8def0123456789ab".parse::<MessageId>();

        assert!(result.is_err(), "got: {result:?}");
    }

    #[test]
    fn identifiers_are_stable_values() {
        let id = MessageId::new(42);
        assert_eq!(id.value(), 42);
        // 32 hex digits plus four hyphens. Was 32 before identifiers became
        // UUIDs and Display became canonical form.
        assert_eq!(id.to_string().len(), 36);
        assert_eq!(id.to_string(), "00000000-0000-0000-0000-00000000002a");
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
