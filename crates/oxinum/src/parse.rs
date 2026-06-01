//! Universal number parser that auto-detects the numeric format.

use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_float::DBig;
use oxinum_int::IBig;
use oxinum_rational::RBig;
use std::str::FromStr;

/// The result of parsing a numeric string with [`parse`].
///
/// The variant reflects the detected format of the input.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedNumber {
    /// An integer value (no decimal point, no fraction slash).
    Integer(IBig),
    /// A rational value (contained a `/`).
    Rational(RBig),
    /// A decimal float value (contained a `.`).
    Float(DBig),
}

impl std::fmt::Display for ParsedNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "{v}"),
            Self::Rational(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
        }
    }
}

/// Parse a numeric string, auto-detecting integer, rational, or float format.
///
/// Detection rules (checked in order):
/// 1. If the string contains `/`, it is parsed as a rational (e.g. `"3/4"`).
/// 2. If the string contains `.`, `e`, or `E`, it is parsed as a float
///    (e.g. `"1.25"`, `"1.5e10"`).
/// 3. Otherwise it is parsed as an integer (e.g. `"42"`, `"-7"`).
///
/// # Errors
///
/// Returns `OxiNumError::Parse` if the string does not match any recognised
/// numeric format.
///
/// # Examples
///
/// ```
/// use oxinum::{parse, ParsedNumber};
///
/// assert!(matches!(parse("42").unwrap(), ParsedNumber::Integer(_)));
/// assert!(matches!(parse("3/4").unwrap(), ParsedNumber::Rational(_)));
/// assert!(matches!(parse("1.25").unwrap(), ParsedNumber::Float(_)));
/// ```
pub fn parse(s: &str) -> OxiNumResult<ParsedNumber> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(OxiNumError::Parse("empty input".into()));
    }

    if trimmed.contains('/') {
        let r = RBig::from_str(trimmed)
            .map_err(|e| OxiNumError::Parse(format!("invalid rational '{trimmed}': {e}").into()))?;
        return Ok(ParsedNumber::Rational(r));
    }

    if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
        let f = DBig::from_str(trimmed)
            .map_err(|e| OxiNumError::Parse(format!("invalid float '{trimmed}': {e}").into()))?;
        return Ok(ParsedNumber::Float(f));
    }

    let i = IBig::from_str(trimmed)
        .map_err(|e| OxiNumError::Parse(format!("invalid integer '{trimmed}': {e}").into()))?;
    Ok(ParsedNumber::Integer(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer() {
        match parse("42").expect("ok") {
            ParsedNumber::Integer(v) => assert_eq!(v, IBig::from(42)),
            other => panic!("expected Integer, got {other:?}"),
        }
    }

    #[test]
    fn parse_negative_integer() {
        match parse("-7").expect("ok") {
            ParsedNumber::Integer(v) => assert_eq!(v, IBig::from(-7)),
            other => panic!("expected Integer, got {other:?}"),
        }
    }

    #[test]
    fn parse_rational() {
        match parse("3/4").expect("ok") {
            ParsedNumber::Rational(v) => {
                assert_eq!(v.numerator(), &IBig::from(3));
            }
            other => panic!("expected Rational, got {other:?}"),
        }
    }

    #[test]
    fn parse_float() {
        match parse("1.25").expect("ok") {
            ParsedNumber::Float(v) => assert!(v.to_string().starts_with("1.25")),
            other => panic!("expected Float, got {other:?}"),
        }
    }

    #[test]
    fn parse_scientific() {
        match parse("1.5e3").expect("ok") {
            ParsedNumber::Float(_) => {}
            other => panic!("expected Float, got {other:?}"),
        }
    }

    #[test]
    fn parse_empty_errors() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }

    #[test]
    fn parse_invalid_errors() {
        assert!(parse("hello").is_err());
        assert!(parse("1/0/2").is_err());
    }

    #[test]
    fn parse_trims_whitespace() {
        match parse("  42  ").expect("ok") {
            ParsedNumber::Integer(v) => assert_eq!(v, IBig::from(42)),
            other => panic!("expected Integer, got {other:?}"),
        }
    }

    #[test]
    fn parsed_number_display() {
        assert_eq!(parse("42").expect("ok").to_string(), "42");
        assert_eq!(parse("3/4").expect("ok").to_string(), "3/4");
        assert!(parse("1.25").expect("ok").to_string().starts_with("1.25"));
    }
}
