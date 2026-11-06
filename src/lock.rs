//! A lock taken even where another thread panicked holding it.
//!
//! A `std` lock a thread panicked under is poisoned, and taking it returns
//! an error holding the guard. Xmip's locks guard values every writer keeps
//! whole between its statements, so the value is still sound: the guard is
//! taken as it is, in one place, rather than each lock site recovering it.

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// A `Mutex` taken whether or not it is poisoned.
pub trait Locked<T: ?Sized> {
    /// The guard, waiting for the lock.
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> Locked<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// An `RwLock` read or written whether or not it is poisoned.
pub trait ReadWriteLocked<T: ?Sized> {
    /// A read guard, waiting for any writer.
    fn read_locked(&self) -> RwLockReadGuard<'_, T>;

    /// The write guard, waiting for every reader and writer.
    fn write_locked(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> ReadWriteLocked<T> for RwLock<T> {
    fn read_locked(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write_locked(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn a_lock_a_thread_panicked_under_is_still_taken_with_its_value() {
        let mutex = Arc::new(Mutex::new(1));
        let rw = Arc::new(RwLock::new(2));
        let (m, r) = (Arc::clone(&mutex), Arc::clone(&rw));
        let panicked = thread::spawn(move || {
            let _m = m.lock();
            let _r = r.write();
            panic!("held");
        })
        .join();
        assert!(panicked.is_err() && mutex.is_poisoned() && rw.is_poisoned());
        assert_eq!(*mutex.locked(), 1);
        assert_eq!(*rw.read_locked(), 2);
        *rw.write_locked() = 3;
        assert_eq!(*rw.read_locked(), 3);
    }
}
