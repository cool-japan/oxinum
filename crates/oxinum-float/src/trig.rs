//! Trigonometric and hyperbolic functions for arbitrary-precision floats.
//!
//! Implementations use Taylor series with argument reduction for
//! trig functions, and exp-based formulas for hyperbolic functions.
//! All functions accept `DBig` (base-10) inputs and return results
//! at the requested precision.

use crate::constants::compute_pi;
use crate::elementary::{convert_dbig_to_fbig, exp, fbig_to_dbig, truncate_to_precision};
use crate::{DBig, OxiNumError, OxiNumResult};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Trigonometric functions
// ---------------------------------------------------------------------------

/// Compute `sin(x)` using Taylor series with argument reduction.
///
/// # Examples
///
/// ```
/// use oxinum_float::sin;
/// use std::str::FromStr;
/// let x = dashu_float::DBig::from_str("0.0").unwrap();
/// let result = sin(&x, 30).unwrap();
/// let s = result.to_string();
/// assert!(s.starts_with("0"), "sin(0) = {}", s);
/// ```
pub fn sin(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x == zero {
        return Ok(zero);
    }
    // Use extra guard digits for intermediate computations
    let guard_prec = precision + 10;
    let reduced = reduce_argument(x, guard_prec);
    let result = sin_taylor(&reduced, guard_prec);
    Ok(truncate_to_precision(result, precision))
}

/// Compute `cos(x)` using Taylor series with argument reduction.
///
/// # Examples
///
/// ```
/// use oxinum_float::cos;
/// use std::str::FromStr;
/// let x = dashu_float::DBig::from_str("0.0").unwrap();
/// let result = cos(&x, 30).unwrap();
/// let s = result.to_string();
/// assert!(s.starts_with("1"), "cos(0) = {}", s);
/// ```
pub fn cos(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x == zero {
        return make_dbig("1.0");
    }
    let guard_prec = precision + 10;
    let reduced = reduce_argument(x, guard_prec);
    let result = cos_taylor(&reduced, guard_prec);
    Ok(truncate_to_precision(result, precision))
}

/// Compute `tan(x) = sin(x) / cos(x)`.
///
/// # Errors
///
/// Returns `OxiNumError::DivByZero` if `cos(x) = 0`.
pub fn tan(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let guard_prec = precision + 10;
    let s = sin(x, guard_prec)?;
    let c = cos(x, guard_prec)?;
    let zero = make_dbig("0.0")?;
    if c == zero {
        return Err(OxiNumError::DivByZero);
    }
    let result = &s / &c;
    Ok(truncate_to_precision(result, precision))
}

/// Compute `atan(x)` using dashu-float's binary conversion and Taylor
/// approach internally. For |x| <= 0.5 we use the series directly;
/// for larger |x| <= 1 we use the half-angle formula; for |x| > 1 we
/// use the identity `atan(x) = pi/2 - atan(1/x)`.
pub fn atan(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x == zero {
        return Ok(zero);
    }

    let guard_prec = precision + 15;
    let one = make_dbig("1.0")?;
    let neg_one = make_dbig("-1.0")?;

    let is_negative = *x < zero;
    let abs_x = if is_negative { x * &neg_one } else { x.clone() };

    let result = if abs_x > one {
        // atan(x) = pi/2 - atan(1/x) for x > 0
        let pi = compute_pi(guard_prec);
        let half_pi = &pi / &make_dbig("2.0")?;
        let recip = &one / &abs_x;
        let atan_recip = atan_small(&recip, guard_prec);
        &half_pi - &atan_recip
    } else {
        atan_small(&abs_x, guard_prec)
    };

    let final_result = if is_negative {
        &result * &neg_one
    } else {
        result
    };
    Ok(truncate_to_precision(final_result, precision))
}

/// Compute `atan2(y, x)` with proper quadrant handling.
///
/// Returns the angle in radians between the positive x-axis and the
/// point (x, y), in the range (-pi, pi].
pub fn atan2(y: &DBig, x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    let guard_prec = precision + 10;
    let pi = compute_pi(guard_prec);

    if *x > zero {
        let ratio = y / x;
        atan(&ratio, precision)
    } else if *x < zero && *y >= zero {
        let ratio = y / x;
        let a = atan(&ratio, guard_prec)?;
        Ok(truncate_to_precision(&a + &pi, precision))
    } else if *x < zero && *y < zero {
        let ratio = y / x;
        let a = atan(&ratio, guard_prec)?;
        Ok(truncate_to_precision(&a - &pi, precision))
    } else if *x == zero && *y > zero {
        let half_pi = &pi / &make_dbig("2.0")?;
        Ok(truncate_to_precision(half_pi, precision))
    } else if *x == zero && *y < zero {
        let neg_half_pi = &(&pi / &make_dbig("2.0")?) * &make_dbig("-1.0")?;
        Ok(truncate_to_precision(neg_half_pi, precision))
    } else {
        // x == 0 && y == 0 -- undefined, return 0
        Ok(zero)
    }
}

