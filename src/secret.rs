//! A secret: the proof Xmip presents to a party it connects to — a
//! password, an SSH private key, or a certificate with its private key —
//! named by a Location and kept in the Xmip Administration Database, never
//! in configuration (ADR-0019, amendment 2026-10-29).
//!
//! What a secret is, said once, here, beneath every capability that names
//! one: its name, its kind and the one check of a value against its kind.
//! Where it is kept is Xmip Storage's (`xmip-core-persist`, the `secret`
//! table), and what reads it is [`Secrets`], handed in.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use zeroize::Zeroizing;

use crate::Failure;

/// The most characters a secret's name holds.
pub const SECRET_NAME_LONGEST: usize = 128;

/// A secret's name: what a Location's `secret` says and the Administration
/// Database keeps it under. One to [`SECRET_NAME_LONGEST`] characters of
/// ASCII letters, digits, `.`, `_` and `-`, opening with a letter or digit.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SecretName(String);

impl SecretName {
    /// `name`, where it is one.
    ///
    /// # Errors
    ///
    /// In words, where it is empty, too long, or holds a character a name
    /// may not.
    pub fn new(name: impl Into<String>) -> Result<Self, String> {
        let name = name.into();
        let opens = name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric());
        let allowed = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        if !opens || !allowed || name.len() > SECRET_NAME_LONGEST {
            return Err(format!(
                "'{name}' is no secret name: one to {SECRET_NAME_LONGEST} ASCII letters, \
                 digits, '.', '_' and '-', opening with a letter or a digit"
            ));
        }
        Ok(Self(name))
    }

    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a secret is.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SecretKind {
    /// A password, as UTF-8.
    Password,
    /// An SSH private key, PEM: OpenSSH's own form, unencrypted.
    Key,
    /// A certificate chain, leaf first, with its private key, PEM.
    Certificate,
}

impl SecretKind {
    /// Every kind, in the order a surface offers them.
    pub const ALL: [Self; 3] = [Self::Password, Self::Key, Self::Certificate];

    /// Its word: what a surface takes and the database keeps.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Key => "key",
            Self::Certificate => "certificate",
        }
    }

    /// The kind `word` names, exactly.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }

    /// What a value of it is, in the words a surface's help uses.
    #[must_use]
    pub const fn meaning(self) -> &'static str {
        match self {
            Self::Password => "a password",
            Self::Key => "an SSH private key, PEM, unencrypted",
            Self::Certificate => "a certificate chain, leaf first, and its private key, PEM",
        }
    }
}

impl fmt::Display for SecretKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// A secret's value, with its name and kind: overwritten when it is
/// dropped, and never printed — `{:?}` shows its name and kind alone.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
    name: SecretName,
    kind: SecretKind,
    value: Zeroizing<Vec<u8>>,
}

impl Secret {
    /// `value` as the secret `name` of `kind`, where it is one of that kind:
    /// a password not empty; a key or a certificate PEM text holding a
    /// private key, a certificate holding a certificate too.
    ///
    /// # Errors
    ///
    /// In words that never repeat the value, where it is not of its kind.
    pub fn new(name: SecretName, kind: SecretKind, value: Vec<u8>) -> Result<Self, String> {
        let value = Zeroizing::new(value);
        let text = core::str::from_utf8(&value).ok();
        let holds = |label: &str| text.is_some_and(|text| pem_holds(text, label));
        let fits = match kind {
            SecretKind::Password => text.is_some_and(|text| !text.is_empty()),
            SecretKind::Key => holds("PRIVATE KEY"),
            SecretKind::Certificate => holds("CERTIFICATE") && holds("PRIVATE KEY"),
        };
        if !fits {
            return Err(format!(
                "the secret '{name}' is no {kind}: a {kind} is {}",
                kind.meaning()
            ));
        }
        Ok(Self { name, kind, value })
    }

    #[must_use]
    pub const fn name(&self) -> &SecretName {
        &self.name
    }

    #[must_use]
    pub const fn kind(&self) -> SecretKind {
        self.kind
    }

