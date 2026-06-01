//! Elementary mathematical functions: exp, ln, sqrt, pow.
//!
//! These wrap `dashu-float`'s built-in methods with `OxiNumResult` error
//! handling and precision-aware context management.

use crate::{DBig, OxiNumError, OxiNumResult};
use dashu_float::round::mode::HalfEven;
use std::str::FromStr;

/// Compute `e^x` with the given number of significant decimal digits.
///
/// # Examples
///
/// ```
/// use oxinum_float::exp;
/// use std::str::FromStr;
/// let x = dashu_float::DBig::from_str("1.0").unwrap();
/// let result = exp(&x, 30).unwrap();
/// let s = result.to_string();
/// assert!(s.starts_with("2.71828"), "exp(1) = {}", s);
/// ```
pub fn exp(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    if precision == 0 {
        return Err(OxiNumError::Precision("precision must be > 0".into()));
    }
    // Special case: exp(0) = 1
    let zero = DBig::from_str("0.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()))?;
    if *x == zero {
        return DBig::from_str("1.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()));
    }
    let guard_bits = precision * 4 + 20;
    let fbig = convert_dbig_to_fbig(x, guard_bits);
    let result = fbig.exp();
    let dbig = fbig_to_dbig(&result, precision);
    Ok(truncate_to_precision(dbig, precision))
}

/// Compute `ln(x)` (natural logarithm) with the given precision.
///
/// # Errors
///
/// Returns `OxiNumError::Precision` if `x <= 0`.
///
/// # Examples
///
/// ```
/// use oxinum_float::ln;
/// use std::str::FromStr;
/// let x = dashu_float::DBig::from_str("2.718281828459045").unwrap();
/// let result = ln(&x, 30).unwrap();
/// let s = result.to_string();
/// assert!(s.starts_with("0.99999") || s.starts_with("1.0000"), "ln(e) = {}", s);
/// ```
pub fn ln(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    if precision == 0 {
        return Err(OxiNumError::Precision("precision must be > 0".into()));
    }
    // Check sign by comparing with a precision-bearing zero
    let zero = DBig::from_str("0.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()))?;
    if *x <= zero {
        return Err(OxiNumError::Precision("ln(x) requires x > 0".into()));
    }
    // Special case: ln(1) = 0
    let one = DBig::from_str("1.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()))?;
    if *x == one {
        return DBig::from_str("0.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()));
    }
    let guard_bits = precision * 4 + 20;
    let fbig = convert_dbig_to_fbig(x, guard_bits);
    let result = fbig.ln();
    let dbig = fbig_to_dbig(&result, precision);
    Ok(truncate_to_precision(dbig, precision))
}

/// Compute the square root of `x` with the given precision.
///
/// # Errors
///
/// Returns `OxiNumError::Precision` if `x < 0`.
///
/// # Examples
///
/// ```
/// use oxinum_float::sqrt;
/// use std::str::FromStr;
/// let x = dashu_float::DBig::from_str("2.0").unwrap();
/// let result = sqrt(&x, 30).unwrap();
/// let s = result.to_string();
/// assert!(s.starts_with("1.4142135"), "sqrt(2) = {}", s);
/// ```
pub fn sqrt(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    if precision == 0 {
        return Err(OxiNumError::Precision("precision must be > 0".into()));
    }
    let zero = DBig::from_str("0.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()))?;
    if *x < zero {
        return Err(OxiNumError::Precision("sqrt(x) requires x >= 0".into()));
    }
    if *x == zero {
        return Ok(zero);
    }
    let guard_bits = precision * 4 + 20;
    let fbig = convert_dbig_to_fbig(x, guard_bits);
    let result = dashu_base::SquareRoot::sqrt(&fbig);
    let dbig = fbig_to_dbig(&result, precision);
    Ok(truncate_to_precision(dbig, precision))
}

/// Compute `base^exp` for arbitrary float exponents.
///
/// Uses the identity `base^exp = e^(exp * ln(base))`.
///
/// # Errors
///
/// Returns `OxiNumError::Precision` if `base <= 0`.
///
/// # Examples
///
/// ```
/// use oxinum_float::pow;
/// use std::str::FromStr;
/// let base = dashu_float::DBig::from_str("2.0").unwrap();
/// let exp = dashu_float::DBig::from_str("10.0").unwrap();
/// let result = pow(&base, &exp, 20).unwrap();
/// let s = result.to_string();
/// assert!(s.starts_with("1024"), "2^10 = {}", s);
/// ```
pub fn pow(base: &DBig, exponent: &DBig, precision: usize) -> OxiNumResult<DBig> {
    if precision == 0 {
        return Err(OxiNumError::Precision("precision must be > 0".into()));
    }
    let zero = DBig::from_str("0.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()))?;
    if *exponent == zero {
        return DBig::from_str("1.0").map_err(|e| OxiNumError::Parse(format!("{e}").into()));
    }
    if *base <= zero {
        return Err(OxiNumError::Precision(
            "pow(base, exp) requires base > 0".into(),
        ));
    }
    let guard_bits = precision * 4 + 20;
    let fbig_base = convert_dbig_to_fbig(base, guard_bits);
    let fbig_exp = convert_dbig_to_fbig(exponent, guard_bits);
    let result = fbig_base.powf(&fbig_exp);
    let dbig = fbig_to_dbig(&result, precision);
    Ok(truncate_to_precision(dbig, precision))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a `DBig` (base-10) to an `FBig<HalfEven, 2>` at the given
/// number of binary digits of precision.
///
/// This ensures the resulting `FBig` always has a defined precision,
/// even for zero-valued inputs.
pub(crate) fn convert_dbig_to_fbig(
    value: &DBig,
    binary_precision: usize,
) -> dashu_float::FBig<HalfEven, 2> {
    // Ensure we have a decimal value with at least some digits of precision
    // by adding it to a high-precision zero. This avoids the "precision
    // cannot be 0" panic from dashu when the input has unlimited precision.
    let ctx = dashu_float::Context::<HalfEven>::new(binary_precision);
    let repr = value
        .clone()
        .with_rounding::<HalfEven>()
        .with_base_and_precision::<2>(binary_precision.max(10))
        .value();
    // Re-apply the context precision to ensure it's set
    let result_repr = repr.repr().clone();
    dashu_float::FBig::from_repr(result_repr, ctx)
}

/// Convert an `FBig<HalfEven, 2>` to `DBig`, handling the zero-precision edge case.
///
/// dashu's `to_decimal()` panics if the FBig has precision=0 (which happens
/// for exact zero results). We detect that and return "0.0" directly.
pub(crate) fn fbig_to_dbig(
    fbig: &dashu_float::FBig<HalfEven, 2>,
    decimal_precision: usize,
) -> DBig {
    // If digits() returns 0 (unlimited/zero), construct a precision-bearing result
    if fbig.digits() == 0 {
        // The value is exact zero (or similar). Return a precision-bearing zero.
        return DBig::from_str("0.0").expect("valid literal");
    }
    // Use with_base_and_precision to get a decimal representation at
    // the requested precision level.
    let decimal_digits = decimal_precision.max(5);
    fbig.clone()
        .with_base_and_precision::<10>(decimal_digits)
        .value()
        .with_rounding::<dashu_float::round::mode::HalfAway>()
}

/// Truncate a `DBig` to `n` significant decimal digits by re-parsing.
pub(crate) fn truncate_to_precision(value: DBig, precision: usize) -> DBig {
    let s = value.to_string();
    let truncated = truncate_decimal_str(&s, precision);
    DBig::from_str(&truncated).unwrap_or(value)
}

/// Truncate a decimal string to `n` significant digits.
///
/// Significant digits are counted as follows:
/// - For values >= 1 (integer part != "0"), all digits count.
/// - For values < 1 (integer part is "0"), leading zeros after the
///   decimal point are NOT significant (but are preserved in output).
pub(crate) fn truncate_decimal_str(src: &str, sig_digits: usize) -> String {
    let mut result = String::with_capacity(sig_digits + 10);
    let mut sig_count = 0;

    // Determine if the integer part is just "0" (or "-0")
    let trimmed = src.trim_start_matches('-');
    let integer_is_zero = trimmed.starts_with("0.") || trimmed == "0";

    // Track whether we've seen any nonzero digit (for leading-zero skip)
    let mut seen_nonzero = !integer_is_zero;

    for ch in src.chars() {
        if ch == '-' {
            result.push(ch);
            continue;
        }
        if ch == '.' {
            result.push(ch);
            continue;
        }
        // Stop at scientific notation
        if ch == 'e' || ch == 'E' {
            break;
        }
        if !ch.is_ascii_digit() {
            continue;
        }

        if !seen_nonzero && ch == '0' {
            // Leading zero -- not significant but kept in output
            result.push(ch);
            continue;
        }
        // From here, it's a significant digit
        seen_nonzero = true;
        sig_count += 1;
        result.push(ch);
        if sig_count >= sig_digits {
            break;
        }
    }

    // If we never produced any output past the sign, at least return "0"
    let content = result.trim_start_matches('-');
    if content.is_empty() {
        if result.starts_with('-') {
            return "-0".to_string();
        }
        return "0".to_string();
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_of_zero() {
        let x = DBig::from_str("0.0").expect("ok");
        let result = exp(&x, 30).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("1.0000") || s == "1", "exp(0) = {s}");
    }

    #[test]
    fn exp_of_one() {
        let x = DBig::from_str("1.0").expect("ok");
        let result = exp(&x, 30).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("2.71828"), "exp(1) = {s}");
    }

    #[test]
    fn ln_of_one() {
        let x = DBig::from_str("1.0").expect("ok");
        let result = ln(&x, 30).expect("ok");
        let s = result.to_string();
        // ln(1) = 0
        let s_clean = s.trim_start_matches('-');
        assert!(
            s_clean.starts_with("0") && !s_clean.starts_with("0.1"),
            "ln(1) = {s}"
        );
    }

    #[test]
    fn ln_negative_errors() {
        let x = DBig::from_str("-1.0").expect("ok");
        assert!(ln(&x, 30).is_err());
    }

    #[test]
    fn sqrt_of_four() {
        let x = DBig::from_str("4.0").expect("ok");
        let result = sqrt(&x, 30).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("2.0000") || s == "2", "sqrt(4) = {s}");
    }

    #[test]
    fn sqrt_of_two() {
        let x = DBig::from_str("2.0").expect("ok");
        let result = sqrt(&x, 30).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("1.4142135"), "sqrt(2) = {s}");
    }

    #[test]
    fn sqrt_negative_errors() {
        let x = DBig::from_str("-1.0").expect("ok");
        assert!(sqrt(&x, 30).is_err());
    }

    #[test]
    fn sqrt_of_zero() {
        let x = DBig::from_str("0.0").expect("ok");
        let result = sqrt(&x, 30).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("0"), "sqrt(0) = {s}");
    }

    #[test]
    fn pow_two_to_ten() {
        let base = DBig::from_str("2.0").expect("ok");
        let exponent = DBig::from_str("10.0").expect("ok");
        let result = pow(&base, &exponent, 20).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("1024"), "2^10 = {s}");
    }

    #[test]
    fn pow_zero_exponent() {
        let base = DBig::from_str("5.0").expect("ok");
        let exponent = DBig::from_str("0.0").expect("ok");
        let result = pow(&base, &exponent, 20).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("1"), "5^0 = {s}");
    }

    #[test]
    fn precision_zero_errors() {
        let x = DBig::from_str("1.0").expect("ok");
        assert!(exp(&x, 0).is_err());
        assert!(ln(&x, 0).is_err());
        assert!(sqrt(&x, 0).is_err());
        assert!(pow(&x, &x, 0).is_err());
    }

    #[test]
    fn truncate_leading_zeros() {
        let s = truncate_decimal_str("0.00123456789", 5);
        assert_eq!(s, "0.0012345");
    }

    #[test]
    fn truncate_integer_part() {
        let s = truncate_decimal_str("123.456789", 6);
        assert_eq!(s, "123.456");
    }

    #[test]
    fn truncate_negative() {
        let s = truncate_decimal_str("-3.14159", 4);
        assert_eq!(s, "-3.141");
    }
}
