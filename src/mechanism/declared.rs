//! The mechanisms Xmip itself implements.
//!
//! Here so that `mutual-tls` means the same thing in every module rather than
//! being re-declared, differently, in each one.
//!
//! Sorted as `xmip-core-authenticate/docs/identity-by-technology.md` sorts
//! them, and every name is the last segment of the repository that implements
//! it, so [`super::Mechanism::name`] and `xmip-core-authenticate-<name>` cannot
//! drift apart.

use super::{Assurance::*, IdentityClass::*, Layer::*, Mechanism};

// -- Transport: cryptographic proof bound to a principal ---------------

/// RFC 8446. The channel proves the client.
#[must_use]
pub fn mutual_tls() -> Mechanism {
    Mechanism::declare("mutual-tls", HighAssurance, Transport, Authenticates)
}

/// RFC 4559 Negotiate/SPNEGO, and GSSAPI over SFTP, NFS, Kafka and
/// PostgreSQL. One mechanism, many carriers.
#[must_use]
pub fn kerberos() -> Mechanism {
    Mechanism::declare("kerberos", HighAssurance, Transport, Authenticates)
}

/// An X.509 certificate outside a TLS handshake — S/MIME in AS2, an Oracle
/// wallet, an OPC UA application instance certificate.
#[must_use]
pub fn certificate() -> Mechanism {
    Mechanism::declare("certificate", HighAssurance, Transport, Authenticates)
}

/// SSH-2 public key. RFC 4252.
#[must_use]
pub fn ssh_key() -> Mechanism {
    Mechanism::declare("ssh-key", HighAssurance, Transport, Authenticates)
}

/// WebAuthn and FIDO2. A key pair bound to a principal and to an origin,
/// with the private half unable to leave its authenticator.
///
/// Absent from `identity-by-technology.md`, which sorted the integration
/// estate rather than the operator surfaces. It reaches Xmip through the
/// GUIs rather than through a transport.
#[must_use]
pub fn passkey() -> Mechanism {
    Mechanism::declare("passkey", HighAssurance, Transport, Authenticates)
}

/// `SO_PEERCRED` on a Unix socket, the caller SID on a named pipe.
///
/// The kernel vouches for it and it is never presented at all, which makes
/// it stronger than most credentials and unlike all of them.
#[must_use]
pub fn peer_credentials() -> Mechanism {
    Mechanism::declare("peer-credentials", HighAssurance, Transport, Authenticates)
}

/// RFC 9421 HTTP Message Signatures. Signs selected headers and the body,
/// and is still transport: it is readable before Message creation.
#[must_use]
pub fn http_message_signature() -> Mechanism {
    Mechanism::declare(
        "http-message-signature",
        HighAssurance,
        Transport,
        Authenticates,
    )
}

// -- Transport: asserted by a third party -------------------------------

/// RFC 6749.
#[must_use]
pub fn oauth2() -> Mechanism {
    Mechanism::declare("oauth2", Federated, Transport, Authenticates)
}

/// OIDC Core 1.0.
#[must_use]
pub fn oidc() -> Mechanism {
    Mechanism::declare("oidc", Federated, Transport, Authenticates)
}

// -- Transport: a secret both sides hold --------------------------------

/// RFC 7617.
#[must_use]
pub fn basic() -> Mechanism {
    Mechanism::declare("basic", SharedSecret, Transport, Authenticates)
}

/// RFC 7616.
#[must_use]
pub fn digest() -> Mechanism {
    Mechanism::declare("digest", SharedSecret, Transport, Authenticates)
}

/// RFC 6750 — carriage, not issuance.
///
/// `sharedSecret` rather than `federated` on purpose. A bearer token by
/// itself proves possession and nothing else; whether an issuer stands
/// behind it is a fact about [`oauth2`] or [`oidc`], which are separate
/// mechanisms for exactly that reason.
#[must_use]
pub fn bearer() -> Mechanism {
    Mechanism::declare("bearer", SharedSecret, Transport, Authenticates)
}

/// No standard. Vendor convention, universally.
#[must_use]
pub fn api_key() -> Mechanism {
    Mechanism::declare("api-key", SharedSecret, Transport, Authenticates)
}

