//! Exponentiation and logarithm for [`BigFloat`].
//!
//! # Functions provided
//!
//! - [`BigFloat::pow`] — `self^exp` at a specified precision. Uses repeated
//!   squaring for exact integer exponents and `exp(exp * ln(self))` otherwise.
//! - [`BigFloat::log`] — `log_base(self)` = `ln(self) / ln(base)`.
//!
//! # Design notes
//!
//! **Integer fast-path:** A `BigFloat` is structurally an integer when its
//! exponent is ≥ 0 (because the mantissa is normalized: no trailing zeros,
//! `bit_length == precision`; so every bit is above the binary point).  We
//! detect this directly without going through `f64`, avoiding precision loss
//! for large exponents.
//!
//! **Guard bits:** All intermediate computations are performed at
//! `prec + 16` guard bits and rounded back to `prec` at the end.

use oxinum_core::{OxiNumError, OxiNumResult, Sign};

use super::float::{BigFloat, RoundingMode};

// ---------------------------------------------------------------------------
// pow
// ---------------------------------------------------------------------------

impl BigFloat {
    /// Return `self^exp` at `prec` bits using the chosen rounding mode.
    ///
    /// # Special cases
    ///
    /// - `x^0 = 1` for any `x` (including `x = 0`).
    /// - `0^pos = 0`.
    /// - `0^neg` → [`OxiNumError::Domain`].
    /// - Fractional exponent with non-positive base → [`OxiNumError::Domain`].
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] — domain violation (negative base with
    ///   fractional exponent, or zero base with negative exponent).
    /// - [`OxiNumError::Overflow`] — propagated from [`BigFloat::exp`] if the
    ///   intermediate `exp * ln(self)` exceeds the representable range.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    ///
    /// let two   = BigFloat::from_i64(2,  100, RoundingMode::HalfEven);
    /// let ten   = BigFloat::from_i64(10, 100, RoundingMode::HalfEven);
    /// let result = two.pow(&ten, 100, RoundingMode::HalfEven).expect("2^10");
    /// assert!((result.to_f64() - 1024.0).abs() < 1e-10);
    /// ```
    pub fn pow(&self, exp: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        assert!(prec > 0, "BigFloat precision must be > 0");

        // x^0 = 1 for ALL x (including NaN^0 = 1, Inf^0 = 1) — IEEE pow convention.
        if exp.is_zero() {
            return Ok(BigFloat::from_i64(1, prec, mode));
        }

        // --- IEEE-754 non-finite guards (after x^0 check) ---
        // NaN base or NaN exponent → NaN
        if self.is_nan() || exp.is_nan() {
            return Ok(BigFloat::nan(prec));
        }
        // Inf base cases
        if self.is_infinite() {
            if exp.signum() > 0 {
                // +Inf^pos = +Inf; -Inf^pos = +Inf (conservative; Wave 3 may refine for odd int)
                return Ok(BigFloat::infinity(prec));
            } else {
                // +Inf^neg = +0; -Inf^neg = +0
                return Ok(BigFloat::zero(prec));
            }
        }
        // Inf exponent cases (self is finite here)
        if exp.is_infinite() {
            // 1^±Inf = 1 (IEEE)
            let one = BigFloat::from_i64(1, prec, mode);
            if self == &one {
                return Ok(one);
            }
            // |base| > 1: base^(+Inf)=+Inf, base^(-Inf)=+0
            // |base| < 1: base^(+Inf)=+0, base^(-Inf)=+Inf
            // For non-positive base with fractional/Inf exponent, return NaN (conservative).
            if self.signum() <= 0 {
                return Ok(BigFloat::nan(prec));
            }
            let abs_f64 = self.abs().to_f64();
            let is_gt_one = abs_f64 > 1.0;
            let is_pos_inf = exp.sign == Sign::Positive;
            return if is_gt_one == is_pos_inf {
                Ok(BigFloat::infinity(prec))
            } else {
                Ok(BigFloat::zero(prec))
            };
        }

        // 0^pos = 0, 0^neg = undefined
        if self.is_zero() {
            if exp.signum() > 0 {
                return Ok(BigFloat::zero(prec));
            } else {
                return Err(OxiNumError::Domain(
                    "0^negative_exponent is undefined".into(),
                ));
            }
        }

        // Detect structural integer exponent: exponent >= 0 means the
        // BigFloat mantissa (normalized, no trailing zeros) sits fully above
        // the binary point.  Negative sign is handled via pow_int(negative n).
        if exp.exponent >= 0 {
            // The exponent is an exact integer.  We may need it as i64.
            // If it is too large to fit, fall through to the general path.
            let exp_i64_opt = exact_integer_to_i64(exp);
            if let Some(n) = exp_i64_opt {
                return self.pow_int(n, prec, mode);
            }
            // Exponent is a huge integer — fall through to exp(n*ln(x)).
            // (This would produce an astronomically large number, which exp()
            // will report as Overflow.)
        }

        // General (non-integer) case: self must be strictly positive.
        if self.signum() <= 0 {
            return Err(OxiNumError::Domain(
                "pow with fractional exponent requires a strictly positive base".into(),
            ));
        }

        let guard = 16u32;
        let work_prec = prec.saturating_add(guard);

        let ln_self = self.ln(work_prec, mode)?;
        let exp_wp = exp.clone().with_precision(work_prec, mode);
        let product = ln_self
            .mul_ref_with_mode(&exp_wp, mode)
            .with_precision(work_prec, mode);
        let result = product.exp(work_prec, mode)?;

        Ok(result.with_precision(prec, mode))
    }

    /// Integer power: `self^n` via binary exponentiation (repeated squaring).
    fn pow_int(&self, n: i64, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        if n == 0 {
            return Ok(BigFloat::from_i64(1, prec, mode));
        }

        if n < 0 {
            // self^(-|n|) = 1 / self^(|n|)
            // Use unsigned_abs to avoid i64::MIN overflow.
            let mag = (n as i128).unsigned_abs() as u64;
            let positive_pow = self.pow_uint(mag, prec, mode);
            let one = BigFloat::from_i64(1, prec, mode);
            return one.div_ref_with_mode(&positive_pow, mode);
        }

        Ok(self.pow_uint(n as u64, prec, mode))
    }

    /// Binary exponentiation: `self^exp_u` (non-negative exponent).
    fn pow_uint(&self, mut exp_u: u64, prec: u32, mode: RoundingMode) -> BigFloat {
        let mut result = BigFloat::from_i64(1, prec, mode);
        let mut base = self.clone().with_precision(prec, mode);

        while exp_u > 0 {
            if exp_u & 1 == 1 {
                result = result
                    .mul_ref_with_mode(&base, mode)
                    .with_precision(prec, mode);
            }
            base = base
                .mul_ref_with_mode(&base.clone(), mode)
                .with_precision(prec, mode);
            exp_u >>= 1;
        }

        result
    }
}

