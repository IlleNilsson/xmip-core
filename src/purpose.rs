//! What an identity is configured for.
//!
//! ADR-0019 clause 4. A Party's identities are per purpose because they differ
//! in kind, not only in value: two of the four match something arriving and
//! need no secret, two produce proof and name where the material is kept.

use core::fmt;

/// What an identity is configured for.
///
/// A Party's identities are per purpose because they genuinely differ, and they
/// differ in kind rather than only in value. Two of the four match something
/// arriving and need no secret; two produce proof and name where the material
/// is kept. ADR-0019 clause 4.
///
/// ```text
/// Receive    a partner arrives          matcher
/// Operate    a person drives Xmip       matcher
/// Process    Xmip runs as somebody      credential
/// Send       Xmip is the client         credential
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Purpose {
    /// Verified when it arrives. A Receive Location names the Parties it takes,
    /// and an arriving credential is compared against a stored matcher.
    Receive,
    /// What a Process runs as.
    ///
    /// The consequential one. ADR-0022 clause 3 gives a host process the work
    /// of exactly one identity context, so this identity is not only what the
    /// Process acts as — it decides which host process the Process can be
    /// placed in, and therefore how many host processes a node runs.
    Process,
    /// Offered when Xmip is the client. ADR-0006 resolves which one, inheriting
    /// up through Send Port and Send Port Group to the Sending Process.
    Send,
    /// Who is driving Xmip itself.
    ///
    /// The CLI, the PowerShell module, the MAUI desktop GUI and the Blazor web
    /// GUI all authenticate somebody, and that somebody is a Party like any
    /// other — a person rather than a trading partner, but recognised the same
    /// way. ADR-0014.
    ///
    /// Matched, not presented: the operator proves themselves to Xmip, so this
    /// stores a matcher and no secret, exactly as [`Purpose::Receive`] does.
    ///
    /// ADR-0009 still applies. Recognising an operator is not granting them
    /// anything; a Party is recognised, a role is granted.
    Operate,
}

impl Purpose {
    /// Whether this purpose requires credential material rather than a matcher.
    ///
    /// Receiving and operating both compare an arriving credential against a
    /// stored name and need no secret. Processing and sending mean producing
    /// proof, so both name where the material is kept.
    #[must_use]
    pub const fn needs_credential(self) -> bool {
        matches!(self, Self::Process | Self::Send)
    }
}

impl fmt::Display for Purpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Receive => "receive",
            Self::Process => "process",
            Self::Send => "send",
            Self::Operate => "operate",
        })
    }
}
