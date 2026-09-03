//! How an identity came to be claimed. The first gate's own answer.
//!
//! Independent of how the Stream arrived. A pushed Stream can yield a detected
//! identity; a scheduled pickup can only yield an inferred one, because there
//! was nobody there to pass anything. ADR-0019 clause 8.

use core::fmt;

/// How an identity came to be claimed. The first gate's own answer.
///
/// Not how it is proven — that is [`Mechanism`] and [`Assurance`] — and not
/// where it travelled, which is [`Layer`]. This is the question an operator
/// asks six months later when a Journey is disputed: *why did Xmip think this
/// was partner-x?*
///
/// ```text
/// Passed     the sender presented it        a certificate, a token, a header
/// Inferred   the configuration says so      this folder, this schedule, this credential Xmip used
/// Detected   it was read out of what is there   ISA06, a signature, an envelope
/// ```
///
/// The three are independent of how the Stream arrived. A pushed Stream can
/// yield a detected identity — a partner posts an X12 interchange over plain
/// HTTP and the only name anywhere is in the envelope. A scheduled pickup can
/// only ever yield an inferred one, because there was nobody there to pass
/// anything.
///
/// **Inferred is not weak by definition and passed is not strong by
/// definition.** A drop folder reachable only over a dedicated line says more
/// than a bearer token pasted into a header. Strength is [`IdentityClass`] and
/// [`Assurance`]; this is provenance, and the two answer different questions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Established {
    /// The sender presented it. Somebody chose to send this value.
    Passed,
    /// The configuration is the identity. ADR-0019 clause 7: a partner drop
    /// folder is not an absence of identity, and neither is a schedule.
    ///
    /// Nothing was presented, so nothing can have been forged — and equally,
    /// anything that can reach the folder inherits the name.
    Inferred,
    /// Read out of what arrived. The Stream carried a name whether or not the
    /// sender meant it as a credential.
    Detected,
}

impl fmt::Display for Established {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Passed => "passed",
            Self::Inferred => "inferred",
            Self::Detected => "detected",
        })
    }
}

