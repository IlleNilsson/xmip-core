//! The material a presented identity is proven with.
//!
//! A reference, never the secret. ADR-0019 clause 4 makes the Party the
//! identity holder and explicitly not a credential store.

use alloc::string::String;
use core::fmt;

/// The material a presented identity is proven with.
///
/// A reference, never the secret. ADR-0019 clause 4 makes the Party the
/// identity holder and explicitly not a credential store: the secret lives in
/// whatever the deployment uses — a certificate store, a key vault, an SSH
/// agent, a TPM — and Xmip carries the name of it.
///
/// A Party that held the bytes would put every partner secret in every
/// configuration export, every backup and every support bundle.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CredentialRef {
    /// Which store — `windows-certificate-store`, `azure-key-vault`,
    /// `ssh-agent`, `file`, `environment`.
    pub store: String,
    /// The name within that store.
    pub reference: String,
}

impl CredentialRef {
    #[must_use]
    pub fn new(store: impl Into<String>, reference: impl Into<String>) -> Self {
        Self {
            store: store.into(),
            reference: reference.into(),
        }
    }
}

impl fmt::Display for CredentialRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.store, self.reference)
    }
}

