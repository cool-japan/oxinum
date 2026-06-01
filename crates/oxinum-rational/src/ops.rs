//! Extended rational operations: continued fractions, best rational
//! approximation, decimal conversion, mediant, mixed numbers, and
//! rounding helpers.

use crate::{IBig, RBig, UBig};
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// Continued fractions
// ---------------------------------------------------------------------------

/// Compute the continued fraction expansion of a rational number.
///
/// Returns the coefficients `[a0; a1, a2, ...]` such that
///
/// ```text
/// x = a0 + 1/(a1 + 1/(a2 + ...))
/// ```
///
/// For a rational number this expansion is always finite.
///
/// # Examples
///
/// ```
/// use oxinum_rational::continued_fraction;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// // 355/113 = [3; 7, 16]
/// let r = RBig::from_parts(IBig::from(355), UBig::from(113u32));
/// let cf = continued_fraction(&r);
/// assert_eq!(cf, vec![IBig::from(3), IBig::from(7), IBig::from(16)]);
/// ```
pub fn continued_fraction(x: &RBig) -> Vec<IBig> {
    let mut coeffs = Vec::new();
    let mut num = x.numerator().clone();
    let mut den: IBig = x.denominator().clone().into();

    if den == IBig::ZERO {
        return coeffs;
    }

    loop {
        // Euclidean division: a = floor(num / den)
        let (q, r) = div_floor(&num, &den);
        coeffs.push(q);
        if r == IBig::ZERO {
            break;
        }
        num = den;
        den = r;
    }
    coeffs
}

/// Reconstruct a rational number from its continued fraction coefficients.
///
/// The inverse of [`continued_fraction`].
///
/// # Examples
///
/// ```
/// use oxinum_rational::from_continued_fraction;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// // [3; 7, 16] = 355/113
/// let cf = vec![IBig::from(3), IBig::from(7), IBig::from(16)];
/// let r = from_continued_fraction(&cf).unwrap();
/// assert_eq!(r, RBig::from_parts(IBig::from(355), UBig::from(113u32)));
/// ```
pub fn from_continued_fraction(coeffs: &[IBig]) -> OxiNumResult<RBig> {
    if coeffs.is_empty() {
        return Err(OxiNumError::Parse("empty continued fraction".into()));
    }
    // Build from the back: start with the last coefficient
    let mut result = RBig::from(coeffs[coeffs.len() - 1].clone());
    for coeff in coeffs[..coeffs.len() - 1].iter().rev() {
        // result = coeff + 1/result
        if result == RBig::ZERO {
            return Err(OxiNumError::DivByZero);
        }
        let reciprocal = rational_reciprocal(&result)?;
        result = RBig::from(coeff.clone()) + reciprocal;
    }
    Ok(result)
}

/// Find the best rational approximation to `x` with denominator at most `max_denom`.
///
/// Uses the continued fraction convergents to find the closest rational
/// whose denominator does not exceed `max_denom`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::best_rational_approximation;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// // Approximate 355/113 (pi) with denominator <= 100 -> 22/7
/// let pi = RBig::from_parts(IBig::from(355), UBig::from(113u32));
/// let approx = best_rational_approximation(&pi, &UBig::from(100u32));
/// assert_eq!(approx, RBig::from_parts(IBig::from(22), UBig::from(7u32)));
/// ```
pub fn best_rational_approximation(x: &RBig, max_denom: &UBig) -> RBig {
    if *max_denom == UBig::ZERO {
        return RBig::from(x.floor());
    }
    let coeffs = continued_fraction(x);
    if coeffs.is_empty() {
        return RBig::ZERO;
    }

    // Build convergents incrementally; stop when the denominator exceeds max_denom.
    // Convergent recurrence:
    //   h_{-1} = 1, h_{-2} = 0
    //   k_{-1} = 0, k_{-2} = 1
    //   h_n = a_n * h_{n-1} + h_{n-2}
    //   k_n = a_n * k_{n-1} + k_{n-2}
    let mut h_prev2 = IBig::ZERO; // h_{-2}
    let mut h_prev1 = IBig::ONE; // h_{-1}
    let mut k_prev2 = IBig::ONE; // k_{-2}
    let mut k_prev1 = IBig::ZERO; // k_{-1}

    let max_denom_i: IBig = max_denom.clone().into();
    let mut best = RBig::from(coeffs[0].clone());

    for coeff in &coeffs {
        let h_n = coeff * &h_prev1 + &h_prev2;
        let k_n = coeff * &k_prev1 + &k_prev2;

        if k_n > max_denom_i {
            // Denominator too big -- try semiconvergent, else stop.
            // The previous convergent is the best full convergent within bound.
            break;
        }

        // k_n is within bounds -- record this convergent
        best = rbig_from_signed(&h_n, &k_n);

        h_prev2 = h_prev1;
        h_prev1 = h_n;
        k_prev2 = k_prev1;
        k_prev1 = k_n;
    }

    best
}

