//! The full set of facts under which a credential operates, and whether two of
//! them may share a host process.
//!
//! ADR-0022 is *Identity classes and runtime isolation*; clause 2 defines the
//! identity context and clause 3 is the rule it exists for. A host process runs
//! the work of one identity context, because a process holds tickets, tokens,
//! session keys and connection handles, and process isolation is the boundary
//! the operating system actually enforces.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::fmt;

use crate::mechanism::{IdentityClass, Mechanism};

/// The full set of facts under which a credential operates. ADR-0022 clause 2.
///
/// Two credentials share a context only when **every** fact matches, which is
/// why this compares structurally rather than by principal name.
///
/// The case that decides the shape: constrained and unconstrained Kerberos
/// delegation are distinct contexts even for the same principal, because
/// unconstrained delegation makes a ticket usable against services the
/// constrained case cannot reach. Treating them as one because the account name
/// matches is the specific mistake this exists to prevent.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdentityContext {
    mechanism: String,
    class: IdentityClass,
    facts: BTreeMap<String, String>,
}

impl IdentityContext {
    #[must_use]
    pub fn new(mechanism: &Mechanism) -> Self {
        Self {
            mechanism: mechanism.name().to_string(),
            class: mechanism.class(),
            facts: BTreeMap::new(),
        }
    }

    /// Add a fact. A later value for the same key replaces an earlier one.
    #[must_use]
    pub fn with(mut self, fact: impl Into<String>, value: impl Into<String>) -> Self {
        self.facts.insert(fact.into(), value.into());
        self
    }

    #[must_use]
    pub fn mechanism(&self) -> &str {
        &self.mechanism
    }

    #[must_use]
    pub const fn class(&self) -> IdentityClass {
        self.class
    }

    #[must_use]
    pub fn fact(&self, name: &str) -> Option<&str> {
        self.facts.get(name).map(String::as_str)
    }

    /// Whether these two may run in one host process. ADR-0022 clause 3.
    ///
    /// A host process runs the work of one identity context, because a process
    /// holds tickets, tokens, session keys and connection handles, and process
    /// isolation is the boundary the operating system actually enforces.
    /// Anything finer is a promise made by whatever code happens to be running
    /// — and Xmip loads third-party Modules across a C ABI, so that is not
    /// hypothetical.
    #[must_use]
    pub fn may_share_host_process(&self, other: &Self) -> bool {
        self == other
    }
}

impl fmt::Display for IdentityContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.mechanism, self.class)?;

        for (fact, value) in &self.facts {
            write!(f, " {fact}={value}")?;
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::mechanism;

    fn kerberos(delegation: &str) -> IdentityContext {
        IdentityContext::new(&mechanism::kerberos())
            .with("realm", "CORP.EXAMPLE")
            .with("principal", "xmip/node-a.corp.example")
            .with("delegation-scope", delegation)
    }

    #[test]
    fn delegation_scope_separates_one_principal_into_two_contexts() {
        // The mistake ADR-0022 clause 2 exists to prevent. Same realm, same
        // principal, and a ticket that reaches services the other cannot.
        assert_ne!(kerberos("unconstrained"), kerberos("constrained"));
        assert!(!kerberos("unconstrained").may_share_host_process(&kerberos("constrained")));
    }

    #[test]
    fn identical_facts_are_one_context() {
        assert!(kerberos("constrained").may_share_host_process(&kerberos("constrained")));
    }

    #[test]
    fn anonymous_does_not_share_with_authenticated_work() {
        let open = IdentityContext::new(&mechanism::anonymous());
        let keyed = IdentityContext::new(&mechanism::api_key()).with("key-id", "partner-x");

        assert!(!open.may_share_host_process(&keyed));
    }
    #[test]
    fn a_context_reads_as_something_an_operator_can_act_on() {
        // The failure in ADR-0022 clause 5 must name both contexts. That is
        // only useful if a context prints as more than a type name.
        assert_eq!(
            kerberos("constrained").to_string(),
            "kerberos[highAssurance] delegation-scope=constrained principal=xmip/node-a.corp.example realm=CORP.EXAMPLE"
        );
    }
}
