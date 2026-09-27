//! A failure, said once: the message-carrying error every crate declares, and
//! the retryable failure the resilience guards read.

use alloc::string::String;

/// Declare a crate's message-carrying error type: the `String` message, the
/// standard `Display` and `Error` impls, and a `new` constructor. A dozen crates
/// hand-rolled this identical boilerplate (ADR-0037); this is the one definition.
/// The `message` field stays public, so a `Name { message }` literal still works.
#[macro_export]
macro_rules! declare_error {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $name {
            pub message: String,
        }

        impl $name {
            #[must_use]
            pub fn new(message: impl Into<String>) -> Self {
                Self { message: message.into() }
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(&self.message)
            }
        }

        impl core::error::Error for $name {}
    };
}

/// Declare a crate's own retryable failure: the shape of [`Failure`] under a
/// name the crate owns, so the crate can convert its dependencies' errors into
/// it, and a conversion into [`Failure`] that keeps the judgement. The
/// transport declares `TransportError` with it; nothing else writes the shape.
///
/// Whether a failure is worth retrying is a property **of the failure**, not
/// of the call site (ADR-0026, ADR-0037): it is decided where the failure is
/// met and read, unchanged, by whatever decides to try again.
#[macro_export]
macro_rules! declare_retryable_error {
    ($(#[$meta:meta])* $name:ident) => {
        $crate::declare_retryable_error!(@shape $(#[$meta])* $name);

        impl From<$name> for $crate::Failure {
            fn from(error: $name) -> Self {
                Self { message: error.message, retryable: error.retryable }
            }
        }
    };
    (@shape $(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name {
            pub message: String,
            pub retryable: bool,
        }

        impl $name {
            /// A failure worth trying again. A blip, a timeout, a reset.
            #[must_use]
            pub fn retryable(message: impl Into<String>) -> Self {
                Self { message: message.into(), retryable: true }
            }

            /// A failure that will say the same thing next time.
            #[must_use]
            pub fn permanent(message: impl Into<String>) -> Self {
                Self { message: message.into(), retryable: false }
            }

            /// The same failure, said from where it was met: `"<where>: <message>"`.
            ///
            /// Retryability is the failure's own property and survives the
            /// wrapping. Writing `format!("{error}")` into a new failure instead
            /// loses it — a timeout becomes permanent, and resilience stops
            /// retrying what it should retry — and doubles the judgement in the
            /// text, which is how it was found: a Linux run read *(retryable)
            /// (not retryable)* on one line (2026-09-19).
            #[must_use]
            pub fn at(mut self, where_met: &str) -> Self {
                self.message.insert_str(0, ": ");
                self.message.insert_str(0, where_met);
                self
            }
        }

        /// The message and the judgement: an operator reads this in a log
        /// without the struct around it.
        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let judgement = if self.retryable { "retryable" } else { "not retryable" };
                write!(f, "{} ({judgement})", self.message)
            }
        }

        impl core::error::Error for $name {}
    };
}

declare_retryable_error!(
    @shape
    /// Why an attempt failed, and whether trying again could change that: the
    /// one retryable failure. A send answers with it, the resilience guards
    /// judge an attempt by it, and a crate's own retryable error converts into
    /// it without losing the judgement.
    Failure
);

#[cfg(test)]
mod tests {
    use super::Failure;
    use alloc::string::ToString;

    #[test]
    fn the_judgement_is_visible_in_the_message_and_survives_saying_where() {
        assert_eq!(
            Failure::retryable("the peer hung up").to_string(),
            "the peer hung up (retryable)"
        );
        let met = Failure::permanent("403").at("sending to partner-x");
        assert_eq!(met.message, "sending to partner-x: 403");
        assert!(!met.retryable);
        assert_eq!(met.to_string(), "sending to partner-x: 403 (not retryable)");
    }
}