    /// The value as it was set.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    /// The value as text: every kind is UTF-8, checked as it was made.
    #[must_use]
    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.value).unwrap_or_default()
    }

    /// The value, where the secret is of `kind`.
    ///
    /// # Errors
    ///
    /// Permanent, in words, where it is of another kind: what a Location's
    /// technology presents is not what the secret holds.
    pub fn as_kind(&self, kind: SecretKind, presenting: &str) -> Result<&str, Failure> {
        if self.kind == kind {
            Ok(self.text())
        } else {
            Err(Failure::permanent(format!(
                "the secret '{}' is a {}, and {presenting} presents a {kind}",
                self.name, self.kind
            )))
        }
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Secret")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Whether `text` holds a PEM block whose label ends with `label`.
fn pem_holds(text: &str, label: &str) -> bool {
    text.lines().any(|line| {
        line.trim()
            .strip_prefix("-----BEGIN ")
            .and_then(|rest| rest.strip_suffix("-----"))
            .is_some_and(|found| found.ends_with(label))
    })
}

/// Where a node reads the secrets its Locations name: the Administration
/// Database, through Xmip Storage. Handed in (ADR-0068).
pub trait Secrets: Send + Sync {
    /// The secret `name` as it is set now; `None` where none is.
    ///
    /// # Errors
    ///
    /// Retryable where Xmip Storage did not answer; permanent where what it
    /// keeps is not a secret.
    fn secret(&self, name: &SecretName) -> Result<Option<Secret>, Failure>;
}

impl<T: Secrets + ?Sized> Secrets for Box<T> {
    fn secret(&self, name: &SecretName) -> Result<Option<Secret>, Failure> {
        (**self).secret(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    /// A value drawn for the test, never one written down.
    fn drawn() -> String {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        format!(
            "drawn-{:x}",
            now.map(|since| since.as_nanos()).unwrap_or_default()
        )
    }

    fn name() -> SecretName {
        SecretName::new("partner-x.sftp").expect("a name")
    }

    #[test]
    fn a_name_is_letters_digits_dots_underscores_and_hyphens() {
        assert!(SecretName::new("orders_out-1.key").is_ok());
        for refused in ["", "-x", "a b", "a/b", &"x".repeat(129)] {
            let said = SecretName::new(refused).expect_err(refused);
            assert!(said.contains("is no secret name"), "{said}");
        }
    }

    #[test]
    fn a_value_is_held_to_its_kind_and_never_said() {
        let drawn = drawn();
        assert!(Secret::new(name(), SecretKind::Password, drawn.clone().into()).is_ok());
        let said = Secret::new(name(), SecretKind::Key, drawn.clone().into()).expect_err("no PEM");
        assert!(!said.contains(&drawn), "{said}");
        assert!(said.contains("is no key"), "{said}");
        let key = "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----";
        assert!(Secret::new(name(), SecretKind::Key, key.into()).is_ok());
        assert!(Secret::new(name(), SecretKind::Certificate, key.into()).is_err());
        let both = "-----BEGIN CERTIFICATE-----\nA\n-----END CERTIFICATE-----\n\
                    -----BEGIN PRIVATE KEY-----\nB\n-----END PRIVATE KEY-----";
        assert!(Secret::new(name(), SecretKind::Certificate, both.into()).is_ok());
        assert!(Secret::new(name(), SecretKind::Password, Vec::new()).is_err());
    }

    #[test]
    fn a_secret_prints_its_name_and_kind_alone() {
        let drawn = drawn();
        let secret = Secret::new(name(), SecretKind::Password, drawn.clone().into()).expect("made");
        let printed = format!("{secret:?}");
        assert!(printed.contains("partner-x.sftp") && !printed.contains(&drawn));
        let other = secret
            .as_kind(SecretKind::Key, "sftp")
            .expect_err("a password");
        assert!(!other.retryable);
        assert!(
            other
                .message
                .contains("is a password, and sftp presents a key")
        );
        assert_eq!(
            secret.as_kind(SecretKind::Password, "sftp").expect("it is"),
            drawn
        );
        assert_eq!(
            SecretKind::named("certificate"),
            Some(SecretKind::Certificate)
        );
        assert_eq!(SecretKind::Key.to_string(), "key");
    }
}
