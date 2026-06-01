//! Mathematical constants at arbitrary precision.
//!
//! These thin wrappers delegate to `oxinum-float`'s constant computation,
//! providing a discoverable `oxinum::constants::pi(n)` entry point.

use oxinum_float::DBig;

/// Returns pi to `precision` significant decimal digits.
///
/// # Examples
///
/// ```
/// let pi = oxinum::constants::pi(50);
/// assert!(pi.to_string().starts_with("3.14159265358979"));
/// ```
pub fn pi(precision: usize) -> DBig {
    oxinum_float::compute_pi(precision)
}

/// Returns Euler's number *e* to `precision` significant decimal digits.
///
/// # Examples
///
/// ```
/// let e = oxinum::constants::e(50);
/// assert!(e.to_string().starts_with("2.71828182845904"));
/// ```
pub fn e(precision: usize) -> DBig {
    oxinum_float::compute_e(precision)
}

/// Returns ln(2) to `precision` significant decimal digits.
///
/// # Examples
///
/// ```
/// let ln2 = oxinum::constants::ln2(50);
/// assert!(ln2.to_string().starts_with("0.69314718055994"));
/// ```
pub fn ln2(precision: usize) -> DBig {
    oxinum_float::compute_ln2(precision)
}

/// Returns sqrt(2) to `precision` significant decimal digits.
///
/// # Examples
///
/// ```
/// let s = oxinum::constants::sqrt2(30);
/// assert!(s.to_string().starts_with("1.41421356237"));
/// ```
pub fn sqrt2(precision: usize) -> DBig {
    use std::str::FromStr;
    let two = DBig::from_str("2.0").expect("valid literal");
    oxinum_float::sqrt(&two, precision).expect("sqrt(2) always succeeds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_value() {
        assert!(pi(40).to_string().starts_with("3.14159265358979"));
    }

    #[test]
    fn e_value() {
        assert!(e(40).to_string().starts_with("2.71828182845904"));
    }

    #[test]
    fn ln2_value() {
        assert!(ln2(40).to_string().starts_with("0.69314718055994"));
    }

    #[test]
    fn sqrt2_value() {
        assert!(sqrt2(30).to_string().starts_with("1.41421356237"));
    }
}
