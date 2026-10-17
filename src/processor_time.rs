//! The processor time a thread or this whole process has run, as the
//! operating system counts it: only while a thread runs, never while it
//! waits for an event or for a processor. What a measurement of it says is
//! the work done, the same however busy the machine is — what a wall clock
//! across a wake cannot say, because a woken thread on a loaded machine
//! waits its turn. The millisecond rule is a millisecond *apart from load*
//! (CONTRIBUTING.md), and this is the time load cannot reach. Compiled for
//! tests only, with `test-support`; the operating system's counters are
//! read in [`crate::system`], the crate's one file of system calls.
//!
//! On Windows the count is cycles (`QueryThreadCycleTime`,
//! `QueryProcessCycleTime`), never a duration by themselves; a reading turns
//! them into one at the rate the thread's cycles were seen to run against
//! the clock over twenty spins of a millisecond, the fastest of them, so a
//! spin a preemption cut short does not count. Elsewhere it is
//! `CLOCK_THREAD_CPUTIME_ID` and `CLOCK_PROCESS_CPUTIME_ID` in nanoseconds.

#[cfg(not(any(windows, unix)))]
compile_error!("processor time is read on Windows and on unix only");

use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scope {
    Thread,
    Process,
}

/// One reading of a thread's or the process's processor time; only the
/// difference of two of the same scope means anything.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessorTime {
    scope: Scope,
    count: u64,
}

impl ProcessorTime {
    /// The calling thread's processor time now.
    ///
    /// # Panics
    /// Where the operating system does not count it, which a test of the
    /// millisecond rule cannot do without.
    #[must_use]
    pub fn thread() -> Self {
        Self::now(Scope::Thread)
    }

    /// This process's processor time now, every thread of it summed.
    ///
    /// # Panics
    /// As [`ProcessorTime::thread`].
    #[must_use]
    pub fn process() -> Self {
        Self::now(Scope::Process)
    }

    /// What was run in this reading's scope since it was taken: the calling
    /// thread's for a [`ProcessorTime::thread`], which must be read on the
    /// thread that took it.
    ///
    /// # Panics
    /// As [`ProcessorTime::thread`].
    #[must_use]
    pub fn elapsed(self) -> Duration {
        Self::now(self.scope).since(self)
    }

    /// What was run between `earlier` and this reading.
    ///
    /// # Panics
    /// Where the two are of different scopes.
    #[must_use]
    pub fn since(self, earlier: Self) -> Duration {
        assert_eq!(self.scope, earlier.scope, "a thread's and a process's time");
        duration(self.count.saturating_sub(earlier.count))
    }

    fn now(scope: Scope) -> Self {
        Self {
            scope,
            count: count(scope).expect("the operating system counts processor time"),
        }
    }
}

#[cfg(windows)]
fn count(scope: Scope) -> Option<u64> {
    match scope {
        Scope::Thread => crate::system::thread_cycles(),
        Scope::Process => crate::system::process_cycles(),
    }
}

#[cfg(unix)]
fn count(scope: Scope) -> Option<u64> {
    match scope {
        Scope::Thread => crate::system::thread_cpu_nanos(),
        Scope::Process => crate::system::process_cpu_nanos(),
    }
}

#[cfg(windows)]
fn duration(cycles: u64) -> Duration {
    let nanos = u128::from(cycles) * 1_000_000_000 / u128::from(cycles_a_second());
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

#[cfg(unix)]
fn duration(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

// The cycles a thread runs a second, the most seen over twenty spins of a
// millisecond: a spin preempted counts fewer cycles for its time.
#[cfg(windows)]
fn cycles_a_second() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static RATE: OnceLock<u64> = OnceLock::new();
    *RATE.get_or_init(|| {
        (0..20)
            .map(|_| {
                let started = Instant::now();
                let from = ProcessorTime::thread().count;
                while started.elapsed() < Duration::from_millis(1) {
                    std::hint::spin_loop();
                }
                let cycles = ProcessorTime::thread().count.saturating_sub(from);
                let nanos = started.elapsed().as_nanos().max(1);
                u64::try_from(u128::from(cycles) * 1_000_000_000 / nanos).unwrap_or(u64::MAX)
            })
            .max()
            .unwrap_or(1)
            .max(1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn a_spin_is_counted_and_a_sleep_is_not() {
        let spun = ProcessorTime::thread();
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(20) {
            std::hint::spin_loop();
        }
        let spent = spun.elapsed();
        let wall = started.elapsed();
        // Never more than the wall clock; less where the spin was preempted.
        assert!(
            spent > Duration::ZERO && spent <= wall + Duration::from_millis(1),
            "a spin of {wall:?} counted {spent:?}"
        );

        let slept = ProcessorTime::thread();
        std::thread::sleep(Duration::from_millis(50));
        let waited = slept.elapsed();
        assert!(
            waited < Duration::from_millis(5),
            "a sleep of 50 ms counted {waited:?}"
        );
    }

    #[test]
    fn the_process_counts_every_thread() {
        let from = ProcessorTime::process();
        let counted = std::thread::spawn(|| {
            let spun = ProcessorTime::thread();
            let started = Instant::now();
            while started.elapsed() < Duration::from_millis(20) {
                std::hint::spin_loop();
            }
            spun.elapsed()
        });
        let theirs = counted.join().expect("spun");
        let spent = from.elapsed();
        assert!(
            spent >= theirs,
            "the process {spent:?}, one of its threads {theirs:?}"
        );
    }
}
