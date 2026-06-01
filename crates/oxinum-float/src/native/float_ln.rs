//! Natural logarithm for `BigFloat`.
//!
//! Implements `BigFloat::ln(x, prec, mode)` using mantissa decomposition
//! followed by Newton's method on the exponential.
//!
//! # Algorithm
//!
//! 1. **Special cases**: `ln(0)` → Domain error, `ln(negative)` → Domain
//!    error, `ln(1)` = 0.
//! 2. **Mantissa decomposition**: Write `x = m * 2^K` where `m ∈ [0.5, 1)`.
//!    From the BigFloat representation: `x = mantissa * 2^e` with
//!    `mantissa.bit_length() == prec`. Set `m` = same mantissa with exponent
//!    `-prec` (so value = mantissa * 2^(-prec) ∈ [0.5, 1.0)) and `K = e + prec`.
//!    Then `ln(x) = ln(m) + K * ln(2)`.
//! 3. **Newton's method for ln(m)**:
//!    - Seed: `y_0 = ln(m)` from f64.
//!    - Iteration: `y_{i+1} = y_i + m/exp(y_i) - 1`.
//!    - Each step doubles correct bits; run `ceil(log2(prec/53)) + 3` iters.
//!    - Precision schedule doubles each iteration.
//! 4. **Combine**: `ln(x) = ln(m) + K * ln(2)`, round to `prec`.

use oxinum_core::{OxiNumError, OxiNumResult, Sign};

use super::constants::ln2;
use super::float::{BigFloat, RoundingMode};

impl BigFloat {
    /// Return `ln(self)` at `prec` bits using the chosen rounding mode.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] if `self <= 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let one = BigFloat::from_i64(1, 64, RoundingMode::HalfEven);
    /// let result = one.ln(64, RoundingMode::HalfEven).expect("ln(1)");
    /// assert!(result.is_zero());
    /// ```
    pub fn ln(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        assert!(prec > 0, "BigFloat precision must be > 0");

        // --- IEEE-754 non-finite guards ---
        if self.is_nan() {
            return Ok(BigFloat::nan(prec));
        }
        if self.is_infinite() {
            return if self.sign == Sign::Negative {
                Ok(BigFloat::nan(prec)) // ln(-Inf) = NaN
            } else {
                Ok(BigFloat::infinity(prec)) // ln(+Inf) = +Inf
            };
        }

        // --- Special cases ---
        if self.is_zero() {
            return Err(OxiNumError::Domain("ln of zero is undefined".into()));
        }
        if self.sign == Sign::Negative {
            return Err(OxiNumError::Domain(
                "ln of negative number is undefined for real BigFloat".into(),
            ));
        }

        // --- Special case: ln(1) = 0 ---
        let one = BigFloat::from_i64(1, prec, mode);
        if self == &one {
            return Ok(BigFloat::zero(prec));
        }

        // --- Mantissa decomposition ---
        // x = mantissa * 2^exponent, with mantissa.bit_length() == precision.
        // Write x = m * 2^K where m = mantissa * 2^(-prec) ∈ [0.5, 1.0)
        // and K = exponent + prec.
        let prec_i = prec as i64;
        let big_k = self.exponent.saturating_add(prec_i);

        // Construct m as a BigFloat with the same mantissa but exponent = -prec.
        // This gives value = mantissa * 2^(-prec) ∈ [0.5, 1.0).
        let guard = 32u32 + prec / 16 + 8;
        let work_prec = prec.saturating_add(guard);

        let m = BigFloat::from_parts(
            Sign::Positive,
            self.mantissa.clone(),
            -prec_i,
            work_prec,
            mode,
        );

        // --- Newton's method for y = ln(m) ---
        // m is in [0.5, 1.0), so ln(m) is in (-ln(2), 0].
        // f64 seed for ln(m): use m.to_f64().ln()
        let m_f64 = m.to_f64();
        let ln_m_f64 = m_f64.ln(); // valid because m_f64 ∈ (0, 1)

        let ln_m = newton_ln(&m, ln_m_f64, work_prec, mode)?;

        // --- Combine: ln(x) = ln(m) + K * ln(2) ---
        let k_float = BigFloat::from_i64(big_k, work_prec, mode);
        let ln2_val = ln2(work_prec)?;
        let k_times_ln2 = k_float.mul_ref_with_mode(&ln2_val, mode);
        let result = ln_m.add_ref_with_mode(&k_times_ln2, mode);

        Ok(result.with_precision(prec, mode))
    }
}

