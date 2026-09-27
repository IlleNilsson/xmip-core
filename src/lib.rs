#![forbid(unsafe_code)]
#![no_std]

//! Core Xmip identifiers, shared types and stable public contracts.

//! The identity vocabulary lives here rather than in `xmip-core-party` because
//! the three gates need it and none of them may depend on the Party.
//! `architecture.toml` gives `xmip-core-identify`, `xmip-core-authenticate` and
//! `xmip-core-authorize` no dependency on `xmip-core-party`, and it is right:
//! authenticating a credential is not the same as knowing whose it is.
//!
//! Three accepted decisions meet in it. ADR-0019 makes the Party the identity
//! holder in both directions. ADR-0022 classifies every identity by how it is
//! proven and forbids two identity contexts from sharing a host process.
//! ADR-0009 keeps roles out of it: a Party is recognised, a role is granted.
//!
//! It was one 705-line file called `identity.rs` until 2026-08-29. Six subjects
//! shared that name, and two other crates had a file called `identity.rs`
//! holding neither of them — `rust-style.md` section 5.

// no_std, so the foundation can be sliced onto a microcontroller.
// deployment-model.md puts an IoT device at one end of the deployment range,
// and a Cortex-M class board has no std to link against. Every type is a
// value, an identifier or a trait, and all I/O lives in the transport and
// persist modules by design; the one reading of the operating system here,
// the system clock, is behind the `std` feature, as the UUIDv7 generator is.
//
// alloc rather than core alone, because String and Vec are what an identifier
// and a fact map are made of. A target with no allocator cannot use this
// crate, and that is a smaller claim than needing an operating system.
extern crate alloc;

// std with the `std` feature, for the system clock, and in tests, where
// thread::sleep is how the UUIDv7 timestamp test advances the clock.
#[cfg(any(test, feature = "std"))]
extern crate std;

mod clock;
mod credential;
mod direction;
mod error;
mod established;
mod id;
mod isolation;
mod phase;
mod purpose;
mod scalar;
mod severity;

pub mod mechanism;
/// What a technology can be configured with, declared once by the technology
/// (ADR-0064, amendment 2026-09-26).
pub mod settings;

#[cfg(feature = "std")]
pub use clock::SystemClock;
pub use clock::{Clock, NANOS_A_SECOND};
pub use credential::CredentialRef;
pub use direction::{Arriving, Departing};
pub use error::Failure;
pub use established::Established;
#[cfg(feature = "uuid-v7")]
pub use id::UuidV7Generator;
pub use id::{
    ArtifactId, AuditId, ClusterId, EventId, ExecutionId, IdGenerator, JourneyId, MessageId,
    NodeId, PartyId, SectionId, StreamId,
};
pub use isolation::IdentityContext;
pub use mechanism::{Assurance, IdentityClass, Layer, Mechanism};
pub use phase::ExecutionPhase;
pub use purpose::Purpose;
pub use scalar::ScalarValue;
pub use severity::Severity;
