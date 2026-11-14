//! The audit chain's rule, once (ADR-0070 clause 5, amended 2026-10-10:
//! *one chain per writer*): a writer's first record is number 1 and
//! carries [`FIRST`] as the digest before it; each next record takes the
//! number after its writer's [`Head`] and carries the head's digest; its
//! own digest is the SHA-256 of its canonical form ([`digest`]), which
//! holds its number and the digest before it, and is the head after it.
//!
//! Each log keeps its own canonical form — the file sink a record's TOML
//! table (`xmip-core-audit`), Xmip Storage a kept record's row and the
//! Streams it carries (`xmip-core-persist`) — and both chain it by this
//! rule; the walk that says where a chain breaks is the audit
//! capability's (`audit_chain::walk`). Until 2026-11-13 the two logs each
//! took the digest from `sha2` and chained by hand; the SHA-256 is the
//! estate's one (`codec::digest`).

use alloc::string::String;

/// How long a digest is, in bytes.
pub const DIGEST: usize = 32;

/// A SHA-256 digest.
pub type Digest = [u8; DIGEST];

/// What the first record of a chain carries as the digest before it: none
/// was.
pub const FIRST: Digest = [0; DIGEST];

/// The digest of a record's canonical form.
#[must_use]
pub fn digest(canonical: &[u8]) -> Digest {
    codec::digest::sha256(canonical)
}

/// A digest as a record writes it: 64 lower-case hex digits.
#[must_use]
pub fn written(digest: &Digest) -> String {
    codec::hex::encode(digest)
}

/// The digest 64 hex digits spell, or `None`.
#[must_use]
pub fn read(text: &str) -> Option<Digest> {
    codec::hex::decode(text).ok()?.try_into().ok()
}

/// Where a writer's chain stands: the number and the digest of its last
/// record, number 0 and [`FIRST`] where it has none.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Head {
    pub position: u64,
    pub digest: Digest,
}

impl Head {
    /// A chain with no record yet.
    pub const GENESIS: Self = Self {
        position: 0,
        digest: FIRST,
    };

    /// How many bytes [`Head::bytes`] writes.
    pub const LENGTH: usize = 8 + DIGEST;

    /// The place the next record takes: its number, and the digest it
    /// carries as the one before it.
    #[must_use]
    pub const fn next(&self) -> (u64, Digest) {
        (self.position.saturating_add(1), self.digest)
    }

    /// The head once the next record, whose own digest is `own`, is in the
    /// chain.
    #[must_use]
    pub const fn after(&self, own: Digest) -> Self {
        Self {
            position: self.position.saturating_add(1),
            digest: own,
        }
    }

    /// The head as Xmip Storage keeps it: the number, eight bytes
    /// big-endian, then the digest.
    #[must_use]
    pub fn bytes(&self) -> [u8; Self::LENGTH] {
        let mut out = [0; Self::LENGTH];
        out[..8].copy_from_slice(&self.position.to_be_bytes());
        out[8..].copy_from_slice(&self.digest);
        out
    }

    /// The head `bytes` hold, or `None` where they are not one.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (number, digest) = bytes.split_at_checked(8)?;
        Some(Self {
            position: u64::from_be_bytes(number.try_into().ok()?),
            digest: digest.try_into().ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_digest_is_sha_256_written_as_hex_and_read_back() {
        let own = digest(b"abc");
        assert_eq!(
            written(&own),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(read(&written(&own)), Some(own));
        assert_eq!(read("00"), None, "too short");
    }

    #[test]
    fn a_chain_starts_at_one_after_nothing_and_goes_on_from_its_head() {
        assert_eq!(Head::GENESIS.next(), (1, FIRST));
        let first = Head::GENESIS.after([1; DIGEST]);
        assert_eq!(
            first,
            Head {
                position: 1,
                digest: [1; DIGEST]
            }
        );
        assert_eq!(first.next(), (2, [1; DIGEST]));
    }

    #[test]
    fn a_head_is_kept_as_its_number_and_its_digest() {
        let head = Head {
            position: 0x0102,
            digest: [7; DIGEST],
        };
        let bytes = head.bytes();
        assert_eq!(&bytes[..8], &[0, 0, 0, 0, 0, 0, 1, 2]);
        assert_eq!(Head::from_bytes(&bytes), Some(head));
        assert_eq!(Head::from_bytes(&bytes[..39]), None);
        assert_eq!(Head::from_bytes(&[0; 7]), None);
    }
}
