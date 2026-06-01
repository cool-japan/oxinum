//! `atan` and `atan2` for native `BigFloat`.
//!
//! # Algorithm for `atan(x)`
//!
//! 1. **Special cases:** `atan(0) = 0`.
//!
//! 2. **Range reduction for `|x| > 1`:** `atan(x) = sign(x) · π/2 − atan(1/|x|)`.
//!    After this step, the input satisfies `|x| ≤ 1`.
//!
//! 3. **Half-angle acceleration:** apply
//!    `atan(x) = 2 · atan(x / (1 + √(1 + x²)))` repeatedly until `|x| < 2^{−32}`.
//!    Count the number of halvings `m`. This reduces to an exponentially small
//!    argument for which the Taylor series converges in O(prec / 32) terms.
//!
//! 4. **Taylor series for small `u`:**
//!    `atan(u) = u − u³/3 + u⁵/5 − …`  (convergent for `|u| < 1`).
//!    Term count: with `|u| < 2^{−32}`, the `(2k+1)`-th term is bounded by
//!    `|u|^(2k+1) / (2k+1) < 2^{−32(2k+1)} / (2k+1)`.  For `k = prec/64` this
//!    is already below `2^{−prec}`, so `n_terms = prec / 64 + 8` suffices.
//!
//! 5. **Undo half-angle:** multiply result by `2^m`.
//!
//! 6. **Undo range reduction:** if `|x| > 1`, return `sign · (π/2 − result)`.
//!
//! # Algorithm for `atan2(y, x)`
//!
//! Standard four-quadrant table:
//!
//! | y | x  | result            |
//! |---|----|-------------------|
//! | 0 | 0  | 0 (by convention) |
//! | * | >0 | atan(y/x)         |
//! | ≥0| <0 | π + atan(y/x)     |
//! | <0| <0 | −π + atan(y/x)    |
//! | ≥0| 0  | +π/2              |
//! | <0| 0  | −π/2              |

use oxinum_core::{OxiNumError, OxiNumResult, Sign};

use super::constants::pi;
use super::float::{BigFloat, RoundingMode};

// ---------------------------------------------------------------------------
// Half-angle reduction threshold
// ---------------------------------------------------------------------------

