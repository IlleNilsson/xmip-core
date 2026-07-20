#![forbid(unsafe_code)]

use core::fmt;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u128);

        impl $name {
            pub const fn new(value: u128) -> Self { Self(value) }
            pub const fn value(self) -> u128 { self.0 }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:032x}", self.0)
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

pub trait IdGenerator: Send + Sync {
    fn next_u128(&self) -> u128;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_stable_values() {
        let id = MessageId::new(42);
        assert_eq!(id.value(), 42);
        assert_eq!(id.to_string().len(), 32);
    }
}
