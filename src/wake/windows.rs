//! Windows: each waiting thread waits on its own kernel event, set by the
//! notify that takes it off the list, and its own high-resolution waitable
//! timer, armed for the due time (the owner, 2026-10-16). Both are made once
//! per thread and closed when it ends; the calls are `crate::system`'s.
//! Where the timer cannot be made — before Windows 10, version 1803 — the
//! event is waited on with a timeout in milliseconds, rounded up.

use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use crate::system::{Event, Timer, either};

/// One thread's event and timer.
#[derive(Clone)]
struct Own {
    event: Arc<Event>,
    timer: Rc<Timer>,
}

std::thread_local! {
    static OWN: Own = Own::new();
}

impl Own {
    fn new() -> Self {
        Self {
            event: Arc::new(Event::new()),
            timer: Rc::new(Timer::new()),
        }
    }

    /// This thread's; a pair of its own for a wait made while the thread
    /// ends, when its own are gone.
    fn of_this_thread() -> Self {
        OWN.try_with(Self::clone).unwrap_or_else(|_| Self::new())
    }

    /// Wait until the event is set or `until`; whether `until` came.
    fn wait_until(&self, until: Instant) -> bool {
        loop {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return true;
            }
            if !self.timer.arm(left) {
                return !self.event.wait_millis(left);
            }
            if either(&self.event, &self.timer) {
                self.timer.cancel();
                return false;
            }
            // The timer, or a failure: looked at again against the clock,
            // so an early end is never taken for the due time.
        }
    }
}

/// The threads waiting, longest first, by the event that wakes each.
#[derive(Default)]
pub(super) struct Waiters {
    waiting: Mutex<VecDeque<Arc<Event>>>,
}

impl Waiters {
    pub(super) const fn new() -> Self {
        Self {
            waiting: Mutex::new(VecDeque::new()),
        }
    }

    fn waiting(&self) -> MutexGuard<'_, VecDeque<Arc<Event>>> {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wait until notified or `until`; whether `until` came. Listed before
    /// the caller's lock is let go of, so a notify made under that lock
    /// after it is never missed.
    pub(super) fn wait<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        guard: MutexGuard<'a, T>,
        until: Option<Instant>,
    ) -> (MutexGuard<'a, T>, bool) {
        let own = Own::of_this_thread();
        self.waiting().push_back(Arc::clone(&own.event));
        drop(guard);
        let mut came = until.is_some_and(|until| own.wait_until(until));
        if until.is_none() {
            own.event.wait();
        }
        // Out of time, and a notify took it off the list meanwhile: that
        // notify is this thread's, and its set is waited for, so it is not
        // left for the next wait.
        if came && !self.withdraw(&own.event) {
            own.event.wait();
            came = false;
        }
        (mutex.lock().unwrap_or_else(PoisonError::into_inner), came)
    }

    /// Take `event` off the list; whether it was on it.
    fn withdraw(&self, event: &Arc<Event>) -> bool {
        let mut waiting = self.waiting();
        let at = waiting.iter().position(|each| Arc::ptr_eq(each, event));
        at.and_then(|at| waiting.remove(at)).is_some()
    }

    pub(super) fn notify_one(&self) {
        let first = self.waiting().pop_front();
        if let Some(event) = first {
            event.set();
        }
    }

    pub(super) fn notify_all(&self) {
        let all = std::mem::take(&mut *self.waiting());
        for event in all {
            event.set();
        }
    }
}
