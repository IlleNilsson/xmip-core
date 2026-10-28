//! What the process holds in memory, as the allocator a test installs
//! counts it: the figure a test of bounded memory holds — that a Stream far
//! larger than a chunk passes through a gate, a unit or the Ledger with no
//! more held than the pieces it is cut into (`runtime-model.md` section 3,
//! *What proves it*). Compiled for tests only, with `test-support`; the
//! allocator is in [`crate::system`], the crate's one file of unsafe code.
//!
//! A test program installs it once:
//!
//! ```ignore
//! #[global_allocator]
//! static HELD: xmip_core::held::Counted = xmip_core::held::Counted;
//! ```
//!
//! The count is the whole process's, every thread's, so a test that holds
//! a figure of it runs with no other test beside it: a test program of its
//! own with one test, or the test started again alone.

use std::sync::atomic::Ordering;

pub use crate::system::Counted;
use crate::system::{HELD, MOST};

/// What the process holds now.
#[must_use]
pub fn now() -> usize {
    HELD.load(Ordering::Relaxed)
}

/// The most the process has held since [`reset`].
#[must_use]
pub fn most() -> usize {
    MOST.load(Ordering::Relaxed)
}

/// Start counting the most held again from what is held now.
pub fn reset() {
    MOST.store(now(), Ordering::Relaxed);
}

/// The most held above what was held when `run` began, while it ran, and
/// what `run` returned.
pub fn above<T>(run: impl FnOnce() -> T) -> (usize, T) {
    reset();
    let before = now();
    let returned = run();
    (most().saturating_sub(before), returned)
}
