//! Free-function wrappers providing `FromRadix` / `ToRadix` semantics for
//! `UBig` / `IBig`.
//!
//! Due to Rust's orphan rule we cannot implement foreign traits on foreign
//! types; instead we expose free functions that match the trait signatures.

use crate::{IBig, UBig};
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// FromRadix
// ---------------------------------------------------------------------------

/// Parse a `UBig` from `src` in the given `radix` (2..=36).
///
/// # Errors
///
/// Returns `OxiNumError::Parse` on invalid digits or
/// `OxiNumError::InvalidRadix` if `radix` is out of range.
pub fn ubig_from_radix(src: &str, radix: u32) -> OxiNumResult<UBig> {
    if !(2..=36).contains(&radix) {
        return Err(OxiNumError::InvalidRadix(radix));
    }
    UBig::from_str_radix(src, radix).map_err(|e| OxiNumError::Parse(format!("{e}").into()))
}

/// Parse an `IBig` from `src` in the given `radix` (2..=36).
///
/// # Errors
///
/// Returns `OxiNumError::Parse` on invalid digits or
/// `OxiNumError::InvalidRadix` if `radix` is out of range.
pub fn ibig_from_radix(src: &str, radix: u32) -> OxiNumResult<IBig> {
    if !(2..=36).contains(&radix) {
        return Err(OxiNumError::InvalidRadix(radix));
    }
    IBig::from_str_radix(src, radix).map_err(|e| OxiNumError::Parse(format!("{e}").into()))
}

// ---------------------------------------------------------------------------
// ToRadix
// ---------------------------------------------------------------------------

/// Format a `UBig` as a string in the given `radix` (2..=36).
///
/// # Errors
///
/// Returns `OxiNumError::InvalidRadix` if `radix` is out of range.
pub fn ubig_to_radix(value: &UBig, radix: u32) -> OxiNumResult<String> {
    if !(2..=36).contains(&radix) {
        return Err(OxiNumError::InvalidRadix(radix));
    }
    Ok(format!("{}", value.in_radix(radix)))
}

/// Format an `IBig` as a string in the given `radix` (2..=36).
///
/// # Errors
///
/// Returns `OxiNumError::InvalidRadix` if `radix` is out of range.
pub fn ibig_to_radix(value: &IBig, radix: u32) -> OxiNumResult<String> {
    if !(2..=36).contains(&radix) {
        return Err(OxiNumError::InvalidRadix(radix));
    }
    Ok(format!("{}", value.in_radix(radix)))
}

// ---------------------------------------------------------------------------
// Convenience predicates
// ---------------------------------------------------------------------------

/// Returns `true` if the `UBig` is zero.
#[inline]
pub fn ubig_is_zero(v: &UBig) -> bool {
    *v == UBig::ZERO
}

/// Returns `true` if the `UBig` is one.
#[inline]
pub fn ubig_is_one(v: &UBig) -> bool {
    *v == UBig::ONE
}

/// Returns `true` if the `IBig` is zero.
#[inline]
pub fn ibig_is_zero(v: &IBig) -> bool {
    *v == IBig::ZERO
}

/// Returns `true` if the `IBig` is one.
#[inline]
pub fn ibig_is_one(v: &IBig) -> bool {
    *v == IBig::ONE
}

/// Returns the sign of the `IBig`.
pub fn ibig_signum(v: &IBig) -> oxinum_core::Sign {
    if *v >= IBig::ZERO {
        oxinum_core::Sign::Positive
    } else {
        oxinum_core::Sign::Negative
    }
}

/// Returns the absolute value of the `IBig`.
pub fn ibig_abs(v: &IBig) -> IBig {
    if *v >= IBig::ZERO {
        v.clone()
    } else {
        -v.clone()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ubig_is_zero_test() {
        assert!(ubig_is_zero(&UBig::ZERO));
        assert!(!ubig_is_zero(&UBig::ONE));
    }

    #[test]
    fn ubig_is_one_test() {
        assert!(ubig_is_one(&UBig::ONE));
        assert!(!ubig_is_one(&UBig::ZERO));
    }

    #[test]
    fn ibig_is_zero_test() {
        assert!(ibig_is_zero(&IBig::ZERO));
        assert!(!ibig_is_zero(&IBig::ONE));
    }

    #[test]
    fn ibig_signum_test() {
        assert_eq!(ibig_signum(&IBig::from(5)), oxinum_core::Sign::Positive);
        assert_eq!(ibig_signum(&IBig::from(-3)), oxinum_core::Sign::Negative);
        assert_eq!(ibig_signum(&IBig::ZERO), oxinum_core::Sign::Positive);
    }

    #[test]
    fn ibig_abs_test() {
        assert_eq!(ibig_abs(&IBig::from(-42)), IBig::from(42));
        assert_eq!(ibig_abs(&IBig::from(42)), IBig::from(42));
        assert_eq!(ibig_abs(&IBig::ZERO), IBig::ZERO);
    }

    #[test]
    fn ubig_from_radix_hex() {
        let v = ubig_from_radix("ff", 16).expect("valid hex");
        assert_eq!(v, UBig::from(255u32));
    }

    #[test]
    fn ibig_from_radix_binary() {
        let v = ibig_from_radix("-1010", 2).expect("valid binary");
        assert_eq!(v, IBig::from(-10));
    }

    #[test]
    fn ubig_to_radix_hex() {
        let s = ubig_to_radix(&UBig::from(255u32), 16).expect("valid radix");
        assert_eq!(s, "ff");
    }

    #[test]
    fn radix_roundtrip() {
        for radix in [2, 8, 10, 16, 36] {
            let original = UBig::from(123_456_789u64);
            let s = ubig_to_radix(&original, radix).expect("to_radix");
            let parsed = ubig_from_radix(&s, radix).expect("from_radix");
            assert_eq!(original, parsed, "roundtrip failed for radix {radix}");
        }
    }

    #[test]
    fn invalid_radix_returns_error() {
        assert!(ubig_from_radix("123", 1).is_err());
        assert!(ubig_from_radix("123", 37).is_err());
        assert!(ubig_to_radix(&UBig::from(42u32), 0).is_err());
    }
}
