//! Failures accumulated and reported once: a summary — how many items a
//! stage took, how many failed — and each failure in detail.
//!
//! The owner, 2026-10-27: *Failures during processing (Receive, Promote,
//! Transform, Processing, Demote, Send) shall be accumulated. The caller
//! shall get a summary and detailed information of what failed. Each failure
//! would be tedious to handle*, and *The same goes for Prepare, summarize
//! failures, then report back* (ADR-0013, amendment 2026-10-27). A stage
//! goes on past an item that fails, gathers every failure here, and fails
//! once. Debatching reports with it today (`xmip-core-runtime`'s `split`);
//! the other stages are open problem 51. The shape is a draft for the owner.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// One item that failed: where it stands among the items, from one, what
/// it is and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failed {
    /// Its position among the stage's items, from one.
    pub position: u64,
    /// Why, in words.
    pub reason: String,
}

/// Every failure of one stage over its items, reported once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failures {
    /// The stage, in the node's stage words or the act's name: `debatch`.
    pub stage: String,
    /// What one item is called, in the singular: `record`.
    pub item: String,
    /// How many items the stage took.
    pub total: u64,
    /// Each that failed, in order.
    pub failed: Vec<Failed>,
}

impl Failures {
    /// No failure yet, of `stage` over items called `item`.
    #[must_use]
    pub fn new(stage: impl Into<String>, item: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            item: item.into(),
            total: 0,
            failed: Vec::new(),
        }
    }

    /// The next item taken: its position, from one.
    pub fn took(&mut self) -> u64 {
        self.total += 1;
        self.total
    }

    /// The item at `position` failed, for `reason`.
    pub fn fail(&mut self, position: u64, reason: impl Into<String>) {
        self.failed.push(Failed {
            position,
            reason: reason.into(),
        });
    }

    /// Whether nothing failed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.failed.is_empty()
    }
}

/// The summary, then each failure: `debatch: 2 of 5 records failed; record
/// 2: …; record 4: …`.
impl fmt::Display for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = if self.total == 1 { "" } else { "s" };
        write!(
            f,
            "{}: {} of {} {}{plural} failed",
            self.stage,
            self.failed.len(),
            self.total,
            self.item
        )?;
        for failed in &self.failed {
            write!(f, "; {} {}: {}", self.item, failed.position, failed.reason)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Failures;
    use alloc::string::ToString;

    #[test]
    fn a_summary_says_how_many_failed_of_how_many_then_each() {
        let mut failures = Failures::new("debatch", "record");
        for _ in 0..5 {
            let at = failures.took();
            if at.is_multiple_of(2) {
                failures.fail(at, "no record");
            }
        }
        assert_eq!(
            failures.to_string(),
            "debatch: 2 of 5 records failed; record 2: no record; record 4: no record"
        );
        assert!(!failures.is_empty());
    }
}