/// Apply `atan(x) = 2·atan(x / (1 + sqrt(1 + x²)))` until `|x| < 2^{-32}`.
/// Returns `(reduced_x, m)` where `m` is the number of halvings applied.
fn half_angle_reduce(x: BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<(BigFloat, u32)> {
    let one = BigFloat::from_i64(1, prec, mode);
    let mut u = x;
    let mut m = 0u32;

    // Threshold: |u| < 2^{-32} means exponent(mantissa) < -32 relative to 1.
    // We check via to_f64: safe as long as |u| ≤ 1, which it is after the
    // range-reduction step.
    loop {
        // f64 comparison is safe here since |u| ≤ 1.
        let abs_f64 = u.abs().to_f64();
        if abs_f64 < f64::from_bits(0x3e00_0000_0000_0000u64) {
            // 2^{-31} — conservative threshold
            break;
        }
        if m >= 300 {
            // Safety cap — should never be reached for any sane precision.
            break;
        }
        // u = u / (1 + sqrt(1 + u²))
        let u_sq = u.mul_ref_with_mode(&u, mode).with_precision(prec, mode);
        let one_plus_u_sq = one.add_ref_with_mode(&u_sq, mode);
        let sqrt_val = one_plus_u_sq.sqrt(prec, mode)?;
        let denom = one.add_ref_with_mode(&sqrt_val, mode);
        u = u.div_ref_with_mode(&denom, mode)?;
        u = u.with_precision(prec, mode);
        m += 1;
    }

    Ok((u, m))
}

// ---------------------------------------------------------------------------
// Taylor series for atan on small arguments
// ---------------------------------------------------------------------------

/// Compute `atan(u)` via Taylor series for `|u| < 2^{-31}`.
///
/// `atan(u) = u − u³/3 + u⁵/5 − …`
///
/// Term count: `prec / 64 + 8` suffices because each halving reduces by
/// 32 bits, so `|u|^{2k+1}/(2k+1) < 2^{-32(2k+1)} / (2k+1) < 2^{-prec}`
/// for `k ≥ prec/64`.
fn atan_taylor(u: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    if u.is_zero() {
        return Ok(BigFloat::zero(prec));
    }

    let u_sq = u.mul_ref_with_mode(u, mode).with_precision(prec, mode);
    // term = u; result = u
    let mut term = u.clone().with_precision(prec, mode);
    let mut result = term.clone();

    let n_terms: u64 = (prec as u64) / 64 + 8;

    for k in 1..=n_terms {
        // term *= -u²;  then divide by (2k+1)
        let numer = term
            .mul_ref_with_mode(&u_sq, mode)
            .with_precision(prec, mode);
        term = numer.neg().with_precision(prec, mode);
        let denom_val = 2 * k + 1;
        let denom_i64 = denom_val.min(i64::MAX as u64) as i64;
        let denom_f = BigFloat::from_i64(denom_i64, prec, mode);
        let scaled = term
            .div_ref_with_mode(&denom_f, mode)
            .map_err(|e| OxiNumError::Precision(format!("atan_taylor denom zero: {e}").into()))?;
        result = result
            .add_ref_with_mode(&scaled, mode)
            .with_precision(prec, mode);
    }

    Ok(result.with_precision(prec, mode))
}

// ---------------------------------------------------------------------------
// Public BigFloat methods
// ---------------------------------------------------------------------------

impl BigFloat {
    /// Return `atan(self)` at `prec` bits using the given rounding mode.
    ///
    /// The result lies in `(−π/2, π/2)`.
    ///
    /// # Errors
    ///
    /// Propagates sqrt or division errors (only if internal invariants break,
    /// which should not happen for well-formed inputs).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let one = BigFloat::from_i64(1, 64, RoundingMode::HalfEven);
    /// let a = one.atan(64, RoundingMode::HalfEven).expect("atan(1)");
    /// // atan(1) = π/4 ≈ 0.7853981633974483
    /// assert!((a.to_f64() - std::f64::consts::FRAC_PI_4).abs() < 1e-14);
    /// ```
    pub fn atan(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        // IEEE-754 non-finite guards
        if self.is_nan() {
            return Ok(BigFloat::nan(prec));
        }
        if self.is_infinite() {
            // atan(+Inf) = +π/2, atan(-Inf) = -π/2
            let pi_val = pi(prec.saturating_add(16))?;
            let two = BigFloat::from_i64(2, prec.saturating_add(16), mode);
            let half_pi = pi_val.div_ref_with_mode(&two, mode)?;
            return if self.sign == Sign::Negative {
                Ok(half_pi.neg().with_precision(prec, mode))
            } else {
                Ok(half_pi.with_precision(prec, mode))
            };
        }

        if self.is_zero() {
            return Ok(BigFloat::zero(prec));
        }

        // Work at higher precision to absorb rounding errors.
        let work_prec = prec.saturating_add(64);

        // We work with |x| and track the sign separately.
        let orig_sign = self.signum();
        let abs_x = self.abs().with_precision(work_prec, mode);

        let one = BigFloat::from_i64(1, work_prec, mode);

        // Step 1: Range reduction — if |x| > 1, use atan(x) = π/2 − atan(1/x).
        let (working_x, use_complement): (BigFloat, bool) = if abs_x > one.clone() {
            let inv = one.div_ref_with_mode(&abs_x, mode)?;
            (inv.with_precision(work_prec, mode), true)
        } else {
            (abs_x, false)
        };

        // Step 2: Half-angle acceleration.
        let (reduced, m) = half_angle_reduce(working_x, work_prec, mode)?;

        // Step 3: Taylor series for the small argument.
        let mut result = atan_taylor(&reduced, work_prec, mode)?;

        // Step 4: Undo half-angle by multiplying by 2^m.
        let two = BigFloat::from_i64(2, work_prec, mode);
        for _ in 0..m {
            result = result
                .mul_ref_with_mode(&two, mode)
                .with_precision(work_prec, mode);
        }

        // Step 5: Undo range reduction if |x| > 1.
        if use_complement {
            let pi_val = pi(work_prec)?;
            let two_wp = BigFloat::from_i64(2, work_prec, mode);
            let pi_over_2 = pi_val.div_ref_with_mode(&two_wp, mode)?;
            result = pi_over_2
                .sub_ref_with_mode(&result, mode)
                .with_precision(work_prec, mode);
        }

        // Step 6: Apply sign.
        if orig_sign < 0 {
            result = result.neg();
        }

        Ok(result.with_precision(prec, mode))
    }

    /// Return `atan2(self, x)` at `prec` bits using the given rounding mode.
    ///
    /// Conventionally `atan2(y, x)` — `self` is `y` and `x` is the argument.
    /// The result lies in `(−π, π]`.
    ///
    /// # Special cases
    ///
    /// - `atan2(0, 0) = 0` (by convention, not mathematically defined).
    /// - `atan2(y, 0) = ±π/2` depending on sign of `y`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigFloat::atan`].
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let y = BigFloat::from_i64(1, 64, RoundingMode::HalfEven);
    /// let x = BigFloat::from_i64(1, 64, RoundingMode::HalfEven);
    /// let a = y.atan2(&x, 64, RoundingMode::HalfEven).expect("atan2(1,1)");
    /// // atan2(1,1) = π/4 ≈ 0.7853981633974483
    /// assert!((a.to_f64() - std::f64::consts::FRAC_PI_4).abs() < 1e-14);
    /// ```
    pub fn atan2(&self, x: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        let y = self;

        // IEEE-754 non-finite guards
        // NaN propagates immediately.
        if y.is_nan() || x.is_nan() {
            return Ok(BigFloat::nan(prec));
        }
        // Both infinite: atan2(±Inf, ±Inf) — four-quadrant results per IEEE.
        if y.is_infinite() && x.is_infinite() {
            // atan2(+Inf,+Inf)=+π/4, atan2(+Inf,-Inf)=+3π/4
            // atan2(-Inf,+Inf)=-π/4, atan2(-Inf,-Inf)=-3π/4
            let pi_val = pi(prec.saturating_add(16))?;
            let four = BigFloat::from_i64(4, prec.saturating_add(16), mode);
            let pi_over_4 = pi_val
                .div_ref_with_mode(&four, mode)?
                .with_precision(prec, mode);
            let three_pi_over_4 = {
                let three = BigFloat::from_i64(3, prec, mode);
                three
                    .mul_ref_with_mode(&pi_over_4, mode)
                    .with_precision(prec, mode)
            };
            let (mag, apply_neg) = if x.sign == Sign::Negative {
                (three_pi_over_4, y.sign == Sign::Negative)
            } else {
                (pi_over_4, y.sign == Sign::Negative)
            };
            return Ok(if apply_neg { mag.neg() } else { mag });
        }
        // y is Inf, x is finite: atan2(+Inf, x) = +π/2, atan2(-Inf, x) = -π/2
        if y.is_infinite() {
            let pi_val = pi(prec.saturating_add(16))?;
            let two = BigFloat::from_i64(2, prec.saturating_add(16), mode);
            let half_pi = pi_val
                .div_ref_with_mode(&two, mode)?
                .with_precision(prec, mode);
            return if y.sign == Sign::Negative {
                Ok(half_pi.neg())
            } else {
                Ok(half_pi)
            };
        }
        // x is Inf, y is finite: atan2(y, +Inf) = +0, atan2(y, -Inf) = ±π
        if x.is_infinite() {
            if x.sign == Sign::Positive {
                return Ok(BigFloat::zero(prec));
            } else {
                // atan2(y, -Inf) = +π if y ≥ 0, -π if y < 0
                let pi_val = pi(prec.saturating_add(16))?.with_precision(prec, mode);
                return if y.sign == Sign::Negative {
                    Ok(pi_val.neg())
                } else {
                    Ok(pi_val)
                };
            }
        }

        let y_sign = y.signum();
        let x_sign = x.signum();

        // Both zero: return 0 by convention.
        if y.is_zero() && x.is_zero() {
            return Ok(BigFloat::zero(prec));
        }

        // x = 0: result is ±π/2.
        if x.is_zero() {
            let pi_val = pi(prec.saturating_add(16))?;
            let two = BigFloat::from_i64(2, prec.saturating_add(16), mode);
            let pi_over_2 = pi_val.div_ref_with_mode(&two, mode)?;
            return if y_sign >= 0 {
                Ok(pi_over_2.with_precision(prec, mode))
            } else {
                Ok(pi_over_2.neg().with_precision(prec, mode))
            };
        }

        // x > 0: atan(y/x).
        if x_sign > 0 {
            let work_prec = prec.saturating_add(16);
            let ratio = y
                .clone()
                .with_precision(work_prec, mode)
                .div_ref_with_mode(&x.clone().with_precision(work_prec, mode), mode)?;
            return ratio.atan(prec, mode);
        }

        // x < 0: atan(y/x) ± π.
        let work_prec = prec.saturating_add(32);
        let ratio = y
            .clone()
            .with_precision(work_prec, mode)
            .div_ref_with_mode(&x.clone().with_precision(work_prec, mode), mode)?;
        let atan_val = ratio.atan(work_prec, mode)?;
        let pi_val = pi(work_prec)?;

        if y_sign >= 0 {
            // y ≥ 0, x < 0: result in (π/2, π]
            let result = pi_val.add_ref_with_mode(&atan_val, mode);
            Ok(result.with_precision(prec, mode))
        } else {
            // y < 0, x < 0: result in (−π, −π/2)
            let result = atan_val.sub_ref_with_mode(&pi_val, mode);
            Ok(result.with_precision(prec, mode))
        }
    }
}

// ---------------------------------------------------------------------------
// Internal tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(n: i64, prec: u32) -> BigFloat {
        BigFloat::from_i64(n, prec, RoundingMode::HalfEven)
    }

    #[test]
    fn atan_zero() {
        let z = mk(0, 64);
        let r = z.atan(64, RoundingMode::HalfEven).expect("atan(0)");
        assert!(r.is_zero());
    }

    #[test]
    fn atan_one_is_pi_over_4() {
        let prec = 64u32;
        let one = mk(1, prec);
        let a = one.atan(prec, RoundingMode::HalfEven).expect("atan(1)");
        assert!(
            (a.to_f64() - std::f64::consts::FRAC_PI_4).abs() < 1e-14,
            "got {}",
            a.to_f64()
        );
    }

    #[test]
    fn atan_minus_one() {
        let prec = 64u32;
        let minus_one = mk(-1, prec);
        let a = minus_one
            .atan(prec, RoundingMode::HalfEven)
            .expect("atan(-1)");
        assert!((a.to_f64() + std::f64::consts::FRAC_PI_4).abs() < 1e-14);
    }

    #[test]
    fn atan2_quadrant_i() {
        let prec = 64u32;
        let mode = RoundingMode::HalfEven;
        let one = mk(1, prec);
        let a = one.atan2(&one, prec, mode).expect("atan2(1,1)");
        assert!((a.to_f64() - std::f64::consts::FRAC_PI_4).abs() < 1e-14);
    }

    #[test]
    fn atan2_negative_x() {
        let prec = 64u32;
        let mode = RoundingMode::HalfEven;
        let one = mk(1, prec);
        let neg_one = mk(-1, prec);
        // atan2(1, -1) = 3π/4
        let a = one.atan2(&neg_one, prec, mode).expect("atan2(1,-1)");
        let expected = 3.0 * std::f64::consts::FRAC_PI_4;
        assert!(
            (a.to_f64() - expected).abs() < 1e-13,
            "got {}, expected {}",
            a.to_f64(),
            expected
        );
    }

    #[test]
    fn atan2_zero_x_positive_y() {
        let prec = 64u32;
        let mode = RoundingMode::HalfEven;
        let one = mk(1, prec);
        let zero = mk(0, prec);
        let a = one.atan2(&zero, prec, mode).expect("atan2(1,0)");
        assert!((a.to_f64() - std::f64::consts::FRAC_PI_2).abs() < 1e-14);
    }

    #[test]
    fn atan2_zero_x_negative_y() {
        let prec = 64u32;
        let mode = RoundingMode::HalfEven;
        let neg_one = mk(-1, prec);
        let zero = mk(0, prec);
        let a = neg_one.atan2(&zero, prec, mode).expect("atan2(-1,0)");
        assert!((a.to_f64() + std::f64::consts::FRAC_PI_2).abs() < 1e-14);
    }
}