// ---------------------------------------------------------------------------
// Hyperbolic functions
// ---------------------------------------------------------------------------

/// Compute `sinh(x) = (e^x - e^(-x)) / 2`.
pub fn sinh(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let guard_prec = precision + 10;
    let exp_x = exp(x, guard_prec)?;
    let neg_x = x * &make_dbig("-1.0")?;
    let exp_neg_x = exp(&neg_x, guard_prec)?;
    let diff = &exp_x - &exp_neg_x;
    let result = &diff / &make_dbig("2.0")?;
    Ok(truncate_to_precision(result, precision))
}

/// Compute `cosh(x) = (e^x + e^(-x)) / 2`.
pub fn cosh(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let guard_prec = precision + 10;
    let exp_x = exp(x, guard_prec)?;
    let neg_x = x * &make_dbig("-1.0")?;
    let exp_neg_x = exp(&neg_x, guard_prec)?;
    let sum = &exp_x + &exp_neg_x;
    let result = &sum / &make_dbig("2.0")?;
    Ok(truncate_to_precision(result, precision))
}

/// Compute `tanh(x) = sinh(x) / cosh(x)`.
pub fn tanh(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let guard_prec = precision + 10;
    let s = sinh(x, guard_prec)?;
    let c = cosh(x, guard_prec)?;
    let result = &s / &c;
    Ok(truncate_to_precision(result, precision))
}

// ---------------------------------------------------------------------------
// Internal: Taylor series
// ---------------------------------------------------------------------------

/// Compute sin(x) via Taylor series: sum_{k=0}^{inf} (-1)^k * x^{2k+1} / (2k+1)!
///
/// Assumes |x| is small (after argument reduction). All arithmetic is
/// performed at `precision` significant digits to avoid precision loss.
fn sin_taylor(x: &DBig, precision: usize) -> DBig {
    let x = extend_precision(x, precision);
    let mut term = x.clone();
    let mut sum = x.clone();
    let x_sq = &x * &x;
    let neg_one = dbig_at_precision("-1.0", precision);

    for k in 1..=(precision as u32 + 20) {
        let denom_a = dbig_u32_at_precision(2 * k, precision);
        let denom_b = dbig_u32_at_precision(2 * k + 1, precision);
        term = &term * &x_sq;
        term = &term / &denom_a;
        term = &term / &denom_b;
        term = &term * &neg_one;
        sum = &sum + &term;

        if is_negligible(&term, precision) {
            break;
        }
    }
    sum
}

/// Compute cos(x) via Taylor series: sum_{k=0}^{inf} (-1)^k * x^{2k} / (2k)!
///
/// Assumes |x| is small (after argument reduction). All arithmetic is
/// performed at `precision` significant digits to avoid precision loss.
fn cos_taylor(x: &DBig, precision: usize) -> DBig {
    let x = extend_precision(x, precision);
    let one = dbig_at_precision("1.0", precision);
    let mut term = one.clone();
    let mut sum = one;
    let x_sq = &x * &x;
    let neg_one = dbig_at_precision("-1.0", precision);

    for k in 1..=(precision as u32 + 20) {
        let denom_a = dbig_u32_at_precision(2 * k - 1, precision);
        let denom_b = dbig_u32_at_precision(2 * k, precision);
        term = &term * &x_sq;
        term = &term / &denom_a;
        term = &term / &denom_b;
        term = &term * &neg_one;
        sum = &sum + &term;

        if is_negligible(&term, precision) {
            break;
        }
    }
    sum
}

/// Compute atan(x) for |x| <= 1 using argument reduction + Taylor series.
///
/// For faster convergence when x is close to 1, we use the identity:
///   atan(x) = 2 * atan(x / (1 + sqrt(1 + x^2)))
/// which reduces the argument closer to 0.
fn atan_small(x: &DBig, precision: usize) -> DBig {
    // Reduce argument: apply halving a few times for faster convergence
    let threshold = make_dbig("0.5").expect("valid literal");
    let one = make_dbig("1.0").expect("valid literal");

    let mut reduced = x.clone();
    let mut halvings = 0u32;

    while reduced > threshold {
        // atan(x) = 2 * atan(x / (1 + sqrt(1 + x^2)))
        let x_sq = &reduced * &reduced;
        let inner = &one + &x_sq;
        // Use dashu's sqrt via binary conversion
        let guard_bits = precision * 4 + 20;
        let fbig = convert_dbig_to_fbig(&inner, guard_bits);
        let sqrt_val = dashu_base::SquareRoot::sqrt(&fbig);
        let sqrt_dbig = fbig_to_dbig(&sqrt_val, precision);
        reduced = &reduced / &(&one + &sqrt_dbig);
        halvings += 1;

        if halvings > 50 {
            break; // safety limit
        }
    }

    let mut result = atan_taylor_core(&reduced, precision);

    // Undo halvings: multiply by 2^halvings
    for _ in 0..halvings {
        result = &result * &make_dbig("2.0").expect("valid literal");
    }

    result
}

