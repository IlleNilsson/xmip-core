//! One scalar value of a field.

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// One scalar value of a field — the shape a promoted property and a structured
/// content field both take. Defined once here so `context` and `contract` share
/// it rather than each redeclaring it: a contract reads and writes a field as
/// this type, and a Message's context holds it. Not `Eq`: the `Decimal(f64)`
/// variant has no total equality.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarValue {
    Null,
    Bool(bool),
    Integer(i64),
    Decimal(f64),
    Text(String),
    Binary(Vec<u8>),
}

impl ScalarValue {
    /// The value as it is written: text as it is, a boolean as `true` or
    /// `false`, a number as Rust prints it. The one rendering, for a filter's
    /// comparison and a path's write alike. `None` for `Null` and `Binary`,
    /// which have no text; a caller that gives `Null` one — empty text, or
    /// absence — matches it before asking.
    #[must_use]
    pub fn text(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Text(text) => Some(Cow::Borrowed(text)),
            Self::Bool(true) => Some(Cow::Borrowed("true")),
            Self::Bool(false) => Some(Cow::Borrowed("false")),
            Self::Integer(integer) => Some(Cow::Owned(format!("{integer}"))),
            Self::Decimal(decimal) => Some(Cow::Owned(format!("{decimal}"))),
            Self::Null | Self::Binary(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scalar_is_written_one_way_and_null_and_bytes_have_no_text() {
        let text = |value: ScalarValue| value.text().map(Cow::into_owned);
        assert_eq!(
            text(ScalarValue::Text("Order".into())).as_deref(),
            Some("Order")
        );
        assert_eq!(text(ScalarValue::Bool(true)).as_deref(), Some("true"));
        assert_eq!(text(ScalarValue::Integer(-12)).as_deref(), Some("-12"));
        assert_eq!(text(ScalarValue::Decimal(2.5)).as_deref(), Some("2.5"));
        assert_eq!(text(ScalarValue::Null), None);
        assert_eq!(text(ScalarValue::Binary(Vec::from([1]))), None);
    }
}
