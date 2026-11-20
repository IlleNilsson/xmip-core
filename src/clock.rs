//! What time it is, in the one unit Xmip keeps it in.
//!
//! **Nanoseconds since the Unix epoch, as an `i128`.** Every stamp the estate
//! takes — when a Journey entry was written, when a gate concluded, when an
//! Event was raised, when a node started — is read from a [`Clock`] in this
//! unit. A wire or a store that writes another unit converts at the wire:
//! RFC 3339 text through `codec::civil`, whole seconds where a protocol
//! counts seconds. `i128` because it holds any `SystemTime` a platform
//! reports, and because `codec::civil::rfc3339_nanos` reads it.
//!
//! **A decision counts a span on the monotonic reading of the Clock it was
//! handed** ([`Clock::monotonic_nanos`]; ADR-0068, amendment 2026-11-15): a
//! cool-down, a refill, a Storage node passed over, a standby's next sync.
//! It keeps each moment as that reading, adds a span with [`nanos`] and
//! reads the span between two readings with [`between`], so a test hands it
//! [`Stepped`] and steps it past the edge, with no sleep. A wait the
//! operating system makes, and a span only reported, read std's `Instant`.

use core::time::Duration;

#[cfg(any(test, feature = "test-support"))]
use core::sync::atomic::{AtomicI64, Ordering};

/// Nanoseconds in a second, for a wire that counts seconds.
pub const NANOS_A_SECOND: i128 = 1_000_000_000;

/// `span` in the clock's unit, nanoseconds: what a reading and a span are
/// added in, at most what an `i128` holds.
#[must_use]
pub fn nanos(span: Duration) -> i128 {
    i128::try_from(span.as_nanos()).unwrap_or(i128::MAX)
}

/// The span from the reading `earlier` to the reading `later` of one clock;
/// none where `later` is not after it.
#[must_use]
pub fn between(earlier: i128, later: i128) -> Duration {
    let apart = later.saturating_sub(earlier).max(0);
    let seconds = u64::try_from(apart / NANOS_A_SECOND).unwrap_or(u64::MAX);
    let rest = u32::try_from(apart % NANOS_A_SECOND).unwrap_or(0);
    Duration::new(seconds, rest)
}

/// The number after `last` at the reading `now`, in whatever unit the
/// caller counts: `now` where it is above `last`, else one above `last`.
/// Strictly increasing from one number to the next whatever the clock does,
/// and never below a clock that runs on, so a number taken after a store
/// was lost or restored is still above every one taken before it, while
/// the clock is right. A slice's serial, a revocation list's number and a
/// node's start are each taken so (ADR-0031, amendment 2026-11-07; ADR-0056,
/// amendment 2026-11-20).
#[must_use]
pub fn serial(last: u64, now: i128) -> u64 {
    let now = u64::try_from(now).unwrap_or(if now < 0 { 0 } else { u64::MAX });
    now.max(last.saturating_add(1))
}

/// `clock`'s monotonic reading as a function of its own, holding the clock
/// as long as it lives: what a part that reads no clock itself — a Library,
/// as `net`'s watched connection and HTTP/2 are — is handed by a caller
/// that was handed `clock` (ADR-0068, amendments 2026-11-14 and
/// 2026-11-15).
#[cfg(feature = "std")]
pub fn reading(clock: &std::sync::Arc<dyn Clock>) -> impl Fn() -> i128 + Send + Sync + 'static {
    let clock = std::sync::Arc::clone(clock);
    move || clock.monotonic_nanos()
}

/// Where the time comes from. [`SystemClock`] in a running node; a test
/// hands in [`Stepped`].
pub trait Clock: Send + Sync {
    /// Nanoseconds since the Unix epoch, now.
    fn unix_timestamp_nanos(&self) -> i128;

    /// Nanoseconds since a moment of this clock's own, now: never less than
    /// a reading before it, whatever the wall clock is set to meanwhile, so
    /// a span counted on it — a lease, a timeout — neither stretches nor
    /// shrinks when the wall clock is stepped. Only a difference of two
    /// readings means anything. A clock that keeps no other time, as a
    /// device's may, answers its wall-clock reading.
    fn monotonic_nanos(&self) -> i128 {
        self.unix_timestamp_nanos()
    }

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

    /// The operating system's monotonic clock (`std::time::Instant`),
    /// counted from the first reading in this process.
    fn monotonic_nanos(&self) -> i128 {
        static ORIGIN: std::sync::LazyLock<std::time::Instant> =
            std::sync::LazyLock::new(std::time::Instant::now);
        i128::try_from(ORIGIN.elapsed().as_nanos()).unwrap_or(i128::MAX)
    }
}

/// A clock a test steps: its wall and monotonic readings move only when the
/// test moves them, never with the operating system's, so a decision on a
/// span is held at its edge without a sleep. Atomics, so a node's threads
/// read it as the test steps it. The estate's one test clock, under
/// `test-support`: never in a node.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct Stepped {
    wall: AtomicI64,
    monotonic: AtomicI64,
}

