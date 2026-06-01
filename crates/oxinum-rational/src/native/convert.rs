//! Conversion between [`BigRational`] and [`BigFloat`].
//!
//! # Functions
//!
//! - [`rational_to_float`] — convert a `BigRational` to a `BigFloat` at the
//!   requested precision by computing `num / den` in floating-point arithmetic.
//! - [`float_to_rational`] — exact conversion: every finite `BigFloat` is
//!   mathematically equal to `mantissa * 2^exponent`, which is a rational
//!   number.  For `exponent >= 0` the denominator is 1; otherwise it is
//!   `2^(-exponent)`.
//!
//! # No cycles
//!
//! `oxinum-rational` → `oxinum-float` → `oxinum-int` — no cycle.

use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_float::native::{BigFloat, RoundingMode};
use oxinum_int::native::{BigInt, BigUint};

use super::BigRational;

/// Convert a `BigRational` to a `BigFloat` at `prec` significant bits.
///
/// The computation is `BigFloat(num) / BigFloat(den)` performed at
/// `prec + 16` guard bits and then rounded to `prec`.
///
/// # Errors
///
/// - [`OxiNumError::DivByZero`] if the denominator of `r` is zero (this
///   should not be reachable via the normal `BigRational` constructors, which
///   enforce `den > 0`, but is included for completeness).
///
/// # Examples
///
/// ```
/// use oxinum_rational::native::{BigRational, rational_to_float};
/// use oxinum_float::native::RoundingMode;
/// use oxinum_int::native::{BigInt, BigUint};
///
/// let r = BigRational::from_parts(BigInt::from(1i64), BigUint::from_u64(3))
///     .expect("1/3");
/// let f = rational_to_float(&r, 64, RoundingMode::HalfEven).expect("conv");
/// let expected = 1.0f64 / 3.0;
/// assert!((f.to_f64() - expected).abs() < 1e-14);
/// ```
pub fn rational_to_float(r: &BigRational, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    assert!(prec > 0, "BigFloat precision must be > 0");

    let guard = 16u32;
    let work_prec = prec.saturating_add(guard);

    let num_f = BigFloat::from_bigint(r.num(), work_prec, mode);
    let den_f = BigFloat::from_biguint(r.den(), work_prec, mode);

    if den_f.is_zero() {
        return Err(OxiNumError::DivByZero);
    }

    let result = num_f.div_ref_with_mode(&den_f, mode)?;
    Ok(result.with_precision(prec, mode))
}

