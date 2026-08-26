#![forbid(unsafe_code)]

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
    };
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