/// RFC 5802. Kafka, PostgreSQL, MySQL `caching_sha2_password`.
#[must_use]
pub fn scram() -> Mechanism {
    Mechanism::declare("scram", SharedSecret, Transport, Authenticates)
}

/// Username and password over a protected channel — FTPS, SASL PLAIN,
/// a SQL login.
#[must_use]
pub fn password() -> Mechanism {
    Mechanism::declare("password", SharedSecret, Transport, Authenticates)
}

/// AWS Signature Version 4.
#[must_use]
pub fn sigv4() -> Mechanism {
    Mechanism::declare("sigv4", SharedSecret, Transport, Authenticates)
}

/// Azure Shared Key and SAS tokens.
#[must_use]
pub fn shared_access_signature() -> Mechanism {
    Mechanism::declare(
        "shared-access-signature",
        SharedSecret,
        Transport,
        Authenticates,
    )
}

/// The path, the permissions and the source address of a drop folder.
///
/// ADR-0019 clause 7: a partner drop folder is not an absence of identity.
/// The circumstance *is* the transport identity, and it is authenticated as
/// that — weakly, and on the record.
#[must_use]
pub fn circumstance() -> Mechanism {
    Mechanism::declare("circumstance", SharedSecret, Transport, Authenticates)
}

// -- Transport: nothing claimed -----------------------------------------

/// ADR-0019 clause 2: an authenticated outcome, not a skipped gate. The
/// claim is "nobody", it is verified as such, and authorization then
/// decides whether nobody may post here.
#[must_use]
pub fn anonymous() -> Mechanism {
    Mechanism::declare("anonymous", Anonymous, Transport, Authenticates)
}

// -- Message: proof inside the payload -----------------------------------

/// OASIS WSS 1.1 — UsernameToken, X.509 and SAML token profiles.
#[must_use]
pub fn ws_security() -> Mechanism {
    Mechanism::declare("ws-security", HighAssurance, Message, Authenticates)
}

/// RFC 7515 JWS, and RFC 7519 JWT inside a payload.
#[must_use]
pub fn jws() -> Mechanism {
    Mechanism::declare("jws", HighAssurance, Message, Authenticates)
}

/// W3C XMLDSIG, RFC 3275.
#[must_use]
pub fn xml_signature() -> Mechanism {
    Mechanism::declare("xml-signature", HighAssurance, Message, Authenticates)
}

/// RFC 8551. The sender half of AS2.
#[must_use]
pub fn s_mime() -> Mechanism {
    Mechanism::declare("s-mime", HighAssurance, Message, Authenticates)
}

/// RFC 6376. Proves the message was signed by the claimed domain and is
/// unaltered.
#[must_use]
pub fn dkim() -> Mechanism {
    Mechanism::declare("dkim", HighAssurance, Message, Authenticates)
}

/// ISO 9735-5/6/7 AUTACK. EDIFACT with cryptography actually applied.
#[must_use]
pub fn autack() -> Mechanism {
    Mechanism::declare("autack", HighAssurance, Message, Authenticates)
}

// -- Message: a name, and nothing behind it ------------------------------

/// X12 ISA05–ISA08. **Identifies. Does not authenticate.**
///
/// Enough to route and bill a Journey. Proving it is the transport's job,
/// or AS2's.
#[must_use]
pub fn edi_x12_interchange() -> Mechanism {
    Mechanism::declare("edi-x12-interchange", SharedSecret, Message, Identifies)
}

/// EDIFACT UNB S002 and S003. **Identifies. Does not authenticate.**
#[must_use]
pub fn edifact_interchange() -> Mechanism {
    Mechanism::declare("edifact-interchange", SharedSecret, Message, Identifies)
}

/// HL7 v2.x MSH-3 and MSH-4. **Identifies. Does not authenticate.**
///
/// MLLP is a framing protocol with no security whatever, carrying
/// healthcare data, so for most HL7 deployments this is the only identity
/// present anywhere.
#[must_use]
pub fn hl7_sending_application() -> Mechanism {
    Mechanism::declare("hl7-sending-application", SharedSecret, Message, Identifies)
}