/// Exact conversion from `BigFloat` to `BigRational`.
///
/// A `BigFloat` represents `(-1)^sign * mantissa * 2^exponent`, which is an
/// exact rational number:
///
/// - If `exponent >= 0`: numerator = `±mantissa * 2^exponent`, denominator = 1.
/// - If `exponent < 0`: numerator = `±mantissa`, denominator = `2^(-exponent)`.
///
/// The result is automatically reduced to lowest terms by [`BigRational::from_parts`].
///
/// # Examples
///
/// ```
/// use oxinum_rational::native::{BigRational, float_to_rational};
/// use oxinum_float::native::{BigFloat, RoundingMode};
/// use oxinum_int::native::{BigInt, BigUint};
///
/// // 1.5 = 3/2
/// let f = BigFloat::from_f64(1.5, 64).expect("1.5");
/// let r = float_to_rational(&f);
/// let expected = BigRational::from_parts(BigInt::from(3i64), BigUint::from_u64(2))
///     .expect("3/2");
/// assert_eq!(r, expected);
/// ```
pub fn float_to_rational(f: &BigFloat) -> BigRational {
    if f.is_zero() {
        return BigRational::zero();
    }

    let mantissa = f.mantissa().clone();
    let exponent = f.exponent();
    let sign = f.sign();

    if exponent >= 0 {
        // numerator = mantissa * 2^exponent (always a positive integer)
        let shift = exponent as u64;
        let num_u = mantissa.shl_bits(shift);
        let num = BigInt::from_parts(sign, num_u);
        BigRational::from_integer(num)
    } else {
        // exponent < 0 → denominator = 2^(-exponent)
        let neg_exp = (-exponent) as u64;
        let den = BigUint::one().shl_bits(neg_exp);
        let num = BigInt::from_parts(sign, mantissa);
        // `den > 0` is guaranteed (it's a power of two), so from_parts always
        // succeeds.  We propagate any unexpected error as a panic because it
        // signals a broken invariant.
        BigRational::from_parts(num, den)
            .unwrap_or_else(|e| panic!("float_to_rational: broken invariant: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oxinum_float::native::RoundingMode;
    use oxinum_int::native::{BigInt, BigUint};

    const MODE: RoundingMode = RoundingMode::HalfEven;

    fn br(n: i64, d: u64) -> BigRational {
        BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d)).expect("br")
    }

    #[test]
    fn rational_to_float_one_third() {
        let r = br(1, 3);
        let f = rational_to_float(&r, 64, MODE).expect("1/3 to float");
        let expected = 1.0f64 / 3.0;
        assert!(
            (f.to_f64() - expected).abs() < 1e-14,
            "1/3 as float: got {}, expected {}",
            f.to_f64(),
            expected
        );
    }

    #[test]
    fn rational_to_float_negative() {
        let r = br(-7, 4);
        let f = rational_to_float(&r, 64, MODE).expect("-7/4 to float");
        assert!(
            (f.to_f64() - (-1.75)).abs() < 1e-14,
            "-7/4 as float: {}",
            f.to_f64()
        );
    }

    #[test]
    fn rational_to_float_zero() {
        let r = BigRational::zero();
        let f = rational_to_float(&r, 64, MODE).expect("0 to float");
        assert!(f.is_zero());
    }

    #[test]
    fn float_to_rational_half() {
        let f = BigFloat::from_f64(0.5, 64).expect("0.5");
        let r = float_to_rational(&f);
        assert_eq!(r, br(1, 2), "0.5 → {r}");
    }

    #[test]
    fn float_to_rational_one_and_a_half() {
        let f = BigFloat::from_f64(1.5, 64).expect("1.5");
        let r = float_to_rational(&f);
        assert_eq!(r, br(3, 2), "1.5 → {r}");
    }

    #[test]
    fn float_to_rational_integer() {
        let f = BigFloat::from_i64(42, 64, MODE);
        let r = float_to_rational(&f);
        assert_eq!(r, br(42, 1), "42 → {r}");
    }

    #[test]
    fn float_to_rational_zero() {
        let f = BigFloat::zero(64);
        let r = float_to_rational(&f);
        assert!(r.is_zero());
    }

    #[test]
    fn float_to_rational_negative() {
        let f = BigFloat::from_f64(-0.25, 64).expect("-0.25");
        let r = float_to_rational(&f);
        assert_eq!(r, br(-1, 4), "-0.25 → {r}");
    }

    #[test]
    fn binary_rational_roundtrip() {
        // Binary rationals (denominator is a power of 2) round-trip exactly.
        for (n, d) in [(1i64, 2u64), (3, 4), (7, 8), (1, 16), (-3, 8)] {
            let r = br(n, d);
            let f = rational_to_float(&r, 128, MODE).expect("to_float");
            let back = float_to_rational(&f);
            assert_eq!(back, r, "Round-trip failed for {n}/{d}: got {back}");
        }
    }

    #[test]
    fn from_bigint_positive() {
        use oxinum_int::native::BigInt;
        let n = BigInt::from(12345i64);
        let f = BigFloat::from_bigint(&n, 64, MODE);
        assert!((f.to_f64() - 12345.0).abs() < 1e-10);
    }

    #[test]
    fn from_bigint_negative() {
        use oxinum_int::native::BigInt;
        let n = BigInt::from(-999i64);
        let f = BigFloat::from_bigint(&n, 64, MODE);
        assert!((f.to_f64() - (-999.0)).abs() < 1e-10);
    }

    #[test]
    fn from_biguint_large() {
        let n = BigUint::from_u64(u64::MAX);
        let f = BigFloat::from_biguint(&n, 64, MODE);
        let expected = u64::MAX as f64;
        // f64 can't represent u64::MAX exactly, but within 1 ULP is fine
        assert!((f.to_f64() / expected - 1.0).abs() < 1e-13);
    }
}
