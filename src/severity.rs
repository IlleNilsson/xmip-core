//! How much a record matters to whoever reads it.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Information,
    Warning,
    Error,
}