// -- Transport: a name read off the connection, and nothing behind it ----
//
// ADR-0050 section 3. ADR-0022 has four classes and none of them is "a name
// with no proof"; [`circumstance`] set the precedent that such a claim is
// filed as a shared secret and marked as identifying only, and these follow
// it. Each is the last segment of `xmip-core-identify-<name>`.

/// The peer address. Spoofable on most networks and still the commonest
/// allow-list there is.
#[must_use]
pub fn ip() -> Mechanism {
    Mechanism::declare("ip", SharedSecret, Transport, Identifies)
}

/// The peer's link-layer address, where a fieldbus or Ethernet transport
/// reports one.
#[must_use]
pub fn mac() -> Mechanism {
    Mechanism::declare("mac", SharedSecret, Transport, Identifies)
}

/// The peer's reverse-resolved name, from the resolver the node is given.
#[must_use]
pub fn dns() -> Mechanism {
    Mechanism::declare("dns", SharedSecret, Transport, Identifies)
}

/// One named transport header, read as presented.
#[must_use]
pub fn header() -> Mechanism {
    Mechanism::declare("header", SharedSecret, Transport, Identifies)
}

/// One named cookie.
#[must_use]
pub fn cookie() -> Mechanism {
    Mechanism::declare("cookie", SharedSecret, Transport, Identifies)
}

/// A username presented without a proof — an FTP `USER`, a SASL name, the
/// user half of a Basic credential. The proof, where one follows, is
/// [`password`]'s.
#[must_use]
pub fn username() -> Mechanism {
    Mechanism::declare("username", SharedSecret, Transport, Identifies)
}

/// The identity the Receive Location's configuration names for whatever
/// arrives on it. Inferred, never passed.
#[must_use]
pub fn endpoint() -> Mechanism {
    Mechanism::declare("endpoint", SharedSecret, Transport, Identifies)
}

/// The Party the Location's configuration names. Inferred, never passed.
#[must_use]
pub fn party() -> Mechanism {
    Mechanism::declare("party", SharedSecret, Transport, Identifies)
}

/// One named property the carrier promoted — an AMQP container id, an MQTT
/// client id, a Kafka client id.
#[must_use]
pub fn transport() -> Mechanism {
    Mechanism::declare("transport", SharedSecret, Transport, Identifies)
}

// -- Transport: verified by something the node holds or reaches -----------

/// RFC 7519 as a transport credential — a `Bearer` token with a signature
/// the node can check. Inside a payload it is [`jws`].
#[must_use]
pub fn jwt() -> Mechanism {
    Mechanism::declare("jwt", Federated, Transport, Authenticates)
}

/// SAML 2.0, an assertion signed by an identity provider.
#[must_use]
pub fn saml() -> Mechanism {
    Mechanism::declare("saml", Federated, Transport, Authenticates)
}

/// NTLMv2. A challenge and a response over the stored NT hash.
#[must_use]
pub fn ntlm() -> Mechanism {
    Mechanism::declare("ntlm", SharedSecret, Transport, Authenticates)
}

/// A username and password proven by a bind at a directory.
#[must_use]
pub fn ldap() -> Mechanism {
    Mechanism::declare("ldap", SharedSecret, Transport, Authenticates)
}

/// A username and password proven by the host's PAM stack.
#[must_use]
pub fn pam() -> Mechanism {
    Mechanism::declare("pam", SharedSecret, Transport, Authenticates)
}

/// A credential proven by the host's SSPI.
#[must_use]
pub fn windows() -> Mechanism {
    Mechanism::declare("windows", SharedSecret, Transport, Authenticates)
}

// -- Message: a name read out of the Message, and nothing behind it -------

/// One named property of the Message's context — an EDI sender id promoted
/// earlier, a routing key. **Identifies. Does not authenticate.**
#[must_use]
pub fn message() -> Mechanism {
    Mechanism::declare("message", SharedSecret, Message, Identifies)
}

/// The Party the Contract the first section is bound to names as its sender.
/// **Identifies. Does not authenticate.**
#[must_use]
pub fn contract() -> Mechanism {
    Mechanism::declare("contract", SharedSecret, Message, Identifies)
}

#[cfg(test)]
mod tests {
    use crate::mechanism;
    use crate::mechanism::{IdentityClass, Layer};