/// Raw Taylor series for atan(x), valid for |x| < 1:
///
/// atan(x) = x - x^3/3 + x^5/5 - x^7/7 + ...
///
/// All arithmetic is performed at `precision` significant digits.
fn atan_taylor_core(x: &DBig, precision: usize) -> DBig {
    let x = extend_precision(x, precision);
    let mut power = x.clone();
    let mut sum = x.clone();
    let x_sq = &x * &x;
    let neg_one = dbig_at_precision("-1.0", precision);
    let mut sign = neg_one.clone();

    for k in 1..=(precision as u32 * 3 + 50) {
        power = &power * &x_sq;
        let denom = dbig_u32_at_precision(2 * k + 1, precision);
        let contrib = &power / &denom;
        sum = &sum + &(&contrib * &sign);
        sign = &sign * &neg_one;

        if is_negligible(&contrib, precision) {
            break;
        }
    }
    sum
}

// ---------------------------------------------------------------------------
// Internal: argument reduction
// ---------------------------------------------------------------------------

/// Reduce `x` modulo 2*pi so the Taylor series converges faster.
fn reduce_argument(x: &DBig, precision: usize) -> DBig {
    let pi = compute_pi(precision.min(195) + 5);
    let two = dbig_at_precision("2.0", precision + 5);
    let two_pi = (&pi * &two).with_precision(precision + 5).value();
    let zero = dbig_at_precision("0.0", precision + 5);

    if two_pi == zero {
        return x.clone();
    }

    // Compute x mod 2*pi
    let mut reduced = extend_precision(x, precision + 5);
    if reduced > zero {
        while reduced > two_pi {
            reduced = (&reduced - &two_pi).with_precision(precision + 5).value();
        }
    } else {
        while reduced < zero {
            reduced = (&reduced + &two_pi).with_precision(precision + 5).value();
        }
    }
    reduced
}

/// Check if a term is negligible for the given precision.
fn is_negligible(term: &DBig, precision: usize) -> bool {
    let s = term.to_string();
    let s = s.trim_start_matches('-');

    if let Some(dot_pos) = s.find('.') {
        let integer_part = &s[..dot_pos];
        if integer_part == "0" {
            let frac = &s[dot_pos + 1..];
            let leading_zeros = frac.chars().take_while(|&c| c == '0').count();
            return leading_zeros >= precision + 3;
        }
    }
    s == "0" || s == "0.0"
}

// ---------------------------------------------------------------------------
// Internal: helpers
// ---------------------------------------------------------------------------

/// Parse a `DBig` from a string literal. Returns `OxiNumResult`.
fn make_dbig(s: &str) -> OxiNumResult<DBig> {
    DBig::from_str(s).map_err(|e| OxiNumError::Parse(format!("{e}").into()))
}

/// Extend a `DBig` to carry at least `precision` significant digits, so
/// subsequent arithmetic does not round to the input's narrow precision.
fn extend_precision(value: &DBig, precision: usize) -> DBig {
    value.clone().with_precision(precision).value()
}

/// Parse a literal and extend it to `precision` significant digits.
fn dbig_at_precision(s: &str, precision: usize) -> DBig {
    let v = DBig::from_str(s).expect("valid decimal literal");
    v.with_precision(precision).value()
}

/// Create a `DBig` from a `u32` at `precision` significant digits.
fn dbig_u32_at_precision(n: u32, precision: usize) -> DBig {
    let v = DBig::from_str(&format!("{n}.0")).expect("integer.0 is always a valid decimal");
    v.with_precision(precision).value()
}