#[cfg(any(test, feature = "test-support"))]
impl Stepped {
    /// Reading `wall` nanoseconds since the Unix epoch, its monotonic clock
    /// at its own moment, zero.
    ///
    /// # Panics
    ///
    /// Where `wall` is more than some 292 years from the epoch.
    #[must_use]
    pub fn at(wall: i128) -> Self {
        let clock = Self::default();
        clock.set_wall(wall);
        clock
    }

    /// Time passing: both readings on by `by`.
    ///
    /// # Panics
    ///
    /// Where `by` is longer than some 292 years.
    pub fn pass(&self, by: Duration) {
        let by = i64::try_from(by.as_nanos()).expect("a span a test passes");
        self.wall.fetch_add(by, Ordering::SeqCst);
        self.monotonic.fetch_add(by, Ordering::SeqCst);
    }

    /// The wall clock set to `wall`, as the time service or a hand sets it:
    /// the monotonic reading stays where it is.
    ///
    /// # Panics
    ///
    /// Where `wall` is more than some 292 years from the epoch.
    pub fn set_wall(&self, wall: i128) {
        let wall = i64::try_from(wall).expect("a moment within 292 years of the epoch");
        self.wall.store(wall, Ordering::SeqCst);
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Clock for Stepped {
    fn unix_timestamp_nanos(&self) -> i128 {
        i128::from(self.wall.load(Ordering::SeqCst))
    }

    fn monotonic_nanos(&self) -> i128 {
        i128::from(self.monotonic.load(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_are_the_same_reading_floored() {
        assert_eq!(
            Stepped::at(1_800_000_000_999_999_999).unix_seconds(),
            1_800_000_000
        );
        assert_eq!(Stepped::at(-1).unix_seconds(), -1);
    }

    #[cfg(feature = "std")]
    #[test]
    fn the_system_clock_is_after_this_was_written() {
        assert!(SystemClock.unix_seconds() > 1_790_000_000);
    }

    #[cfg(feature = "std")]
    #[test]
    fn the_monotonic_reading_never_goes_back() {
        let first = SystemClock.monotonic_nanos();
        assert!(SystemClock.monotonic_nanos() >= first);
    }

    /// A device's clock that keeps the wall clock alone.
    struct WallAlone;

    impl Clock for WallAlone {
        fn unix_timestamp_nanos(&self) -> i128 {
            42
        }
    }

    #[test]
    fn a_serial_is_the_clock_above_the_last_and_one_above_it_otherwise() {
        assert_eq!(serial(5, 9), 9, "the clock, above the last");
        assert_eq!(serial(9, 9), 10, "one above, the clock not moved");
        assert_eq!(serial(9, 3), 10, "one above, the clock set back");
        assert_eq!(serial(0, -7), 1, "a reading before the epoch counts none");
        assert_eq!(serial(u64::MAX, 0), u64::MAX, "held at the last there is");
        assert_eq!(serial(0, i128::MAX), u64::MAX);
    }

    #[test]
    fn a_clock_that_keeps_the_wall_clock_alone_answers_it_monotonically() {
        assert_eq!(WallAlone.monotonic_nanos(), 42);
    }

    #[test]
    fn a_stepped_clock_moves_when_stepped_and_its_wall_clock_set_apart() {
        fn shared<T: Send + Sync>(_: &T) {}
        let clock = Stepped::at(1_000);
        shared(&clock);
        assert_eq!(
            (clock.unix_timestamp_nanos(), clock.monotonic_nanos()),
            (1_000, 0)
        );
        clock.pass(Duration::from_nanos(10));
        assert_eq!(
            (clock.unix_timestamp_nanos(), clock.monotonic_nanos()),
            (1_010, 10)
        );
        clock.set_wall(-3_600 * NANOS_A_SECOND);
        assert_eq!(
            clock.unix_timestamp_nanos(),
            -3_600 * NANOS_A_SECOND,
            "set back"
        );
        assert_eq!(clock.monotonic_nanos(), 10, "the monotonic reading stays");
        clock.pass(Duration::from_secs(1));
        assert_eq!(clock.monotonic_nanos(), NANOS_A_SECOND + 10);
    }

    #[cfg(feature = "std")]
    #[test]
    fn a_reading_handed_on_reads_the_clock_it_was_made_of_as_it_is_stepped() {
        let stepped = std::sync::Arc::new(Stepped::default());
        let clock: std::sync::Arc<dyn Clock> = stepped.clone();
        let now = reading(&clock);
        drop(clock);
        assert_eq!(now(), 0);
        stepped.pass(Duration::from_millis(3));
        assert_eq!(now(), 3_000_000, "the clock held, as stepped");
    }

    #[test]
    fn a_span_is_added_in_nanoseconds_and_read_back_between_two_readings() {
        assert_eq!(nanos(Duration::from_millis(2)), 2_000_000);
        assert_eq!(
            nanos(Duration::MAX),
            i128::from(u64::MAX) * NANOS_A_SECOND + 999_999_999
        );
        let at = 7 * NANOS_A_SECOND;
        let later = at + nanos(Duration::from_millis(1_500));
        assert_eq!(between(at, later), Duration::from_millis(1_500));
        assert_eq!(
            between(later, at),
            Duration::ZERO,
            "none where it is not after"
        );
        assert_eq!(between(i128::MIN, i128::MAX).as_secs(), u64::MAX, "at most");
    }
}
