//! What time it is, in the one unit Xmip keeps it in.
//!
//! **Nanoseconds since the Unix epoch, as an `i128`.** Every stamp the estate
//! takes — when a Journey entry was written, when a gate concluded, when an
//! Event was raised, when a node started — is read from a [`Clock`] in this
//! unit. A wire or a store that writes another unit converts at the wire:
//! RFC 3339 text through `codec::civil`, whole seconds where a protocol
//! counts seconds. `i128` because it holds any `SystemTime` a platform
//! reports, and because `codec::civil::rfc3339_nanos` reads it.

/// Nanoseconds in a second, for a wire that counts seconds.
pub const NANOS_A_SECOND: i128 = 1_000_000_000;

/// Where the time comes from. [`SystemClock`] in a running node; a test pins
/// its own.
pub trait Clock: Send + Sync {
    /// Nanoseconds since the Unix epoch, now.
    fn unix_timestamp_nanos(&self) -> i128;

    /// Whole seconds since the Unix epoch, now: the same reading, for a wire
    /// that counts seconds.
    fn unix_seconds(&self) -> i64 {
        i64::try_from(self.unix_timestamp_nanos().div_euclid(NANOS_A_SECOND)).unwrap_or(i64::MAX)
    }
}

/// The operating system's wall clock: the one production [`Clock`].
#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

#[cfg(feature = "std")]
impl Clock for SystemClock {
    /// A moment before the epoch, which no clock this runs under reports, is
    /// counted backwards from it.
    fn unix_timestamp_nanos(&self) -> i128 {
        let now = std::time::SystemTime::now();
        match now.duration_since(std::time::UNIX_EPOCH) {
            Ok(since) => i128::try_from(since.as_nanos()).unwrap_or(i128::MAX),
            Err(before) => -i128::try_from(before.duration().as_nanos()).unwrap_or(i128::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pinned(i128);

    impl Clock for Pinned {
        fn unix_timestamp_nanos(&self) -> i128 {
            self.0
        }
    }

    #[test]
    fn seconds_are_the_same_reading_floored() {
        assert_eq!(
            Pinned(1_800_000_000_999_999_999).unix_seconds(),
            1_800_000_000
        );
        assert_eq!(Pinned(-1).unix_seconds(), -1);
    }

    #[cfg(feature = "std")]
    #[test]
    fn the_system_clock_is_after_this_was_written() {
        assert!(SystemClock.unix_seconds() > 1_790_000_000);
    }
}
