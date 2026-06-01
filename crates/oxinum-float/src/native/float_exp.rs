//! Exponential function for `BigFloat`.
//!
//! Implements `BigFloat::exp(x, prec, mode)` using argument reduction followed
//! by a direct Taylor series evaluation, then k-fold squaring to recover the
//! result.
//!
//! # Algorithm
//!
//! 1. **Special cases**: `exp(0) = 1`. Overflow/underflow guarded by f64 range.
//! 2. **Argument reduction**: Divide x by `2^k` so that `|x/2^k|` is small.
//!    This is a pure exponent shift — no loss of precision.
//! 3. **Taylor series**: `e^y = Σ_{n=0}^{N} y^n / n!` computed iteratively
//!    at `work_prec = prec + guard` bits.
//! 4. **Squaring back**: `e^x = (e^(x/2^k))^(2^k)` — k squarings at work_prec.
//! 5. **Round** to requested `prec`.

use oxinum_core::{OxiNumError, OxiNumResult, Sign};

use super::float::{BigFloat, RoundingMode};

impl BigFloat {
    /// Return `e^self` at `prec` bits using the chosen rounding mode.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Overflow`] if `self` is so large that the result
    ///   exceeds the representable range (|x| > 745).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode, e_const};
    /// let one = BigFloat::from_i64(1, 100, RoundingMode::HalfEven);
    /// let result = one.exp(100, RoundingMode::HalfEven).expect("exp(1)");
    /// let e = e_const(100).expect("e_const");
    /// let diff = (result.to_f64() - e.to_f64()).abs();
    /// assert!(diff < 1e-14);
    /// ```
    pub fn exp(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        assert!(prec > 0, "BigFloat precision must be > 0");

        // --- IEEE-754 non-finite guards (must be before to_f64() call) ---
        if self.is_nan() {
            return Ok(BigFloat::nan(prec));
        }
        if self.is_infinite() {
            return if self.sign == Sign::Negative {
                Ok(BigFloat::zero(prec)) // exp(-Inf) = +0
            } else {
                Ok(BigFloat::infinity(prec)) // exp(+Inf) = +Inf
            };
        }

        // --- Special case: exp(0) = 1 ---
        if self.is_zero() {
            return Ok(BigFloat::from_i64(1, prec, mode));
        }

        // --- Overflow / underflow check via f64 approximation ---
        let x_f64 = self.to_f64();
        // ln(f64::MAX) ≈ 709.78; add a small margin
        if x_f64 > 745.0 {
            return Err(OxiNumError::Overflow(
                "exp: argument too large (result exceeds BigFloat range)".into(),
            ));
        }
        // For very negative x the result is effectively 0 (subnormal/underflow)
        if x_f64 < -745.0 {
            return Ok(BigFloat::zero(prec));
        }

        // --- Argument reduction ---
        // Choose k so |x / 2^k| <= 1, with extra reduction for precision.
        // k = max(0, ceil(log2(|x|+1))) + prec/64 + 4
        let x_abs = x_f64.abs();
        let log2_x_abs = if x_abs >= 1.0 {
            x_abs.log2().ceil() as u64
        } else {
            0u64
        };
        let guard = 32u32 + (prec / 64 + 4);
        let work_prec = prec.saturating_add(guard);
        let k = log2_x_abs + (prec as u64 / 64) + 4;

        // Construct x_reduced = self with exponent decreased by k.
        // This is exact: value = mantissa * 2^(exponent - k)
        let x_reduced = if self.is_zero() {
            BigFloat::zero(work_prec)
        } else {
            BigFloat::from_parts(
                self.sign,
                self.mantissa.clone(),
                self.exponent.saturating_sub(k as i64),
                work_prec,
                mode,
            )
        };

        // --- Taylor series sum ---
        // e^y = Σ_{n=0}^{N} y^n / n!
        // After reduction, |y| <= 2^{-(prec/64 + 4)}.
        // Number of terms: N = max(64, prec / 4 + 16) to ensure full precision.
        let n_terms = (prec / 4 + 16).max(64) as u64;
        let mut result = exp_taylor(&x_reduced, n_terms, work_prec, mode)?;

        // --- Square back k times ---
        // e^x = (e^(x/2^k))^(2^k) = result^(2^k)
        for _ in 0..k {
            result = result
                .mul_ref_with_mode(&result.clone(), mode)
                .with_precision(work_prec, mode);
        }

        Ok(result.with_precision(prec, mode))
    }
}