// ---------------------------------------------------------------------------
// Decimal conversion
// ---------------------------------------------------------------------------

/// Convert a rational to a decimal string with `decimal_places` digits
/// after the point (truncated, not rounded).
///
/// # Examples
///
/// ```
/// use oxinum_rational::to_decimal_string;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// // 1/3 to 6 decimal places
/// let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
/// assert_eq!(to_decimal_string(&third, 6), "0.333333");
/// // 1/4 = 0.25
/// let quarter = RBig::from_parts(IBig::from(1), UBig::from(4u32));
/// assert_eq!(to_decimal_string(&quarter, 4), "0.2500");
/// ```
pub fn to_decimal_string(x: &RBig, decimal_places: usize) -> String {
    let num = x.numerator().clone();
    let den: IBig = x.denominator().clone().into();

    let is_negative = num < IBig::ZERO;
    let abs_num = if is_negative { -num } else { num };

    // Integer part
    let (int_part, mut remainder) = div_floor(&abs_num, &den);

    let mut result = String::new();
    if is_negative && (int_part != IBig::ZERO || decimal_places > 0) {
        result.push('-');
    }
    result.push_str(&int_part.to_string());

    if decimal_places > 0 {
        result.push('.');
        let ten = IBig::from(10);
        for _ in 0..decimal_places {
            remainder *= &ten;
            let (digit, new_rem) = div_floor(&remainder, &den);
            result.push_str(&digit.to_string());
            remainder = new_rem;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Mediant
// ---------------------------------------------------------------------------

/// Compute the mediant of two rationals: `(a_num + b_num) / (a_den + b_den)`.
///
/// The mediant of `a/b` and `c/d` is `(a+c)/(b+d)`, which always lies
/// strictly between the two fractions (when they differ).
///
/// # Examples
///
/// ```
/// use oxinum_rational::mediant;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// // mediant(1/2, 1/3) = 2/5
/// let a = RBig::from_parts(IBig::from(1), UBig::from(2u32));
/// let b = RBig::from_parts(IBig::from(1), UBig::from(3u32));
/// assert_eq!(mediant(&a, &b), RBig::from_parts(IBig::from(2), UBig::from(5u32)));
/// ```
pub fn mediant(a: &RBig, b: &RBig) -> RBig {
    let a_num = a.numerator().clone();
    let a_den: IBig = a.denominator().clone().into();
    let b_num = b.numerator().clone();
    let b_den: IBig = b.denominator().clone().into();

    let num = a_num + b_num;
    let den = a_den + b_den;
    rbig_from_signed(&num, &den)
}

// ---------------------------------------------------------------------------
// Mixed numbers
// ---------------------------------------------------------------------------

/// Decompose a rational into a mixed number: `(whole, fractional)` where
/// `whole` is the integer part (toward zero) and `fractional` is the
/// remaining proper fraction with the same sign.
///
/// # Examples
///
/// ```
/// use oxinum_rational::mixed_number;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// // 7/3 = 2 + 1/3
/// let r = RBig::from_parts(IBig::from(7), UBig::from(3u32));
/// let (whole, frac) = mixed_number(&r);
/// assert_eq!(whole, IBig::from(2));
/// assert_eq!(frac, RBig::from_parts(IBig::from(1), UBig::from(3u32)));
/// ```
pub fn mixed_number(x: &RBig) -> (IBig, RBig) {
    let whole = x.trunc();
    let frac = x.fract();
    (whole, frac)
}

// ---------------------------------------------------------------------------
// Rounding helpers (wrapping dashu's methods for ergonomics)
// ---------------------------------------------------------------------------

/// Returns the largest integer not greater than `x` (toward negative infinity).
pub fn rational_floor(x: &RBig) -> IBig {
    x.floor()
}

/// Returns the smallest integer not less than `x` (toward positive infinity).
pub fn rational_ceil(x: &RBig) -> IBig {
    x.ceil()
}

/// Returns the nearest integer to `x` (ties away from zero).
pub fn rational_round(x: &RBig) -> IBig {
    x.round()
}

/// Returns the integer part of `x` (truncated toward zero).
pub fn rational_truncate(x: &RBig) -> IBig {
    x.trunc()
}

// ---------------------------------------------------------------------------
// Integer interop (convenience free fns)
// ---------------------------------------------------------------------------

/// Construct an `RBig` from an integer value.
///
/// Equivalent to `n / 1` — the result satisfies
/// [`rational_is_integer`]`(&rational_from_integer(n)) == true`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{rational_from_integer, rational_is_integer};
/// use dashu_int::IBig;
/// let r = rational_from_integer(&IBig::from(42));
/// assert!(rational_is_integer(&r));
/// ```
pub fn rational_from_integer(n: &IBig) -> RBig {
    RBig::from_parts(n.clone(), UBig::ONE)
}

/// Returns `true` iff `x` represents an integer (i.e. its reduced
/// denominator is one).
///
/// Because `RBig` keeps its values in lowest terms, this is a constant-time
/// comparison against `UBig::ONE`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::rational_is_integer;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// assert!(rational_is_integer(&RBig::from_parts(IBig::from(3), UBig::ONE)));
/// assert!(rational_is_integer(&RBig::from_parts(IBig::from(10), UBig::from(5u32)))); // reduces to 2/1
/// assert!(!rational_is_integer(&RBig::from_parts(IBig::from(3), UBig::from(2u32))));
/// ```
pub fn rational_is_integer(x: &RBig) -> bool {
    *x.denominator() == UBig::ONE
}

/// Extract the integer value of `x` if it is one.
///
/// Returns `Some(numerator)` when [`rational_is_integer`] is `true`, and
/// `None` otherwise.  Combined with [`rational_from_integer`] this gives a
/// round-trip between `IBig` and an integer-valued `RBig`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{rational_from_integer, rational_to_integer};
/// use dashu_int::IBig;
/// let r = rational_from_integer(&IBig::from(42));
/// assert_eq!(rational_to_integer(&r), Some(IBig::from(42)));
/// ```
pub fn rational_to_integer(x: &RBig) -> Option<IBig> {
    if rational_is_integer(x) {
        Some(x.numerator().clone())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Sign / abs / reciprocal / pow
// ---------------------------------------------------------------------------

/// Returns the absolute value of `x`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::rational_abs;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// let r = RBig::from_parts(IBig::from(-3), UBig::from(4u32));
/// assert_eq!(rational_abs(&r), RBig::from_parts(IBig::from(3), UBig::from(4u32)));
/// ```
pub fn rational_abs(x: &RBig) -> RBig {
    if x.numerator() < &IBig::ZERO {
        -x.clone()
    } else {
        x.clone()
    }
}

/// Returns the sign of `x`: -1, 0, or +1 as an `IBig`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::rational_signum;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// let r = RBig::from_parts(IBig::from(-3), UBig::from(4u32));
/// assert_eq!(rational_signum(&r), IBig::from(-1));
/// ```
pub fn rational_signum(x: &RBig) -> IBig {
    if x.is_zero() {
        IBig::ZERO
    } else if x.numerator() < &IBig::ZERO {
        IBig::from(-1)
    } else {
        IBig::ONE
    }
}

/// Returns the reciprocal `1/x`.
///
/// # Errors
///
/// Returns `OxiNumError::DivByZero` if `x` is zero.
///
/// # Examples
///
/// ```
/// use oxinum_rational::rational_reciprocal;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// let r = RBig::from_parts(IBig::from(3), UBig::from(4u32));
/// let recip = rational_reciprocal(&r).unwrap();
/// assert_eq!(recip, RBig::from_parts(IBig::from(4), UBig::from(3u32)));
/// ```
pub fn rational_reciprocal(x: &RBig) -> OxiNumResult<RBig> {
    if x.is_zero() {
        return Err(OxiNumError::DivByZero);
    }
    // 1/x: swap numerator and denominator. x is stored as (signed num)/(unsigned den),
    // so the reciprocal is (signed den-with-x's-sign)/(|num|).
    let num = x.numerator().clone();
    let den: IBig = x.denominator().clone().into();
    let sign_negative = num < IBig::ZERO;
    // New numerator carries the sign; new denominator is |num| (always positive).
    let abs_num = if sign_negative { -&num } else { num };
    let new_num = if sign_negative { -den } else { den };
    Ok(rbig_from_signed(&new_num, &abs_num))
}

/// Raise a rational to an integer power.
///
/// Negative exponents produce the reciprocal raised to the absolute value.
///
/// # Errors
///
/// Returns `OxiNumError::DivByZero` if `x` is zero and `n` is negative.
///
/// # Examples
///
/// ```
/// use oxinum_rational::rational_pow;
/// use dashu_ratio::RBig;
/// use dashu_int::{IBig, UBig};
/// // (2/3)^2 = 4/9
/// let r = RBig::from_parts(IBig::from(2), UBig::from(3u32));
/// let result = rational_pow(&r, 2).unwrap();
/// assert_eq!(result, RBig::from_parts(IBig::from(4), UBig::from(9u32)));
/// // (2/3)^-1 = 3/2
/// let inv = rational_pow(&r, -1).unwrap();
/// assert_eq!(inv, RBig::from_parts(IBig::from(3), UBig::from(2u32)));
/// ```
pub fn rational_pow(x: &RBig, n: i32) -> OxiNumResult<RBig> {
    if n == 0 {
        return Ok(RBig::ONE);
    }
    if n > 0 {
        Ok(x.pow(n as usize))
    } else {
        let reciprocal = rational_reciprocal(x)?;
        let abs_n = n.unsigned_abs() as usize;
        Ok(reciprocal.pow(abs_n))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Floor division returning `(quotient, remainder)` where the remainder
/// has the same sign as the divisor and `0 <= remainder < |divisor|`.
pub(crate) fn div_floor(num: &IBig, den: &IBig) -> (IBig, IBig) {
    use dashu_base::DivRem;
    let (mut q, mut r) = num.clone().div_rem(den.clone());
    // Adjust so remainder is non-negative when divisor is positive
    if (r < IBig::ZERO && *den > IBig::ZERO) || (r > IBig::ZERO && *den < IBig::ZERO) {
        q -= IBig::ONE;
        r += den;
    }
    (q, r)
}

/// Construct an `RBig` from a signed numerator and a signed denominator.
pub(crate) fn rbig_from_signed(num: &IBig, den: &IBig) -> RBig {
    RBig::from_parts_signed(num.clone(), den.clone())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_355_over_113() {
        // 355/113 = [3; 7, 16]
        let r = RBig::from_parts(IBig::from(355), UBig::from(113u32));
        let cf = continued_fraction(&r);
        assert_eq!(cf, vec![IBig::from(3), IBig::from(7), IBig::from(16)]);
    }

    #[test]
    fn cf_integer() {
        // 5/1 = [5]
        let r = RBig::from(5u32);
        let cf = continued_fraction(&r);
        assert_eq!(cf, vec![IBig::from(5)]);
    }

    #[test]
    fn cf_half() {
        // 1/2 = [0; 2]
        let r = RBig::from_parts(IBig::from(1), UBig::from(2u32));
        let cf = continued_fraction(&r);
        assert_eq!(cf, vec![IBig::ZERO, IBig::from(2)]);
    }

    #[test]
    fn cf_roundtrip() {
        let r = RBig::from_parts(IBig::from(355), UBig::from(113u32));
        let cf = continued_fraction(&r);
        let reconstructed = from_continued_fraction(&cf).expect("ok");
        assert_eq!(reconstructed, r);
    }

    #[test]
    fn cf_roundtrip_various() {
        let cases = [(22, 7u32), (1, 7u32), (100, 3u32), (17, 12u32), (1, 1u32)];
        for (n, d) in cases {
            let r = RBig::from_parts(IBig::from(n), UBig::from(d));
            let cf = continued_fraction(&r);
            let back = from_continued_fraction(&cf).expect("ok");
            assert_eq!(back, r, "roundtrip failed for {n}/{d}");
        }
    }

    #[test]
    fn from_cf_pi_approx() {
        // [3; 7, 16] = 355/113
        let cf = vec![IBig::from(3), IBig::from(7), IBig::from(16)];
        let r = from_continued_fraction(&cf).expect("ok");
        assert_eq!(r, RBig::from_parts(IBig::from(355), UBig::from(113u32)));
    }

    #[test]
    fn from_cf_empty_errors() {
        assert!(from_continued_fraction(&[]).is_err());
    }

    #[test]
    fn best_approx_pi() {
        // Best approx of 355/113 with denom <= 100 is 22/7
        let pi = RBig::from_parts(IBig::from(355), UBig::from(113u32));
        let approx = best_rational_approximation(&pi, &UBig::from(100u32));
        assert_eq!(approx, RBig::from_parts(IBig::from(22), UBig::from(7u32)));
    }

    #[test]
    fn best_approx_small_denom() {
        // Best approx of 355/113 with denom <= 10 is 22/7 (denom 7 <= 10)
        let pi = RBig::from_parts(IBig::from(355), UBig::from(113u32));
        let approx = best_rational_approximation(&pi, &UBig::from(10u32));
        assert_eq!(approx, RBig::from_parts(IBig::from(22), UBig::from(7u32)));
    }

    #[test]
    fn best_approx_denom_one() {
        // Best approx of 7/2 = 3.5 with denom <= 1 is 3
        let r = RBig::from_parts(IBig::from(7), UBig::from(2u32));
        let approx = best_rational_approximation(&r, &UBig::ONE);
        assert_eq!(approx, RBig::from(3u32));
    }

    #[test]
    fn decimal_third() {
        let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        assert_eq!(to_decimal_string(&third, 6), "0.333333");
    }

    #[test]
    fn decimal_quarter() {
        let quarter = RBig::from_parts(IBig::from(1), UBig::from(4u32));
        assert_eq!(to_decimal_string(&quarter, 4), "0.2500");
    }

    #[test]
    fn decimal_negative() {
        let neg = RBig::from_parts(IBig::from(-1), UBig::from(2u32));
        assert_eq!(to_decimal_string(&neg, 2), "-0.50");
    }

    #[test]
    fn decimal_improper() {
        // 7/4 = 1.75
        let r = RBig::from_parts(IBig::from(7), UBig::from(4u32));
        assert_eq!(to_decimal_string(&r, 2), "1.75");
    }

    #[test]
    fn decimal_zero_places() {
        let r = RBig::from_parts(IBig::from(7), UBig::from(4u32));
        assert_eq!(to_decimal_string(&r, 0), "1");
    }

    #[test]
    fn mediant_basic() {
        // mediant(1/2, 1/3) = 2/5
        let a = RBig::from_parts(IBig::from(1), UBig::from(2u32));
        let b = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        let m = mediant(&a, &b);
        assert_eq!(m, RBig::from_parts(IBig::from(2), UBig::from(5u32)));
    }

    #[test]
    fn mediant_between() {
        // mediant(0/1, 1/1) = 1/2
        let a = RBig::ZERO;
        let b = RBig::ONE;
        let m = mediant(&a, &b);
        assert_eq!(m, RBig::from_parts(IBig::from(1), UBig::from(2u32)));
    }

    #[test]
    fn mixed_number_basic() {
        // 7/3 = 2 + 1/3
        let r = RBig::from_parts(IBig::from(7), UBig::from(3u32));
        let (whole, frac) = mixed_number(&r);
        assert_eq!(whole, IBig::from(2));
        assert_eq!(frac, RBig::from_parts(IBig::from(1), UBig::from(3u32)));
    }

    #[test]
    fn mixed_number_negative() {
        // -7/3 = -2 - 1/3
        let r = RBig::from_parts(IBig::from(-7), UBig::from(3u32));
        let (whole, frac) = mixed_number(&r);
        assert_eq!(whole, IBig::from(-2));
        assert_eq!(frac, RBig::from_parts(IBig::from(-1), UBig::from(3u32)));
    }

    #[test]
    fn floor_ceil_round_trunc() {
        // 7/3 ≈ 2.333
        let r = RBig::from_parts(IBig::from(7), UBig::from(3u32));
        assert_eq!(rational_floor(&r), IBig::from(2));
        assert_eq!(rational_ceil(&r), IBig::from(3));
        assert_eq!(rational_round(&r), IBig::from(2));
        assert_eq!(rational_truncate(&r), IBig::from(2));
    }

    #[test]
    fn floor_ceil_negative() {
        // -7/3 ≈ -2.333
        let r = RBig::from_parts(IBig::from(-7), UBig::from(3u32));
        assert_eq!(rational_floor(&r), IBig::from(-3));
        assert_eq!(rational_ceil(&r), IBig::from(-2));
        assert_eq!(rational_truncate(&r), IBig::from(-2));
    }

    #[test]
    fn abs_basic() {
        let r = RBig::from_parts(IBig::from(-3), UBig::from(4u32));
        assert_eq!(
            rational_abs(&r),
            RBig::from_parts(IBig::from(3), UBig::from(4u32))
        );
    }

    #[test]
    fn signum_basic() {
        let pos = RBig::from_parts(IBig::from(3), UBig::from(4u32));
        let neg = RBig::from_parts(IBig::from(-3), UBig::from(4u32));
        assert_eq!(rational_signum(&pos), IBig::ONE);
        assert_eq!(rational_signum(&neg), IBig::from(-1));
        assert_eq!(rational_signum(&RBig::ZERO), IBig::ZERO);
    }

    #[test]
    fn reciprocal_basic() {
        let r = RBig::from_parts(IBig::from(3), UBig::from(4u32));
        let recip = rational_reciprocal(&r).expect("ok");
        assert_eq!(recip, RBig::from_parts(IBig::from(4), UBig::from(3u32)));
    }

    #[test]
    fn reciprocal_negative() {
        let r = RBig::from_parts(IBig::from(-3), UBig::from(4u32));
        let recip = rational_reciprocal(&r).expect("ok");
        assert_eq!(recip, RBig::from_parts(IBig::from(-4), UBig::from(3u32)));
    }

    #[test]
    fn reciprocal_zero_errors() {
        assert!(rational_reciprocal(&RBig::ZERO).is_err());
    }

    #[test]
    fn reciprocal_roundtrip() {
        // (a/b) * (b/a) = 1
        let r = RBig::from_parts(IBig::from(7), UBig::from(13u32));
        let recip = rational_reciprocal(&r).expect("ok");
        assert_eq!(r * recip, RBig::ONE);
    }

    #[test]
    fn pow_positive() {
        let r = RBig::from_parts(IBig::from(2), UBig::from(3u32));
        let result = rational_pow(&r, 2).expect("ok");
        assert_eq!(result, RBig::from_parts(IBig::from(4), UBig::from(9u32)));
    }

    #[test]
    fn pow_negative() {
        let r = RBig::from_parts(IBig::from(2), UBig::from(3u32));
        let result = rational_pow(&r, -1).expect("ok");
        assert_eq!(result, RBig::from_parts(IBig::from(3), UBig::from(2u32)));
    }

    #[test]
    fn pow_zero() {
        let r = RBig::from_parts(IBig::from(5), UBig::from(7u32));
        assert_eq!(rational_pow(&r, 0).expect("ok"), RBig::ONE);
    }

    #[test]
    fn pow_negative_squared() {
        // (2/3)^-2 = 9/4
        let r = RBig::from_parts(IBig::from(2), UBig::from(3u32));
        let result = rational_pow(&r, -2).expect("ok");
        assert_eq!(result, RBig::from_parts(IBig::from(9), UBig::from(4u32)));
    }

    #[test]
    fn div_floor_positive() {
        let (q, r) = div_floor(&IBig::from(7), &IBig::from(3));
        assert_eq!(q, IBig::from(2));
        assert_eq!(r, IBig::from(1));
    }

    #[test]
    fn div_floor_negative_dividend() {
        // -7 / 3 = -3 remainder 2 (floor division)
        let (q, r) = div_floor(&IBig::from(-7), &IBig::from(3));
        assert_eq!(q, IBig::from(-3));
        assert_eq!(r, IBig::from(2));
    }

    // ----- Integer interop convenience fns ---------------------------------

    #[test]
    fn from_integer_basic() {
        let r = rational_from_integer(&IBig::from(42));
        assert_eq!(r.numerator(), &IBig::from(42));
        assert_eq!(r.denominator(), &UBig::ONE);
    }

    #[test]
    fn from_integer_negative() {
        let r = rational_from_integer(&IBig::from(-7));
        assert_eq!(r.numerator(), &IBig::from(-7));
        assert_eq!(r.denominator(), &UBig::ONE);
    }

    #[test]
    fn from_integer_zero() {
        let r = rational_from_integer(&IBig::ZERO);
        assert_eq!(r, RBig::ZERO);
    }

    #[test]
    fn is_integer_true_for_whole() {
        let r = rbig_from_signed(&IBig::from(3), &IBig::from(1));
        assert!(rational_is_integer(&r));
    }

    #[test]
    fn is_integer_false_for_fraction() {
        let r = rbig_from_signed(&IBig::from(3), &IBig::from(2));
        assert!(!rational_is_integer(&r));
    }

    #[test]
    fn is_integer_after_simplification() {
        // 10/5 reduces to 2/1
        let r = RBig::from_parts(IBig::from(10), UBig::from(5u32));
        assert!(rational_is_integer(&r));
    }

    #[test]
    fn to_integer_round_trip() {
        let n = IBig::from(42);
        let r = rational_from_integer(&n);
        assert_eq!(rational_to_integer(&r), Some(n));
    }

    #[test]
    fn to_integer_round_trip_negative() {
        let n = IBig::from(-12345);
        let r = rational_from_integer(&n);
        assert_eq!(rational_to_integer(&r), Some(n));
    }

    #[test]
    fn to_integer_none_for_fraction() {
        let r = RBig::from_parts(IBig::from(3), UBig::from(2u32));
        assert_eq!(rational_to_integer(&r), None);
    }
}
