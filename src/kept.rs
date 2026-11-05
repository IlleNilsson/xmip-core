//! What an opened step keeps between Streams: the working state a unit
//! works in — its pieces, its compressor, its decoder — made once and
//! reset, never made again for every Stream.
//!
//! On 2026-11-05 each prepare unit made its state anew for every Stream.
//! Carrying 1 KiB both ways, making it was ten times the unit's work: zlib
//! built about 200 KiB of compressor tables to compress it (160 µs), and
//! once a unit had freed state that large, the operating system's heap gave
//! the pages back, so every unit after it paid again for each fresh piece —
//! text normalization went from 15 µs to over 140. Kept, a step makes its
//! state on its first Stream and works in it from then on.
//!
//! A step may carry several Streams at once; each takes a state of its own
//! and gives it back, so a step keeps as many as it ever carried at once.

mod reading;
mod writing;

pub use reading::Reading;
pub use writing::Writing;

use alloc::boxed::Box;
use alloc::vec::Vec;
use std::sync::{Mutex, PoisonError};

/// The idle states of one opened step, and how to make another.
pub struct Kept<T> {
    idle: Mutex<Vec<T>>,
    make: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T: Send> Kept<T> {
    /// No state yet; `make` makes one when a Stream finds none idle.
    pub fn new(make: impl Fn() -> T + Send + Sync + 'static) -> Self {
        Self {
            idle: Mutex::new(Vec::new()),
            make: Box::new(make),
        }
    }

    /// `work` done in an idle state, or a new one, given back after it.
    /// A state is given back as `work` leaves it, so `work` resets what it
    /// relies on before it starts. A state `work` panicked in is dropped.
    pub fn with<R>(&self, work: impl FnOnce(&mut T) -> R) -> R {
        let idle = self
            .idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop();
        let mut state = idle.unwrap_or_else(|| (self.make)());
        let done = work(&mut state);
        self.idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(state);
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_state_is_made_once_and_kept_for_the_next_stream() {
        let made = Arc::new(AtomicUsize::new(0));
        let counting = made.clone();
        let kept = Kept::new(move || {
            counting.fetch_add(1, Ordering::Relaxed);
            Vec::<u8>::with_capacity(16)
        });
        kept.with(|state| state.push(1));
        let seen = kept.with(|state| state.clone());
        assert_eq!(seen, [1], "the second Stream works in the first's state");
        assert_eq!(made.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn streams_at_once_each_take_a_state_of_their_own() {
        let kept = Kept::new(Vec::<u8>::new);
        kept.with(|outer| {
            outer.push(1);
            kept.with(|inner| assert!(inner.is_empty(), "not the outer's"));
        });
        assert_eq!(kept.idle.lock().expect("idle").len(), 2);
    }
}