    #[test]
    fn class_and_layer_come_from_the_mechanism_not_from_configuration() {
        assert_eq!(
            mechanism::mutual_tls().class(),
            IdentityClass::HighAssurance
        );

        // An API key is a shared secret however anyone would prefer to
        // describe it, and there is no setter to argue with.
        assert_eq!(mechanism::api_key().class(), IdentityClass::SharedSecret);
    }

    #[test]
    fn a_bearer_token_proves_possession_and_not_an_issuer() {
        // RFC 6750 is carriage. Whether an issuer stands behind the token is a
        // fact about oauth2 or oidc, which is why those are separate.
        assert_eq!(mechanism::bearer().class(), IdentityClass::SharedSecret);
        assert_eq!(mechanism::oauth2().class(), IdentityClass::Federated);
    }

    #[test]
    fn a_signed_http_request_is_still_transport() {
        // RFC 9421 signs the body, so what it proves is content integrity. It
        // is read before Message creation, so the layer is transport anyway.
        let signed = mechanism::http_message_signature();

        assert_eq!(signed.layer(), Layer::Transport);
        assert_eq!(signed.class(), IdentityClass::HighAssurance);
    }
    #[test]
    fn edi_interchange_identifiers_name_a_party_and_prove_nothing() {
        // The classic B2B mistake, refused at the type level.
        for claim in [
            mechanism::edi_x12_interchange(),
            mechanism::edifact_interchange(),
            mechanism::hl7_sending_application(),
        ] {
            assert!(
                !claim.authenticates(),
                "{} carries no cryptography",
                claim.name()
            );
        }

        // AUTACK is EDIFACT with the cryptography actually applied, and does.
        assert!(mechanism::autack().authenticates());
    }

    #[test]
    fn a_drop_folder_is_an_identity_rather_than_the_absence_of_one() {
        let folder = mechanism::circumstance();

        assert_eq!(folder.layer(), Layer::Transport);
        assert_ne!(folder.class(), IdentityClass::Anonymous);
        assert!(folder.authenticates());
    }

    #[test]
    fn a_kernel_vouched_identity_is_never_presented_and_still_proves_most() {
        assert_eq!(
            mechanism::peer_credentials().class(),
            IdentityClass::HighAssurance
        );
    }
    #[test]
    fn every_identity_technology_of_adr_0050_has_a_declared_mechanism() {
        // Nineteen identify, eighteen authenticate: the name a technology
        // reads and the name its sibling verifies are one value, declared here.
        let names = [
            "ip",
            "mac",
            "dns",
            "header",
            "cookie",
            "username",
            "api-key",
            "certificate",
            "ssh-key",
            "jwt",
            "oidc",
            "saml",
            "kerberos",
            "ntlm",
            "endpoint",
            "party",
            "transport",
            "message",
            "contract",
            "password",
            "basic",
            "digest",
            "bearer",
            "scram",
            "oauth2",
            "mutual-tls",
            "ldap",
            "pam",
            "windows",
        ];
        let declared = [
            mechanism::ip(),
            mechanism::mac(),
            mechanism::dns(),
            mechanism::header(),
            mechanism::cookie(),
            mechanism::username(),
            mechanism::api_key(),
            mechanism::certificate(),
            mechanism::ssh_key(),
            mechanism::jwt(),
            mechanism::oidc(),
            mechanism::saml(),
            mechanism::kerberos(),
            mechanism::ntlm(),
            mechanism::endpoint(),
            mechanism::party(),
            mechanism::transport(),
            mechanism::message(),
            mechanism::contract(),
            mechanism::password(),
            mechanism::basic(),
            mechanism::digest(),
            mechanism::bearer(),
            mechanism::scram(),
            mechanism::oauth2(),
            mechanism::mutual_tls(),
            mechanism::ldap(),
            mechanism::pam(),
            mechanism::windows(),
        ];

        for (name, declared) in names.iter().zip(declared.iter()) {
            assert_eq!(declared.name(), *name);
        }
    }

    #[test]
    fn a_name_read_off_the_connection_identifies_and_proves_nothing() {
        for claim in [
            mechanism::ip(),
            mechanism::header(),
            mechanism::username(),
            mechanism::endpoint(),
            mechanism::message(),
        ] {
            assert!(!claim.authenticates(), "{} is a name", claim.name());
        }
    }
}
