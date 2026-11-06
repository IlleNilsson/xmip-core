//! A thread waiting under a lock until another wakes it or a due time comes:
//! the one timed wait of the estate's message path (the owner, 2026-10-16).
//!
//! It is `std::sync::Condvar`'s shape — wait while a condition holds, under
//! the caller's `Mutex`, woken by a notify — with one difference that is the
//! reason it exists: a due time is kept to the microsecond. On Windows a
//! `Condvar` waits on an address with a timeout counted in milliseconds and
//! ends on the system tick, so a wait due in 2 ms ended up to 9-14 ms late
//! (measured 2026-10-16). Here, on Windows, each waiting thread waits on its
//! own kernel event and its own high-resolution waitable timer, created with
//! `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` (Windows 10, version 1803, and
//! later), not by raising the system's timer resolution. A notified thread
//! keeps the priority boost Windows gives a satisfied wait. Every other
//! target waits on `std::sync::Condvar`, as before.
//!
//! A wait takes the `Mutex` its guard came from, because on Windows the
//! lock is let go of and taken again here, and std gives a guard no way to
//! name its lock. A guard from another lock unlocks that one and returns
//! this one's: wrong, never unsound.
//!
//! [`Revision`] is the wait most of the estate needs: a count one thread
//! raises and others wait past, built on [`Wake`] once.

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[cfg(not(windows))]
mod portable;
mod revision;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use portable::Waiters;
pub use revision::Revision;
#[cfg(windows)]
use windows::Waiters;

/// Threads waiting under one lock, woken by a notify or their due time.
#[derive(Default)]
pub struct Wake {
    waiters: Waiters,
}

impl core::fmt::Debug for Wake {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Wake")
    }
}

impl Wake {
    /// None waiting.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            waiters: Waiters::new(),
        }
    }

    /// Let go of `guard`'s lock, `mutex`, until notified, and take it again.
    /// A wake may come with nothing notified: wait in a loop, or use
    /// [`Wake::wait_while`].
    pub fn wait<'a, T>(&self, mutex: &'a Mutex<T>, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        self.waiters.wait(mutex, guard, None).0
    }

    /// Wait while `condition` holds.
    pub fn wait_while<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        mut guard: MutexGuard<'a, T>,
        mut condition: impl FnMut(&mut T) -> bool,
    ) -> MutexGuard<'a, T> {
        while condition(&mut guard) {
            guard = self.wait(mutex, guard);
        }
        guard
    }

    /// Wait until notified or `until`, whichever is first; the lock taken
    /// again, and whether `until` came. Past already, it does not wait.
    pub fn wait_until<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        guard: MutexGuard<'a, T>,
        until: Instant,
    ) -> (MutexGuard<'a, T>, bool) {
        if Instant::now() >= until {
            return (guard, true);
        }
        self.waiters.wait(mutex, guard, Some(until))
    }

    /// Wait until notified or for `timeout`; whether it ran out. A timeout
    /// too long to count waits until notified.
    pub fn wait_timeout<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        guard: MutexGuard<'a, T>,
        timeout: Duration,
    ) -> (MutexGuard<'a, T>, bool) {
        match Instant::now().checked_add(timeout) {
            Some(until) => self.wait_until(mutex, guard, until),
            None => (self.wait(mutex, guard), false),
        }
    }

    /// Wait while `condition` holds, for at most `timeout`; whether it ran
    /// out with `condition` still holding.
    pub fn wait_timeout_while<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        guard: MutexGuard<'a, T>,
        timeout: Duration,
        condition: impl FnMut(&mut T) -> bool,
    ) -> (MutexGuard<'a, T>, bool) {
        match Instant::now().checked_add(timeout) {
            Some(until) => self.wait_until_while(mutex, guard, until, condition),
            None => (self.wait_while(mutex, guard, condition), false),
        }
    }

    /// Wait while `condition` holds, until `until` at the latest; whether
    /// it came with `condition` still holding.
    pub fn wait_until_while<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        mut guard: MutexGuard<'a, T>,
        until: Instant,
        mut condition: impl FnMut(&mut T) -> bool,
    ) -> (MutexGuard<'a, T>, bool) {
        while condition(&mut guard) {
            let (again, came) = self.wait_until(mutex, guard, until);
            guard = again;
            if came {
                let holds = condition(&mut guard);
                return (guard, holds);
            }
        }
        (guard, false)
    }

    /// Wake one thread waiting, the longest waiting first where the target
    /// keeps an order.
    pub fn notify_one(&self) {
        self.waiters.notify_one();
    }

    /// Wake every thread waiting.
    pub fn notify_all(&self) {
        self.waiters.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::vec::Vec;

    /// One notify wakes one waiter; notifying all wakes the rest.
    #[test]
    fn notify_one_wakes_one_and_notify_all_the_rest() {
        // How many wait, and whether they were let go; the waiters' wake,
        // and the one that tells this thread how many wait.
        let shared = Arc::new((Mutex::new((0_u32, false)), Wake::new(), Wake::new()));
        let waiters: Vec<_> = (0..4)
            .map(|_| {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    let (mutex, wake, told) = &*shared;
                    let mut guard = relock(mutex);
                    guard.0 += 1;
                    told.notify_all();
                    // Woken without being let go: only by a notify.
                    let mut guard = wake.wait(mutex, guard);
                    guard.0 -= 1;
                    told.notify_all();
                    let mut guard = wake.wait_while(mutex, guard, |state| !state.1);
                    guard.1 = true;
                })
            })
            .collect();
        let (mutex, wake, told) = &*shared;
        let within = Duration::from_secs(5);
        let (state, _) = told.wait_timeout_while(mutex, relock(mutex), within, |s| s.0 < 4);
        drop(state);
        wake.notify_one();
        let (state, _) = told.wait_timeout_while(mutex, relock(mutex), within, |s| s.0 > 3);
        assert_eq!(state.0, 3, "one woken");
        let (mut state, came) =
            told.wait_timeout_while(mutex, state, Duration::from_millis(50), |s| s.0 == 3);
        assert!(came, "no other woken");
        state.1 = true;
        drop(state);
        wake.notify_all();
        for waiter in waiters {
            waiter.join().expect("woken");
        }
        assert_eq!(relock(mutex).0, 0, "all woken");
    }

    /// A notify that races a due time is the waiter's, never lost and never
    /// left over: the wait after it still waits its whole time.
    #[test]
    fn a_notify_racing_the_due_time_is_not_left_over() {
        let shared = Arc::new((Mutex::new(()), Wake::new()));
        let notifier = {
            let shared = Arc::clone(&shared);
            thread::spawn(move || {
                for _ in 0..2_000 {
                    shared.1.notify_one();
                    thread::yield_now();
                }
            })
        };
        let (mutex, wake) = &*shared;
        while !notifier.is_finished() {
            let _ = wake.wait_timeout(mutex, relock(mutex), Duration::from_micros(50));
        }
        notifier.join().expect("notified");
        let began = Instant::now();
        let (guard, came) = wake.wait_timeout(mutex, relock(mutex), Duration::from_millis(20));
        drop(guard);
        assert!(came, "nothing left to wake it");
        assert!(
            began.elapsed() >= Duration::from_millis(20),
            "{:?}",
            began.elapsed()
        );
    }

    /// A due time already past does not wait.
    #[test]
    fn a_due_time_past_does_not_wait() {
        let wake = Wake::new();
        let mutex = Mutex::new(());
        let (guard, came) = wake.wait_until(&mutex, relock(&mutex), Instant::now());
        drop(guard);
        assert!(came);
    }

    fn relock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