// ---------------------------------------------------------------------------
// log
// ---------------------------------------------------------------------------

impl BigFloat {
    /// Return `log_base(self)` (i.e. the logarithm of `self` in the given
    /// `base`) at `prec` bits using the chosen rounding mode.
    ///
    /// Computed as `ln(self) / ln(base)`.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] — `self <= 0`, `base <= 0`, or `base == 1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    ///
    /// let hundred = BigFloat::from_i64(100, 100, RoundingMode::HalfEven);
    /// let ten     = BigFloat::from_i64(10,  100, RoundingMode::HalfEven);
    /// let result  = hundred.log(&ten, 100, RoundingMode::HalfEven).expect("log_10(100)");
    /// assert!((result.to_f64() - 2.0).abs() < 1e-14);
    /// ```
    pub fn log(&self, base: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        assert!(prec > 0, "BigFloat precision must be > 0");

        if self.is_zero() || self.signum() < 0 {
            return Err(OxiNumError::Domain(
                "log of non-positive number is undefined".into(),
            ));
        }
        if base.is_zero() || base.signum() < 0 {
            return Err(OxiNumError::Domain(
                "log with non-positive base is undefined".into(),
            ));
        }

        // base == 1: log is undefined.
        let one = BigFloat::from_i64(1, prec, mode);
        if base == &one {
            return Err(OxiNumError::Domain("log base 1 is undefined".into()));
        }

        let guard = 16u32;
        let work_prec = prec.saturating_add(guard);

        let ln_self = self.ln(work_prec, mode)?;
        let ln_base = base.ln(work_prec, mode)?;

        // Divide using div_ref_with_mode (Div operator panics on zero — this
        // is unreachable because ln(1) = 0 case was caught above, but we use
        // the Result form regardless to be safe).
        let result = ln_self.div_ref_with_mode(&ln_base, mode)?;

        Ok(result.with_precision(prec, mode))
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// If `f` represents an exact integer that fits in `i64`, return `Some(n)`.
/// Returns `None` if the value is too large or would lose precision.
///
/// A normalized `BigFloat` with `exponent >= 0` represents `mantissa * 2^exp`
/// which is always an integer.  We need to fit it in i64.
fn exact_integer_to_i64(f: &BigFloat) -> Option<i64> {
    // f.exponent >= 0, so value = mantissa * 2^exponent — it is an integer.
    // The mantissa's bit_length == f.precision.
    // Total bits in the value = bit_length + exponent (as integer shift).
    // Must fit in 63 bits (signed) plus sign.
    let bit_len = f.mantissa().bit_length();
    let total_bits = bit_len.saturating_add(f.exponent() as u64);
    if total_bits > 63 {
        return None;
    }
    // safe: mantissa fits in u64 at <= 63 significant bits
    let mantissa_u64 = f.mantissa().to_u64()?;
    // shift: 0 <= f.exponent() (guaranteed by the caller)
    let exp = f.exponent() as u64;
    let value = mantissa_u64.checked_shl(exp as u32)?;
    // Apply sign
    if f.sign() == oxinum_core::Sign::Negative {
        // i64::MIN is -2^63 (= 9223372036854775808).
        // value <= 2^63-1 ensures -(value as i64) doesn't overflow.
        if value > i64::MAX as u64 {
            return None;
        }
        Some(-(value as i64))
    } else {
        if value > i64::MAX as u64 {
            return None;
        }
        Some(value as i64)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::RoundingMode;

    const MODE: RoundingMode = RoundingMode::HalfEven;

    fn mk(n: i64, prec: u32) -> BigFloat {
        BigFloat::from_i64(n, prec, MODE)
    }

    fn flt(x: f64, prec: u32) -> BigFloat {
        BigFloat::from_f64(x, prec).expect("from_f64")
    }

    // --- pow tests ---

    #[test]
    fn pow_two_to_ten() {
        let two = mk(2, 100);
        let ten = mk(10, 100);
        let result = two.pow(&ten, 100, MODE).expect("2^10");
        assert!(
            (result.to_f64() - 1024.0).abs() < 1e-10,
            "2^10 = {}",
            result.to_f64()
        );
    }

    #[test]
    fn pow_zero_exponent_is_one() {
        let zero_exp = mk(0, 64);
        for x_f in [2.0f64, 0.5, 10.0, 0.001, -3.0] {
            let x = flt(x_f, 64);
            let result = x.pow(&zero_exp, 64, MODE).expect("x^0");
            assert!(
                (result.to_f64() - 1.0).abs() < 1e-14,
                "{}^0 should be 1, got {}",
                x_f,
                result.to_f64()
            );
        }
    }

    #[test]
    fn pow_zero_base_positive_exp() {
        let z = mk(0, 64);
        let pos = mk(3, 64);
        let result = z.pow(&pos, 64, MODE).expect("0^3");
        assert!(result.is_zero(), "0^3 should be 0");
    }

    #[test]
    fn pow_zero_base_negative_exp_is_error() {
        let z = mk(0, 64);
        let neg = mk(-1, 64);
        assert!(z.pow(&neg, 64, MODE).is_err());
    }

    #[test]
    fn pow_negative_base_fractional_exp_is_error() {
        let neg = mk(-2, 64);
        let half = flt(1.5, 64);
        assert!(neg.pow(&half, 64, MODE).is_err());
    }

    #[test]
    fn pow_inverse_property() {
        // x^y * x^(-y) ≈ 1
        let prec = 100u32;
        for x_f in [2.0f64, 3.0, 7.5, 0.5] {
            let x = flt(x_f, prec);
            let y = flt(1.5, prec);
            let neg_y = y.neg();
            let xy = x.pow(&y, prec, MODE).expect("x^y");
            let xny = x.pow(&neg_y, prec, MODE).expect("x^(-y)");
            let product = xy.mul_ref_with_mode(&xny, MODE).with_precision(prec, MODE);
            assert!(
                (product.to_f64() - 1.0).abs() < 1e-13,
                "x^y * x^(-y) ≠ 1 for x={}, got {}",
                x_f,
                product.to_f64()
            );
        }
    }

    #[test]
    fn pow_negative_integer_exponent() {
        // 2^(-1) = 0.5
        let two = mk(2, 100);
        let neg_one = mk(-1, 100);
        let result = two.pow(&neg_one, 100, MODE).expect("2^(-1)");
        assert!(
            (result.to_f64() - 0.5).abs() < 1e-14,
            "2^-1 = {}",
            result.to_f64()
        );
    }

    #[test]
    fn pow_large_integer_exponent_cross_val() {
        // 3^20 = 3486784401
        let three = mk(3, 100);
        let twenty = mk(20, 100);
        let result = three.pow(&twenty, 100, MODE).expect("3^20");
        assert!(
            (result.to_f64() - 3_486_784_401.0_f64).abs() < 1.0,
            "3^20 ≈ {}, expected 3486784401",
            result.to_f64()
        );
    }

    #[test]
    fn pow_fractional_exp_sqrt() {
        // x^0.5 ≈ sqrt(x)
        let prec = 100u32;
        for x_f in [4.0f64, 9.0, 2.0, 0.25] {
            let x = flt(x_f, prec);
            let half = flt(0.5, prec);
            let result = x.pow(&half, prec, MODE).expect("x^0.5");
            let expected = x_f.sqrt();
            assert!(
                (result.to_f64() - expected).abs() < 1e-13,
                "{}^0.5 ≈ {}, expected {}",
                x_f,
                result.to_f64(),
                expected
            );
        }
    }

    // --- log tests ---

    #[test]
    fn log_base_10_of_100() {
        let hundred = mk(100, 100);
        let ten = mk(10, 100);
        let result = hundred.log(&ten, 100, MODE).expect("log_10(100)");
        assert!(
            (result.to_f64() - 2.0).abs() < 1e-14,
            "log_10(100) = {}, expected 2",
            result.to_f64()
        );
    }

    #[test]
    fn log_base_x_of_x_is_one() {
        let prec = 100u32;
        for x_f in [2.0f64, std::f64::consts::E, 10.0, 100.0] {
            let x = flt(x_f, prec);
            let result = x.log(&x.clone(), prec, MODE).expect("log_x(x)");
            assert!(
                (result.to_f64() - 1.0).abs() < 1e-12,
                "log_{}({}) ≠ 1, got {}",
                x_f,
                x_f,
                result.to_f64()
            );
        }
    }

    #[test]
    fn log_base_10_of_1000() {
        let val = mk(1000, 100);
        let ten = mk(10, 100);
        let result = val.log(&ten, 100, MODE).expect("log_10(1000)");
        assert!(
            (result.to_f64() - 3.0).abs() < 1e-14,
            "log_10(1000) = {}, expected 3",
            result.to_f64()
        );
    }

    #[test]
    fn log_base_non_positive_is_error() {
        let pos = mk(8, 64);
        let zero_base = mk(0, 64);
        let neg_base = mk(-2, 64);
        assert!(pos.log(&zero_base, 64, MODE).is_err());
        assert!(pos.log(&neg_base, 64, MODE).is_err());
    }

    #[test]
    fn log_of_non_positive_is_error() {
        let ten = mk(10, 64);
        let zero_val = mk(0, 64);
        let neg_val = mk(-1, 64);
        assert!(zero_val.log(&ten, 64, MODE).is_err());
        assert!(neg_val.log(&ten, 64, MODE).is_err());
    }

    #[test]
    fn log_base_1_is_error() {
        let val = mk(5, 64);
        let one = mk(1, 64);
        assert!(val.log(&one, 64, MODE).is_err());
    }
}
