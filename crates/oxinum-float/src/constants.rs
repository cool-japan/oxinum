//! High-precision mathematical constants: pi, e, ln(2).
//!
//! These are computed by parsing pre-computed decimal digit strings at
//! the requested precision (via `DBig::from_str`), rather than running
//! expensive iterative algorithms at runtime.  The pre-stored strings
//! contain 200 decimal digits, which is sufficient for most use cases.
//! For higher precision, callers can use the `dashu-float` API directly.

use crate::elementary::truncate_decimal_str;
use crate::DBig;
use std::str::FromStr;

/// 200 decimal digits of pi.
const PI_200: &str = "3.14159265358979323846264338327950288419716939937510\
58209749445923078164062862089986280348253421170679\
82148086513282306647093844609550582231725359408128\
48111745028410270193852110555964462294895493038196";

/// 200 decimal digits of e (Euler's number).
const E_200: &str = "2.71828182845904523536028747135266249775724709369995\
95749669676277240766303535475945713821785251664274\
27466391932003059921817413596629043572900334295260\
59563073813232862794349076323382988075319525101901";

/// 200 decimal digits of ln(2).
const LN2_200: &str = "0.69314718055994530941723212145817656807550013436025\
52541206800094933936219696947156058633269964186875\
42001481020570685733685520235758130557032670751635\
07526908163220817225884781126976816832788376249023";

/// Compute pi to the given number of decimal digits of precision.
///
/// The returned `DBig` has at most `precision` significant digits.
/// If `precision > 200`, the result contains 200 digits (the
/// pre-stored resolution).
///
/// # Examples
///
/// ```
/// use oxinum_float::compute_pi;
/// let pi = compute_pi(50);
/// let s = pi.to_string();
/// assert!(s.starts_with("3.14159265358979"));
/// ```
pub fn compute_pi(precision: usize) -> DBig {
    parse_at_precision(PI_200, precision)
}

/// Compute Euler's number *e* to the given number of decimal digits.
///
/// # Examples
///
/// ```
/// use oxinum_float::compute_e;
/// let e = compute_e(50);
/// let s = e.to_string();
/// assert!(s.starts_with("2.71828182845904"));
/// ```
pub fn compute_e(precision: usize) -> DBig {
    parse_at_precision(E_200, precision)
}

/// Compute ln(2) to the given number of decimal digits.
///
/// # Examples
///
/// ```
/// use oxinum_float::compute_ln2;
/// let ln2 = compute_ln2(50);
/// let s = ln2.to_string();
/// assert!(s.starts_with("0.69314718055994"));
/// ```
pub fn compute_ln2(precision: usize) -> DBig {
    parse_at_precision(LN2_200, precision)
}

/// Parse a constant string, truncated to `precision` significant digits.
fn parse_at_precision(src: &str, precision: usize) -> DBig {
    let prec = precision.clamp(1, 200);
    let truncated = truncate_decimal_str(src, prec);
    DBig::from_str(&truncated).expect("pre-stored constant is a valid decimal literal")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_starts_correctly() {
        let pi = compute_pi(50);
        let s = pi.to_string();
        assert!(s.starts_with("3.14159265358979"), "pi = {s}");
    }

    #[test]
    fn e_starts_correctly() {
        let e = compute_e(50);
        let s = e.to_string();
        assert!(s.starts_with("2.71828182845904"), "e = {s}");
    }

    #[test]
    fn ln2_starts_correctly() {
        let ln2 = compute_ln2(50);
        let s = ln2.to_string();
        assert!(s.starts_with("0.69314718055994"), "ln2 = {s}");
    }

    #[test]
    fn pi_precision_10() {
        let pi = compute_pi(10);
        let s = pi.to_string();
        assert!(s.starts_with("3.141592653"), "pi(10) = {s}");
    }

    #[test]
    fn constants_are_nonzero() {
        assert_ne!(compute_pi(20).to_string(), "0");
        assert_ne!(compute_e(20).to_string(), "0");
        assert_ne!(compute_ln2(20).to_string(), "0");
    }
}
