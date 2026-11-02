//! The processor time a thread or this whole process has run, as the
//! operating system counts it: only while a thread runs, never while it
//! waits for an event or for a processor. What a measurement of it says is
//! the time the work ran — what a wall clock across a wake cannot say,
//! because a woken thread on a loaded machine waits its turn. The
//! millisecond rule is a millisecond *apart from load* (CONTRIBUTING.md),
//! and no other program's turn reaches this time: a node counts its own
//! share of each Message in it (`crate::spent`), and the estate's tests
//! measure with it. Load still reaches it one way, and it is real time:
//! the same work runs longer on a processor whose clock has fallen as all
//! its cores are busy, and beside another thread on the same core (on the
//! estate's i9-9880H, 2026-11-01, a fixed piece of work ran 393 µs alone and
//! 560–720 µs under a parallel build). The operating system's counters are
//! read in [`crate::system`], the crate's one file of system calls.
//!
//! On Windows the count is cycles (`QueryThreadCycleTime`,
//! `QueryProcessCycleTime`), read from the processor's time-stamp counter,
//! which runs at one rate whatever the core's clock where the processor's
//! counter is invariant, as x86 processors' have been since about 2008,
//! and is never a duration by itself; a reading turns them into one at the
//! rate that counter runs against the clock, the median of five spins of a
//! millisecond, which neither a preemption nor an interrupt moves (on
//! another processor, the rate the thread's own cycles were seen to run,
//! the most of five spins, so a spin a preemption cut short does not
//! count). That is measured
//! once a process, when it first asks ([`ProcessorTime::calibrate`]) — a
//! node as it starts, so it never lands in a Message's time. Elsewhere it is
//! `CLOCK_THREAD_CPUTIME_ID` and `CLOCK_PROCESS_CPUTIME_ID` in nanoseconds,
//! which need no rate. `GetThreadTimes` is not used: it moves a clock tick,
//! about 15.6 ms, at a time.

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

    /// The calling thread's processor time now; `None` where the operating
    /// system does not count it.
    #[must_use]
    pub fn try_thread() -> Option<Self> {
        count(Scope::Thread).map(|count| Self {
            scope: Scope::Thread,
            count,
        })
    }

    /// The rate a count is turned into a duration at, measured now where it
    /// was not yet (on Windows; nothing elsewhere): what a node does as it
    /// starts, so no reading in a Message's time measures it.
    pub fn calibrate() {
        #[cfg(windows)]
        let _ = cycles_a_second();
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

// The cycles a thread runs a second, measured once.
#[cfg(windows)]
fn cycles_a_second() -> u64 {
    use std::sync::OnceLock;

    static RATE: OnceLock<u64> = OnceLock::new();
    *RATE.get_or_init(|| rate().max(1))
}

// Where a thread's cycles are the time-stamp counter's (x86-64): that
// counter against the clock over five spins of a millisecond, the median.
// It ticks whether the thread runs or not, so a preemption or an interrupt
// during a spin does not move it, and a window an endpoint was preempted
// in is not the median. The thread's own cycles would count neither, and
// read a rate as much too low as the share of the millisecond it lost —
// every duration read at it as much too long (measured 2026-11-01: up to
// 0.8 % under a parallel build, the most of five spins; 0.2 % on an idle
// machine, the interrupts a thread's cycles leave out).
#[cfg(all(windows, target_arch = "x86_64"))]
fn rate() -> u64 {
    let mut rates = spins(crate::system::time_stamp);
    rates.sort_unstable();
    rates[2]
}

// Elsewhere the thread's own cycles, the most of five spins: a spin
// preempted counts fewer.
#[cfg(all(windows, not(target_arch = "x86_64")))]
fn rate() -> u64 {
    spins(|| ProcessorTime::thread().count)
        .into_iter()
        .max()
        .unwrap_or(1)
}

// What `count` counted a second against the clock in each of five spins of
// a millisecond.
#[cfg(windows)]
fn spins(count: fn() -> u64) -> [u64; 5] {
    use std::time::Instant;

    core::array::from_fn(|_| {
        let started = Instant::now();
        let from = count();
        while started.elapsed() < Duration::from_millis(1) {
            std::hint::spin_loop();
        }
        let counted = count().saturating_sub(from);
        let nanos = started.elapsed().as_nanos().max(1);
        u64::try_from(u128::from(counted) * 1_000_000_000 / nanos).unwrap_or(u64::MAX)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// The rate is the time-stamp counter's against the clock: over a
    /// sleep, where the thread runs nothing, the counter still ticks at it.
    #[cfg(all(windows, target_arch = "x86_64"))]
    #[test]
    fn the_rate_is_the_time_stamp_counter_s_whether_the_thread_runs_or_not() {
        let (started, from) = (Instant::now(), crate::system::time_stamp());
        std::thread::sleep(Duration::from_millis(200));
        let ticked = crate::system::time_stamp() - from;
        let slept = started.elapsed().as_nanos();
        let over_a_sleep = u128::from(ticked) * 1_000_000_000 / slept;
        let rate = u128::from(cycles_a_second());
        // Within one percent: a preemption between an endpoint's two
        // readings moves the sleep's own figure, never the rate.
        assert!(
            over_a_sleep.abs_diff(rate) * 100 < rate,
            "{rate} a second, {over_a_sleep} over a sleep"
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