/// Compute `ln(m)` via Newton's method: `y_{i+1} = y_i + m/exp(y_i) - 1`.
///
/// Converges quadratically from the f64 seed `ln_m_f64`.
///
/// Each iteration doubles the number of correct bits, starting from ~53 bits.
/// We run `ceil(log2(work_prec / 53)) + 3` iterations (capped at 30).
fn newton_ln(
    m: &BigFloat,
    ln_m_f64: f64,
    work_prec: u32,
    mode: RoundingMode,
) -> OxiNumResult<BigFloat> {
    // Seed from f64
    let mut current_prec: u32 = 64;
    let mut y = BigFloat::from_f64(ln_m_f64, current_prec)?;

    // Number of Newton iterations needed to reach work_prec from ~53 bits.
    // Each iteration: bits_correct *= 2
    // Need: 53 * 2^n >= work_prec => n = ceil(log2(work_prec / 53))
    let n_iters = if work_prec <= 64 {
        3u32
    } else {
        let ratio = (work_prec as f64) / 53.0;
        ratio.log2().ceil() as u32 + 3
    };
    let n_iters = n_iters.min(60);

    for _ in 0..n_iters {
        // Double precision for this step (up to work_prec)
        current_prec = (current_prec.saturating_mul(2)).min(work_prec);

        let m_at_prec = m.clone().with_precision(current_prec, mode);
        let y_at_prec = y.clone().with_precision(current_prec, mode);

        // Compute exp(y) at current precision
        let ey = y_at_prec.exp(current_prec, mode)?;

        // correction = m / exp(y)
        let correction = m_at_prec.div_ref_with_mode(&ey, mode)?;

        // y_{i+1} = y_i + m/exp(y_i) - 1
        let one = BigFloat::from_i64(1, current_prec, mode);
        let diff = correction.sub_ref_with_mode(&one, mode);
        y = y_at_prec
            .add_ref_with_mode(&diff, mode)
            .with_precision(current_prec, mode);
    }

    Ok(y.with_precision(work_prec, mode))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{e_const, ln2 as ln2_const, RoundingMode};

    fn mk(n: i64, prec: u32) -> BigFloat {
        BigFloat::from_i64(n, prec, RoundingMode::HalfEven)
    }

    fn approx_eq_bits(a: &BigFloat, b: &BigFloat, tol_bits: u32) -> bool {
        let diff = a.sub_ref(b).abs();
        // threshold = 2^(-(tol_bits)) constructed as a BigFloat at work precision
        // Use: diff.exponent + diff.mantissa.bit_length() - 1 < -tol_bits as a proxy
        // for "diff < 2^(-tol_bits)".
        // More precisely: compare threshold via from_parts.
        if diff.is_zero() {
            return true;
        }
        // diff.value approx = diff.mantissa * 2^diff.exponent
        // diff < 2^(-tol_bits) iff diff.exponent + diff.mantissa.bit_length() - 1 < -(tol_bits as i64)
        let diff_top_exp = diff
            .exponent
            .saturating_add(diff.mantissa.bit_length() as i64 - 1);
        diff_top_exp < -(tol_bits as i64)
    }

    #[test]
    fn ln_one_is_zero() {
        let x = mk(1, 100);
        let result = x.ln(100, RoundingMode::HalfEven).expect("ln(1)");
        assert!(result.is_zero(), "ln(1) should be 0, got: {result:?}");
    }

    #[test]
    fn ln_zero_is_domain_error() {
        let x = mk(0, 64);
        let result = x.ln(64, RoundingMode::HalfEven);
        assert!(
            matches!(result, Err(OxiNumError::Domain(_))),
            "Expected Domain error, got: {result:?}"
        );
    }

    #[test]
    fn ln_negative_is_domain_error() {
        let x = mk(-1, 64);
        let result = x.ln(64, RoundingMode::HalfEven);
        assert!(
            matches!(result, Err(OxiNumError::Domain(_))),
            "Expected Domain error, got: {result:?}"
        );
    }

    #[test]
    fn ln_e_is_one() {
        let prec = 100u32;
        let e = e_const(prec).expect("e_const");
        let result = e.ln(prec, RoundingMode::HalfEven).expect("ln(e)");
        let one = mk(1, prec);
        assert!(
            approx_eq_bits(&result, &one, 85),
            "ln(e) should ≈ 1, got: {} (diff from 1: {})",
            result.to_f64(),
            (result.to_f64() - 1.0).abs()
        );
    }

    #[test]
    fn ln2_matches_constant() {
        let prec = 100u32;
        let two = mk(2, prec);
        let computed = two.ln(prec, RoundingMode::HalfEven).expect("ln(2)");
        let expected = ln2_const(prec).expect("ln2 constant");
        assert!(
            approx_eq_bits(&computed, &expected, 85),
            "ln(2) should match ln2 constant, diff = {}",
            (computed.to_f64() - expected.to_f64()).abs()
        );
    }
}