/// Compute `e^y` via direct Taylor series at `work_prec` bits.
///
/// `e^y = Σ_{n=0}^{n_terms} y^n / n!`
///
/// Iterates: `term_n = term_{n-1} * y / n`, starting from `term_0 = 1`.
fn exp_taylor(
    y: &BigFloat,
    n_terms: u64,
    work_prec: u32,
    mode: RoundingMode,
) -> OxiNumResult<BigFloat> {
    // term = y^0 / 0! = 1
    let mut term = BigFloat::from_i64(1, work_prec, mode);
    // result = sum of terms
    let mut result = term.clone();

    for k in 1..=n_terms {
        // term_k = term_{k-1} * y / k
        term = term
            .mul_ref_with_mode(y, mode)
            .with_precision(work_prec, mode);
        let k_float = BigFloat::from_i64(k as i64, work_prec, mode);
        term = term.div_ref_with_mode(&k_float, mode)?;
        term = term.with_precision(work_prec, mode);

        // Accumulate
        result = result.add_ref_with_mode(&term, mode);
        result = result.with_precision(work_prec, mode);

        // Early exit: term is negligible relative to 2^(-work_prec)
        // This avoids wasting iterations once convergence is achieved.
        if term.is_zero() {
            break;
        }
        // Check if term is negligible: compare magnitudes via exponent.
        // result's value is roughly O(1); term < 2^{-work_prec} when
        // term.exponent + term.mantissa.bit_length() < result.exponent + result.mantissa.bit_length() - work_prec
        if !result.is_zero() && !term.is_zero() {
            let term_top = term
                .exponent
                .saturating_add(term.mantissa.bit_length() as i64 - 1);
            let result_top = result
                .exponent
                .saturating_add(result.mantissa.bit_length() as i64 - 1);
            if term_top < result_top.saturating_sub(work_prec as i64 + 8) {
                break;
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{e_const, RoundingMode};

    fn mk(n: i64, prec: u32) -> BigFloat {
        BigFloat::from_i64(n, prec, RoundingMode::HalfEven)
    }

    #[test]
    fn exp_zero_is_one() {
        let x = mk(0, 64);
        let result = x.exp(64, RoundingMode::HalfEven).expect("exp(0)");
        let one = mk(1, 64);
        assert_eq!(result, one, "exp(0) should == 1");
    }

    #[test]
    fn exp_one_approx_e() {
        let x = mk(1, 100);
        let result = x.exp(100, RoundingMode::HalfEven).expect("exp(1)");
        let e = e_const(100).expect("e_const(100)");
        let diff = (result.to_f64() - e.to_f64()).abs();
        assert!(diff < 1e-14, "exp(1) diff from e: {diff}");
    }

    #[test]
    fn exp_overflow() {
        let x = BigFloat::from_f64(800.0, 64).expect("from_f64");
        let result = x.exp(64, RoundingMode::HalfEven);
        assert!(
            matches!(result, Err(OxiNumError::Overflow(_))),
            "Expected Overflow, got: {result:?}"
        );
    }

    #[test]
    fn exp_large_negative_returns_zero() {
        let x = BigFloat::from_f64(-800.0, 64).expect("from_f64");
        let result = x.exp(64, RoundingMode::HalfEven).expect("exp(-800)");
        assert!(result.is_zero(), "exp(-800) should be zero");
    }

    #[test]
    fn exp_small_value_cross_val() {
        // exp(0.5) ≈ 1.6487212707
        let x = BigFloat::from_f64(0.5, 64).expect("0.5");
        let result = x.exp(64, RoundingMode::HalfEven).expect("exp(0.5)");
        let expected = 0.5_f64.exp();
        let diff = (result.to_f64() - expected).abs();
        assert!(diff < 1e-14, "exp(0.5) diff: {diff}");
    }
}
