//! Failures accumulated per stage and reported like a compiler's: what,
//! where and, where it is known, why.
//!
//! The owner, 2026-10-27: *Failures during processing (Receive, Promote,
//! Transform, Processing, Demote, Send) shall be accumulated. The caller
//! shall get a summary and detailed information of what failed. Each failure
//! would be tedious to handle*; *The same goes for Prepare, summarize
//! failures, then report back*; and *All failures in each stage through Xmip
//! shall report what, where and if possible why. Like a compiler, just
//! batched per stage* (ADR-0013, amendment 2026-10-27). A stage goes on past
//! an item that fails, gathers every failure in its [`Failures`], and fails
//! once. Debatching reports with it today (`xmip-core-runtime`'s `split`);
//! the other stages are open problem 51. The field names are drafts for the
//! owner; the shape is decided.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Where inside the data a failure is: each part where it is known.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Place {
    /// The item's position among the stage's items, from one: a record.
    pub position: Option<u64>,
    /// The byte offset within the item.
    pub offset: Option<u64>,
    /// The line, from one.
    pub line: Option<u64>,
    /// The column, from one.
    pub column: Option<u64>,
    /// A path into the item: an XPath, a JSON Pointer.
    pub path: Option<String>,
}

/// One failure: what, where, and why where it is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failed {
    /// What: a stable code, `record-refused`.
    pub code: String,
    /// What, in words.
    pub message: String,
    /// Where: the stage, in the node's stage words or the act's name.
    pub stage: String,
    /// Where: the artifact by name — a Location, a Port, a unit, a step.
    pub artifact: Option<String>,
    /// Where inside the data.
    pub at: Place,
    /// Why, in words, where it is known.
    pub cause: Option<String>,
}

/// One stage's failures over its items, reported once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failures {
    /// The stage: `debatch`.
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

    /// A failure of this stage: `code` and `message`, at the item
    /// `position`, of `artifact` where one is named; its place and cause
    /// added on the answer.
    pub fn fail(
        &mut self,
        (code, message): (impl Into<String>, impl Into<String>),
        artifact: Option<String>,
        position: u64,
    ) -> &mut Failed {
        self.failed.push(Failed {
            code: code.into(),
            message: message.into(),
            stage: self.stage.clone(),
            artifact,
            at: Place {
                position: Some(position),
                ..Place::default()
            },
            cause: None,
        });
        let last = self.failed.len() - 1;
        &mut self.failed[last]
    }

    /// How many items failed.
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    /// Whether nothing failed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.failed.is_empty()
    }
}

/// A summary line, then a block per failure, as a compiler prints:
///
/// ```text
/// error: debatch: 2 of 5 records failed
/// error[record-refused]: the line 'bad!' is no record
///   --> debatch, unit 'Lines' of 'Units', record 2, line 1, column 4
///   = cause: …
/// ```
impl fmt::Display for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = if self.total == 1 { "" } else { "s" };
        write!(
            f,
            "error: {}: {} of {} {}{plural} failed",
            self.stage,
            self.failed.len(),
            self.total,
            self.item
        )?;
        for failed in &self.failed {
            write!(f, "\n{}", Block(failed, &self.item))?;
        }
        Ok(())
    }
}

/// One failure's block, its item called by the batch's word.
struct Block<'a>(&'a Failed, &'a str);

impl fmt::Display for Block<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(failed, item) = self;
        write!(
            f,
            "error[{}]: {}\n  --> {}",
            failed.code, failed.message, failed.stage
        )?;
        if let Some(artifact) = &failed.artifact {
            write!(f, ", {artifact}")?;
        }
        let at = &failed.at;
        for (word, value) in [
            (*item, at.position),
            ("byte", at.offset),
            ("line", at.line),
            ("column", at.column),
        ] {
            if let Some(value) = value {
                write!(f, ", {word} {value}")?;
            }
        }
        if let Some(path) = &at.path {
            write!(f, ", at {path}")?;
        }
        if let Some(cause) = &failed.cause {
            write!(f, "\n  = cause: {cause}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Failures;
    use alloc::string::{String, ToString};

    #[test]
    fn a_summary_line_then_a_block_per_failure_like_a_compiler() {
        let mut failures = Failures::new("debatch", "record");
        for _ in 0..5 {
            let at = failures.took();
            if at.is_multiple_of(2) {
                let unit = Some(String::from("unit 'Lines' of 'Units'"));
                let failed = failures.fail(("record-refused", "no record"), unit, at);
                failed.at.line = Some(1);
                failed.at.column = Some(4);
                if at == 4 {
                    failed.cause = Some("a field is missing".into());
                }
            }
        }
        assert_eq!(failures.failed_count(), 2);
        assert_eq!(
            failures.to_string(),
            "error: debatch: 2 of 5 records failed\n\
             error[record-refused]: no record\n  \
             --> debatch, unit 'Lines' of 'Units', record 2, line 1, column 4\n\
             error[record-refused]: no record\n  \
             --> debatch, unit 'Lines' of 'Units', record 4, line 1, column 4\n  \
             = cause: a field is missing"
        );
    }
}
