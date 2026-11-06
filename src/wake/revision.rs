//! A count that only rises, and the threads waiting for it to: how one
//! thread tells others that something changed, at once, without their
//! looking again and again. Revision 0 is nothing raised yet.

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use super::Wake;
use crate::lock::Locked;

/// A revision raised by one thread and waited on by others.
#[derive(Debug, Default)]
pub struct Revision {
    count: Mutex<u64>,
    raised: Wake,
}

impl Revision {
    /// Revision 0, none waiting.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            count: Mutex::new(0),
            raised: Wake::new(),
        }
    }

    fn count(&self) -> MutexGuard<'_, u64> {
        self.count.locked()
    }

    /// Raise it, waking every thread waiting; the revision now. It never
    /// comes back to 0.
    pub fn raise(&self) -> u64 {
        let mut count = self.count();
        *count = count.wrapping_add(1).max(1);
        self.raised.notify_all();
        *count
    }

    /// The revision now.
    #[must_use]
    pub fn now(&self) -> u64 {
        *self.count()
    }

    /// Wait up to `within` for the revision to pass `after`; the revision
    /// then.
    pub fn wait(&self, after: u64, within: Duration) -> u64 {
        self.wait_unless(after, within, || false)
    }

    /// Wait up to `within` for the revision to pass `after`, or until
    /// `stopped` says so when [`Revision::wake`] wakes the wait; the
    /// revision then.
    pub fn wait_unless(&self, after: u64, within: Duration, stopped: impl Fn() -> bool) -> u64 {
        let (count, _) =
            self.raised
                .wait_timeout_while(&self.count, self.count(), within, |count| {
                    *count <= after && !stopped()
                });
        *count
    }

    /// Wake every wait without raising, so each looks at what stops it.
    pub fn wake(&self) {
        let _count = self.count();
        self.raised.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Instant;

    #[test]
    fn a_raise_wakes_a_wait_at_once_and_none_is_lost() {
        let revision = Arc::new(Revision::new());
        assert_eq!(revision.now(), 0);
        let waiting = {
            let revision = Arc::clone(&revision);
            thread::spawn(move || revision.wait(0, Duration::from_secs(10)))
        };
        thread::sleep(Duration::from_millis(20));
        let raised = Instant::now();
        assert_eq!(revision.raise(), 1);
        assert_eq!(waiting.join().expect("waited"), 1);
        assert!(raised.elapsed() < Duration::from_secs(1));
        assert_eq!(revision.wait(0, Duration::from_secs(10)), 1, "past already");
    }

    #[test]
    fn a_wait_ends_by_its_time_or_when_woken_to_stop() {
        let revision = Arc::new(Revision::new());
        assert_eq!(revision.wait(0, Duration::from_millis(5)), 0);
        let stopping = Arc::new(AtomicBool::new(false));
        let waiting = {
            let (revision, stopping) = (Arc::clone(&revision), Arc::clone(&stopping));
            thread::spawn(move || {
                revision.wait_unless(0, Duration::from_secs(30), || {
                    stopping.load(Ordering::SeqCst)
                })
            })
        };
        thread::sleep(Duration::from_millis(20));
        let stopped = Instant::now();
        stopping.store(true, Ordering::SeqCst);
        revision.wake();
        assert_eq!(waiting.join().expect("waited"), 0);
        assert!(stopped.elapsed() < Duration::from_secs(1));
    }
}
