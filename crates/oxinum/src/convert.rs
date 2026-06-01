//! Cross-type conversions between OxiNum numeric types.
//!
//! These functions convert between integers, floats, and rationals,
//! with explicit precision control where lossy conversions are involved.

use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_float::DBig;
use oxinum_int::{IBig, UBig};
use oxinum_rational::RBig;
use std::str::FromStr;

/// Convert an integer (`IBig`) to a decimal float (`DBig`).
///
/// This is always exact.
///
/// # Examples
///
/// ```
/// use oxinum::convert::int_to_float;
/// use oxinum::IBig;
/// let f = int_to_float(&IBig::from(42));
/// assert_eq!(f.to_string(), "42");
/// ```
pub fn int_to_float(n: &IBig) -> DBig {
    DBig::from(n.clone())
}

/// Convert an integer (`IBig`) to an exact rational (`RBig`).
///
/// # Examples
///
/// ```
/// use oxinum::convert::int_to_rational;
/// use oxinum::IBig;
/// let r = int_to_rational(&IBig::from(7));
/// assert_eq!(r.to_string(), "7");
/// ```
pub fn int_to_rational(n: &IBig) -> RBig {
    RBig::from(n.clone())
}

/// Convert a rational (`RBig`) to a decimal float (`DBig`) at the given
/// number of significant decimal digits.
///
/// # Examples
///
/// ```
/// use oxinum::convert::rational_to_float;
/// use oxinum::{IBig, RBig};
/// use oxinum::UBig;
/// // 1/4 = 0.25
/// let r = RBig::from_parts(IBig::from(1), UBig::from(4u32));
/// let f = rational_to_float(&r, 10);
/// assert!(f.to_string().starts_with("0.25"));
/// ```
pub fn rational_to_float(r: &RBig, precision: usize) -> DBig {
    let num = DBig::from(r.numerator().clone());
    let den_int: IBig = r.denominator().clone().into();
    let den = DBig::from(den_int);
    // Divide at the requested precision
    let num_p = num.with_precision(precision.max(1)).value();
    let den_p = den.with_precision(precision.max(1)).value();
    (&num_p / &den_p).with_precision(precision.max(1)).value()
}

/// Convert a decimal float (`DBig`) to an exact rational (`RBig`).
///
/// Since `DBig` is a finite decimal, the conversion is exact:
/// a value with `n` fractional digits becomes `significand / 10^n`.
///
/// # Examples
///
/// ```
/// use oxinum::convert::float_to_rational;
/// use oxinum::DBig;
/// use std::str::FromStr;
/// // 0.25 = 1/4
/// let f = DBig::from_str("0.25").unwrap();
/// let r = float_to_rational(&f).unwrap();
/// assert_eq!(r.to_string(), "1/4");
/// ```
pub fn float_to_rational(f: &DBig) -> OxiNumResult<RBig> {
    // DBig is significand * 10^exponent.
    let repr = f.repr();
    let significand = repr.significand().clone();
    let exponent = repr.exponent();

    if exponent >= 0 {
        // Integer value: significand * 10^exponent
        let ten = IBig::from(10);
        let mut value = significand;
        for _ in 0..exponent {
            value *= &ten;
        }
        Ok(RBig::from(value))
    } else {
        // significand / 10^(-exponent)
        let ten = UBig::from(10u32);
        let denom = ten.pow((-exponent) as usize);
        Ok(RBig::from_parts(significand, denom))
    }
}

/// Convert a rational (`RBig`) to an integer (`IBig`) by truncation
/// (rounding toward zero).
///
/// # Examples
///
/// ```
/// use oxinum::convert::rational_to_int;
/// use oxinum::{IBig, RBig, UBig};
/// // 7/3 truncates to 2
/// let r = RBig::from_parts(IBig::from(7), UBig::from(3u32));
/// assert_eq!(rational_to_int(&r), IBig::from(2));
/// ```
pub fn rational_to_int(r: &RBig) -> IBig {
    r.trunc()
}

/// Parse a decimal float (`DBig`) from a string.
///
/// # Errors
///
/// Returns `OxiNumError::Parse` if the string is not a valid decimal.
///
/// # Examples
///
/// ```
/// use oxinum::convert::float_from_str;
/// let f = float_from_str("3.14").unwrap();
/// assert!(f.to_string().starts_with("3.14"));
/// ```
pub fn float_from_str(s: &str) -> OxiNumResult<DBig> {
    DBig::from_str(s).map_err(|e| OxiNumError::Parse(format!("{e}").into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_to_float_basic() {
        let f = int_to_float(&IBig::from(42));
        assert_eq!(f.to_string(), "42");
    }

    #[test]
    fn int_to_rational_basic() {
        let r = int_to_rational(&IBig::from(7));
        assert_eq!(r.to_string(), "7");
    }

    #[test]
    fn rational_to_float_quarter() {
        let r = RBig::from_parts(IBig::from(1), UBig::from(4u32));
        let f = rational_to_float(&r, 10);
        assert!(f.to_string().starts_with("0.25"), "got {f}");
    }

    #[test]
    fn rational_to_float_third() {
        let r = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        let f = rational_to_float(&r, 10);
        assert!(f.to_string().starts_with("0.3333"), "got {f}");
    }

    #[test]
    fn float_to_rational_quarter() {
        let f = DBig::from_str("0.25").expect("ok");
        let r = float_to_rational(&f).expect("ok");
        assert_eq!(r.to_string(), "1/4");
    }

    #[test]
    fn float_to_rational_integer() {
        let f = DBig::from_str("42").expect("ok");
        let r = float_to_rational(&f).expect("ok");
        assert_eq!(r.to_string(), "42");
    }

    #[test]
    fn float_to_rational_roundtrip() {
        // 0.125 = 1/8
        let f = DBig::from_str("0.125").expect("ok");
        let r = float_to_rational(&f).expect("ok");
        assert_eq!(r, RBig::from_parts(IBig::from(1), UBig::from(8u32)));
    }

    #[test]
    fn rational_to_int_truncate() {
        let r = RBig::from_parts(IBig::from(7), UBig::from(3u32));
        assert_eq!(rational_to_int(&r), IBig::from(2));
    }

    #[test]
    fn float_from_str_basic() {
        let f = float_from_str("3.14").expect("ok");
        assert!(f.to_string().starts_with("3.14"));
    }

    #[test]
    fn float_from_str_invalid() {
        assert!(float_from_str("not a number").is_err());
    }
}
