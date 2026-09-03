//! A way of proving identity, and the three facts that follow from it.
//!
//! **None of `class`, `layer` or `assurance` is configurable.** ADR-0022
//! clause 1: an operator who could declare an API key `highAssurance` would
//! have declared away the only thing the classification is for. ADR-0019
//! clause 5 says the same of the layer — feasibility is a property of the
//! technology, and a file dropped in a folder carries no transport credential
//! however the TOML reads.
//!
//! A mechanism is declared by the module that implements it, through
//! [`Mechanism::declare`], and is never built from configuration. The ones Xmip
//! implements itself are in [`declared`].

use alloc::string::String;
use core::fmt;

mod declared;
pub use declared::*;

/// Where an identity travels.
///
/// The line, from ADR-0019 clause 5, needs no other concept: anything Xmip can
/// read *before* Message creation is transport, anything requiring the Message
/// to exist is message.
///
/// RFC 9421 HTTP Message Signatures is the case that proves the rule is about
/// readability rather than about what is proven. It signs selected headers and
/// the body — so what it proves is content integrity — but it is read before
/// Message creation, and Xmip therefore treats it as transport.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Layer {
    /// Who opened the connection.
    Transport,
    /// On whose behalf the content was produced.
    Message,
}

/// How an identity is proven. ADR-0022 clause 1.
///
/// A property of the proof, not of who holds it and not of how much it is
/// trusted. A Party may hold identities in several classes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdentityClass {
    /// Cryptographic proof bound to a named principal.
    HighAssurance,
    /// Asserted by a trusted third party.
    Federated,
    /// A secret both sides hold.
    SharedSecret,
    /// No identity is claimed. A context like any other, and the cheapest to
    /// isolate — also the one most likely to be reached by an attacker.
    Anonymous,
}

impl fmt::Display for IdentityClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::HighAssurance => "highAssurance",
            Self::Federated => "federated",
            Self::SharedSecret => "sharedSecret",
            Self::Anonymous => "anonymous",
        })
    }
}

/// Whether a mechanism can settle the second gate, or only the first.
///
/// ADR-0019 clause 2 orders the gates: identity, then authentication, then
/// authorization. Most of the estate does both boxes at once — a client
/// certificate names a Party *and* proves it. Some mechanisms only ever do the
/// first.
///
/// X12's ISA06 and EDIFACT's UNB S002 name the counterparty and carry no
/// cryptography whatever. `identity-by-technology.md` calls treating them as
/// authentication "the classic B2B mistake": they are enough to route and bill
/// a Journey, and proving the claim is the transport's job, or AS2's. HL7's
/// MSH-3 is the same shape in healthcare, and MLLP gives it nothing to lean on.
///
/// **This is what the protocol provides, not a choice Xmip makes.** X12 was
/// standardised without cryptography and HL7 v2 travels over a framing protocol
/// with no security whatever. Xmip covers them because they are most of the
/// installed base, and it covers them honestly — recording that a name was
/// claimed rather than that a claim was proven.
///
/// This value does not gate anything on its own. Every arrival is authenticated
/// and authorized at runtime regardless, and authorization is where "may a
/// Party recognised only by an ISA06 send this contract on this Path" is
/// answered. The distinction exists so that question has something true to read.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Assurance {
    /// Names a Party. Proves nothing.
    Identifies,
    /// Names a Party and proves the claim.
    Authenticates,
}

/// A way of proving identity, and the three facts that follow from it.
///
/// **None of `class`, `layer` or `assurance` is configurable.** ADR-0022
/// clause 1: an operator who could declare an API key `highAssurance` would
/// have declared away the only thing the classification is for. ADR-0019
/// clause 5 says the same of the layer — feasibility is a property of the
/// technology, and a file dropped in a folder carries no transport credential
/// however the TOML reads.
///
/// A mechanism is declared by the module that implements it, through
/// [`Mechanism::declare`], and is never built from configuration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Mechanism {
    name: String,
    class: IdentityClass,
    layer: Layer,
    assurance: Assurance,
}

impl Mechanism {
    /// Declared by the module that implements the mechanism.
    ///
    /// Not reachable from TOML, which is the point. A provider adding
    /// `xmip-acme-authenticate-scim` states its own class, layer and assurance
    /// because it is the only code that knows them; an operator deploying it
    /// does not get to disagree.
    #[must_use]
    pub fn declare(
        name: impl Into<String>,
        class: IdentityClass,
        layer: Layer,
        assurance: Assurance,
    ) -> Self {
        Self {
            name: name.into(),
            class,
            layer,
            assurance,
        }
    }

    /// The last segment of the repository that implements it —
    /// `xmip-core-authenticate-<name>`, or `xmip-core-identify-<name>`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn class(&self) -> IdentityClass {
        self.class
    }

    #[must_use]
    pub const fn layer(&self) -> Layer {
        self.layer
    }

    #[must_use]
    pub const fn assurance(&self) -> Assurance {
        self.assurance
    }

    /// Whether this mechanism can settle the authentication gate on its own.
    #[must_use]
    pub const fn authenticates(&self) -> bool {
        matches!(self.assurance, Assurance::Authenticates)
    }
}

