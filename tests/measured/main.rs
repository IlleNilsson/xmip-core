//! The crate's timing program (`doc/governance/rust-style.md` section 5c):
//! the instruments the millisecond rule is measured with, and the timed
//! wait, held to a time. A plain `cargo test` leaves it out; the landing
//! runs it after every other test of the module, alone, one test at a time
//! (`cargo test --test measured --all-features -- --test-threads=1`), so
//! what is measured is the crate's work and not its neighbors':
//!
//! - `wake.rs`: a wait ends within a millisecond of its due time, and a
//!   notify wakes within a millisecond, apart from load;
//! - `processor_time.rs`: a spin is counted and a sleep is not;
//! - `spent.rs`: what a thread runs for a Message counts, and its I/O and
//!   its waits do not, a Stream's reads and its closing among that I/O.

mod processor_time;
mod spent;
mod wake;