fn validate_precision(precision: usize) -> OxiNumResult<()> {
    if precision == 0 {
        return Err(OxiNumError::Precision("precision must be > 0".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_of_zero() {
        let x = make_dbig("0.0").expect("ok");
        let result = sin(&x, 30).expect("ok");
        assert_eq!(result, make_dbig("0.0").expect("ok"));
    }

    #[test]
    fn cos_of_zero() {
        let result = cos(&make_dbig("0.0").expect("ok"), 30).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("1"), "cos(0) = {s}");
    }

    #[test]
    fn sin_of_pi_is_near_zero() {
        let pi = compute_pi(40);
        let result = sin(&pi, 20).expect("ok");
        let s = result.to_string();
        let s_trimmed = s.trim_start_matches('-');
        assert!(
            s_trimmed.starts_with("0.000") || s_trimmed == "0" || s_trimmed.starts_with("0.0"),
            "sin(pi) = {s}"
        );
    }

    #[test]
    fn cos_of_pi_is_near_neg_one() {
        let pi = compute_pi(40);
        let result = cos(&pi, 20).expect("ok");
        let s = result.to_string();
        assert!(
            s.starts_with("-1") || s.starts_with("-0.9999"),
            "cos(pi) = {s}"
        );
    }

    #[test]
    fn sinh_of_zero() {
        let x = make_dbig("0.0").expect("ok");
        let result = sinh(&x, 30).expect("ok");
        let s = result.to_string();
        let s_trimmed = s.trim_start_matches('-');
        assert!(s_trimmed.starts_with("0"), "sinh(0) = {s}");
    }

    #[test]
    fn cosh_of_zero() {
        let x = make_dbig("0.0").expect("ok");
        let result = cosh(&x, 30).expect("ok");
        let s = result.to_string();
        assert!(
            s.starts_with("1.000") || s.starts_with("1"),
            "cosh(0) = {s}"
        );
    }

    #[test]
    fn tanh_of_zero() {
        let x = make_dbig("0.0").expect("ok");
        let result = tanh(&x, 30).expect("ok");
        let s = result.to_string();
        let s_trimmed = s.trim_start_matches('-');
        assert!(s_trimmed.starts_with("0"), "tanh(0) = {s}");
    }

    #[test]
    fn atan_of_zero() {
        let x = make_dbig("0.0").expect("ok");
        let result = atan(&x, 30).expect("ok");
        assert_eq!(result, make_dbig("0.0").expect("ok"));
    }

    #[test]
    fn atan_of_one_is_pi_over_4() {
        let x = make_dbig("1.0").expect("ok");
        let result = atan(&x, 30).expect("ok");
        let s = result.to_string();
        assert!(
            s.starts_with("0.7853981"),
            "atan(1) = {s}, expected ~0.7853981..."
        );
    }

    #[test]
    fn atan2_positive_x() {
        let y = make_dbig("1.0").expect("ok");
        let x = make_dbig("1.0").expect("ok");
        let result = atan2(&y, &x, 20).expect("ok");
        let s = result.to_string();
        assert!(s.starts_with("0.785"), "atan2(1,1) = {s}");
    }

    #[test]
    fn precision_zero_errors_trig() {
        let x = make_dbig("1.0").expect("ok");
        assert!(sin(&x, 0).is_err());
        assert!(cos(&x, 0).is_err());
        assert!(tan(&x, 0).is_err());
        assert!(atan(&x, 0).is_err());
        assert!(sinh(&x, 0).is_err());
        assert!(cosh(&x, 0).is_err());
        assert!(tanh(&x, 0).is_err());
    }

    #[test]
    fn sin_cos_identity() {
        // sin^2(x) + cos^2(x) = 1 to high precision
        let x = make_dbig("0.7").expect("ok");
        let prec = 30;
        let s = sin(&x, prec).expect("ok");
        let c = cos(&x, prec).expect("ok");
        let sum = &(&s * &s) + &(&c * &c);
        let s_str = sum.to_string();
        // Should agree with 1.0 to at least ~25 digits
        assert!(
            s_str.starts_with("0.9999999999999999999999999")
                || s_str.starts_with("1.000000000000000000000000"),
            "sin^2(0.7) + cos^2(0.7) = {s_str}"
        );
    }

    #[test]
    fn sin_of_known_value() {
        // sin(0.7) = 0.6442176872376910536726143513...
        let x = make_dbig("0.7").expect("ok");
        let result = sin(&x, 25).expect("ok");
        let s = result.to_string();
        assert!(
            s.starts_with("0.644217687237691053672614"),
            "sin(0.7) = {s}"
        );
    }

    #[test]
    fn cos_of_known_value() {
        // cos(0.7) = 0.7648421872844884262558599...
        let x = make_dbig("0.7").expect("ok");
        let result = cos(&x, 25).expect("ok");
        let s = result.to_string();
        assert!(
            s.starts_with("0.764842187284488426255859"),
            "cos(0.7) = {s}"
        );
    }
}
