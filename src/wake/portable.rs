//! Every target but Windows: `std::sync::Condvar`, whose timed wait is
//! already kept to the operating system's own precision.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// The threads waiting, as std keeps them.
#[derive(Default)]
pub(super) struct Waiters {
    ready: Condvar,
}

impl Waiters {
    pub(super) const fn new() -> Self {
        Self {
            ready: Condvar::new(),
        }
    }

    /// Wait until notified or `until`; whether `until` came.
    pub(super) fn wait<'a, T>(
        &self,
        _mutex: &'a Mutex<T>,
        guard: MutexGuard<'a, T>,
        until: Option<Instant>,
    ) -> (MutexGuard<'a, T>, bool) {
        let Some(until) = until else {
            let guard = self
                .ready
                .wait(guard)
                .unwrap_or_else(PoisonError::into_inner);
            return (guard, false);
        };
        let left = until.saturating_duration_since(Instant::now());
        let (guard, _) = self
            .ready
            .wait_timeout(guard, left)
            .unwrap_or_else(PoisonError::into_inner);
        (guard, Instant::now() >= until)
    }

    pub(super) fn notify_one(&self) {
        self.ready.notify_one();
    }

    pub(super) fn notify_all(&self) {
        self.ready.notify_all();
    }
}
