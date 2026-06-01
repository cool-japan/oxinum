//! AGM-based natural logarithm for `BigFloat`.
//!
//! Implements `BigFloat::ln_agm(prec, mode)` using the arithmetic-geometric
//! mean algorithm. This is algorithmically distinct from the Newton-Raphson +
//! Taylor `ln` in `float_ln.rs`.
//!
//! # Algorithm
//!
//! ## Core formula (Borwein & Borwein)
//!
//! For a sufficiently large argument `s >> 1`:
//!
//! ```text
//! ln(s) = π / (2 · AGM(1, 4/s))
//! ```
//!
//! This follows from the connection between the complete elliptic integral
//! K(k) and the AGM: `K(k) = π / (2 · AGM(1, √(1−k²)))`, combined with the
//! limit `K(k) → (1/2) · ln(4/(1−k²))` as `k → 1`. Setting `k² → 1 − (4/s)²`
//! and taking `s → ∞` yields the identity above.
//!
//! ## Argument reduction
//!
//! For arbitrary `x > 0`:
//!
//! 1. Find `floor(log₂(x)) = x.exponent + x.mantissa.bit_length() - 1`.
//! 2. Choose a left-shift `K` so that `s = x · 2^K` satisfies
//!    `floor(log₂(s)) ≥ work_prec/2 + 10`.
//! 3. Compute `ln(s) = π / (2 · AGM(1, 4/s))` at `work_prec` guard bits.
//! 4. Recover `ln(x) = ln(s) − K · ln(2)`.
//!
//! ## AGM iteration
//!
//! Starting from `a₀ = 1`, `b₀ = 4/s`, the arithmetic-geometric mean iteration
//! is:
//!
//! ```text
//! a_{n+1} = (a_n + b_n) / 2
//! b_{n+1} = sqrt(a_n · b_n)
//! ```
//!
//! Convergence is quadratic: the number of correct bits doubles each iteration.
//! Starting with `b₀ ≈ 4/s ≈ 2^(−work_prec/2)`, convergence occurs in
//! approximately `log₂(work_prec) + 8` iterations.

use oxinum_core::{OxiNumError, OxiNumResult, Sign};

use super::constants::{ln2, pi};
use super::float::{BigFloat, RoundingMode};

impl BigFloat {
    /// Return `ln(self)` at `prec` bits using the AGM algorithm.
    ///
    /// This is algorithmically distinct from [`BigFloat::ln`], which uses
    /// Newton-Raphson iteration on the exponential. The AGM method converges
    /// quadratically from the start (no f64 seed required) and avoids calling
    /// `exp` internally, making it independent of the Taylor-series path.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] if `self <= 0` or `prec == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let two = BigFloat::from_i64(2, 64, RoundingMode::HalfEven);
    /// let result = two.ln_agm(64, RoundingMode::HalfEven).expect("ln_agm(2)");
    /// assert!((result.to_f64() - std::f64::consts::LN_2).abs() < 1e-14);
    /// ```
    pub fn ln_agm(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        if prec == 0 {
            return Err(OxiNumError::Domain("ln_agm: precision must be > 0".into()));
        }

        // --- IEEE-754 non-finite guards (placed after prec==0 check to avoid
        //     BigFloat::nan(0) panic) ---
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
            return Err(OxiNumError::Domain("ln_agm of zero is undefined".into()));
        }
        if self.sign == Sign::Negative {
            return Err(OxiNumError::Domain(
                "ln_agm of a negative number is undefined for real BigFloat".into(),
            ));
        }

        // --- Special case: ln(1) = 0 ---
        {
            let one = BigFloat::from_i64(1, prec, mode);
            if self == &one {
                return Ok(BigFloat::zero(prec));
            }
        }

        // Guard bits: need enough room for the AGM formula constant error.
        // prec + 64 is generous; it covers any reasonable input range.
        let guard = 64u32;
        let work_prec = prec.saturating_add(guard);

        // --- Argument reduction ---
        //
        // floor(log2(self)) = self.exponent + self.mantissa.bit_length() - 1
        // (The mantissa is normalized: bit_length == self.precision.)
        let cur_log2: i64 = self
            .exponent
            .saturating_add(self.mantissa.bit_length() as i64 - 1);

        // We need floor(log2(s)) >= work_prec/2 + 10 for the AGM formula.
        // s = self * 2^shift_k  =>  floor(log2(s)) = cur_log2 + shift_k.
        let target_log2 = (work_prec / 2 + 10) as i64;
        // shift_k can be negative (i.e., x is already large) — that's fine;
        // we use saturating arithmetic and allow it to be 0 or negative.
        let shift_k: i64 = target_log2 - cur_log2;

        // Build s = self * 2^shift_k at work_prec.
        // Adjust exponent directly: s = BigFloat { same mantissa, exp + shift_k }.
        let s = {
            let new_exp = self.exponent.saturating_add(shift_k);
            BigFloat::from_parts(
                Sign::Positive,
                self.mantissa.clone(),
                new_exp,
                work_prec,
                mode,
            )
        };

        // --- Compute ln(s) via the AGM formula ---
        let ln_s = agm_ln_large(&s, work_prec, mode)?;

        // --- Recover ln(x) = ln(s) - shift_k * ln(2) ---
        let ln_result = if shift_k == 0 {
            ln_s
        } else {
            let ln2_val = ln2(work_prec)?;
            let shift_f = BigFloat::from_i64(shift_k, work_prec, mode);
            let correction = shift_f.mul_ref_with_mode(&ln2_val, mode);
            ln_s.sub_ref_with_mode(&correction, mode)
        };

        Ok(ln_result.with_precision(prec, mode))
    }
}

/// Compute `ln(s)` for a large positive `s` using the AGM formula:
///
/// ```text
/// ln(s) = π / (2 · AGM(1, 4/s))
/// ```
///
/// This is valid when `s >> 1` (specifically when `floor(log2(s)) ≥ work_prec/2`).
/// The caller must ensure this precondition is met (via argument reduction).
fn agm_ln_large(s: &BigFloat, work_prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    // a0 = 1
    let a = BigFloat::from_i64(1, work_prec, mode);
    // b0 = 4 / s
    let four = BigFloat::from_i64(4, work_prec, mode);
    let b = four.div_ref_with_mode(s, mode)?;

    // Compute AGM(a, b)
    let agm_val = agm_iterate(a, b, work_prec, mode)?;

    // pi / (2 * agm_val)
    let pi_val = pi(work_prec)?;
    let two = BigFloat::from_i64(2, work_prec, mode);
    let two_agm = two.mul_ref_with_mode(&agm_val, mode);
    let result = pi_val.div_ref_with_mode(&two_agm, mode)?;

    Ok(result)
}

/// Compute the arithmetic-geometric mean `AGM(a0, b0)`.
///
/// The iteration converges quadratically; for `b0 ≈ 4/s` with
/// `log2(s) ≥ work_prec/2`, the iteration requires at most
/// `log2(work_prec) + 8` steps.
///
/// Convergence test: the value has converged when the top-bit position of
/// `|a - b|` satisfies `exponent + bit_length - 1 < -(work_prec as i64) + 4`.
fn agm_iterate(
    mut a: BigFloat,
    mut b: BigFloat,
    work_prec: u32,
    mode: RoundingMode,
) -> OxiNumResult<BigFloat> {
    // Number of iterations: quadratic convergence from ~work_prec/2 correct
    // bits to work_prec. Iterations needed: ceil(log2(work_prec / (work_prec/2)))
    // ≈ log2(2) = 1, but we add generous headroom because the initial gap can
    // be larger. Mirror the existing newton_ln pattern.
    let max_iters: u32 = if work_prec <= 64 {
        10
    } else {
        let n = (work_prec as f64).log2().ceil() as u32 + 8;
        n.min(64)
    };

    // Convergence threshold: |a - b| < 2^(-(work_prec - 4))
    // Expressed as: top_bit_pos < -((work_prec as i64) - 4)
    let threshold = -((work_prec as i64) - 4);

    for _ in 0..max_iters {
        // a_new = (a + b) / 2
        // Halving is exact: increment exponent by -1 (equivalent to * 0.5).
        // We compute a + b first, then halve the result.
        let sum = a.add_ref_with_mode(&b, mode);
        // Halve: adjust exponent by -1 (exact, no rounding).
        let a_new = BigFloat::from_parts(
            sum.sign(),
            sum.mantissa().clone(),
            sum.exponent().saturating_sub(1),
            work_prec,
            mode,
        );

        // b_new = sqrt(a * b)
        let product = a.mul_ref_with_mode(&b, mode);
        let b_new = product.sqrt(work_prec, mode)?;

        // Convergence check: |a_new - b_new|
        let diff = a_new.sub_ref_with_mode(&b_new, mode).abs();
        if diff.is_zero() {
            return Ok(a_new);
        }
        // top_bit_pos = diff.exponent + diff.mantissa.bit_length() - 1
        let top_bit_pos = diff
            .exponent()
            .saturating_add(diff.mantissa().bit_length() as i64 - 1);
        if top_bit_pos < threshold {
            return Ok(a_new);
        }

        a = a_new;
        b = b_new;
    }

    // Return best estimate (should have converged within max_iters).
    Ok(a)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::RoundingMode;

    fn mk(n: i64, prec: u32) -> BigFloat {
        BigFloat::from_i64(n, prec, RoundingMode::HalfEven)
    }

    /// Check |a - b| < 2^{-tol_bits} using exponent-based comparison.
    fn approx_eq_bits(a: &BigFloat, b: &BigFloat, tol_bits: u32) -> bool {
        let diff = a.sub_ref_with_mode(b, RoundingMode::HalfEven).abs();
        if diff.is_zero() {
            return true;
        }
        let top_bit_pos = diff
            .exponent()
            .saturating_add(diff.mantissa().bit_length() as i64 - 1);
        top_bit_pos < -(tol_bits as i64)
    }

    #[test]
    fn ln_agm_one_is_zero() {
        let x = mk(1, 100);
        let result = x.ln_agm(100, RoundingMode::HalfEven).expect("ln_agm(1)");
        assert!(result.is_zero(), "ln_agm(1) should be 0, got: {result:?}");
    }

    #[test]
    fn ln_agm_zero_is_domain_error() {
        let x = mk(0, 64);
        let result = x.ln_agm(64, RoundingMode::HalfEven);
        assert!(
            matches!(result, Err(OxiNumError::Domain(_))),
            "Expected Domain error for ln_agm(0), got: {result:?}"
        );
    }

    #[test]
    fn ln_agm_negative_is_domain_error() {
        let x = mk(-1, 64);
        let result = x.ln_agm(64, RoundingMode::HalfEven);
        assert!(
            matches!(result, Err(OxiNumError::Domain(_))),
            "Expected Domain error for ln_agm(-1), got: {result:?}"
        );
    }

    #[test]
    fn ln_agm_prec_zero_is_domain_error() {
        let x = mk(2, 64);
        let result = x.ln_agm(0, RoundingMode::HalfEven);
        assert!(
            matches!(result, Err(OxiNumError::Domain(_))),
            "Expected Domain error for prec=0, got: {result:?}"
        );
    }

    #[test]
    fn ln_agm_e_is_approximately_one() {
        use crate::native::e_const;
        let prec = 100u32;
        let e = e_const(prec).expect("e_const");
        let result = e.ln_agm(60, RoundingMode::HalfEven).expect("ln_agm(e)");
        let expected = mk(1, 60);
        assert!(
            approx_eq_bits(&result, &expected, 45),
            "ln_agm(e) should be ≈ 1, got: {} (diff from 1: {})",
            result.to_f64(),
            (result.to_f64() - 1.0).abs()
        );
    }

    #[test]
    fn ln_agm_matches_newton_ln() {
        // Cross-validate AGM ln against Newton-Raphson ln for several inputs.
        let prec = 80u32;
        let tol = 60u32; // Allow ~20 guard bits of slack
        let mode = RoundingMode::HalfEven;
        for n in [2i64, 7, 100, 1000] {
            let x = BigFloat::from_i64(n, prec + 64, mode);
            let ln_newton = x.ln(prec, mode).expect("newton ln");
            let ln_agm = x.ln_agm(prec, mode).expect("agm ln");
            assert!(
                approx_eq_bits(&ln_newton, &ln_agm, tol),
                "ln_agm({n}) vs Newton mismatch: newton={}, agm={}",
                ln_newton.to_f64(),
                ln_agm.to_f64()
            );
        }
    }

    #[test]
    fn ln_agm_small_fraction() {
        // ln(0.5) = -ln(2)
        use crate::native::ln2 as ln2_const;
        let prec = 80u32;
        let mode = RoundingMode::HalfEven;
        // 0.5 = 1 * 2^(-1)
        let half = BigFloat::from_parts(
            Sign::Positive,
            oxinum_int::native::BigUint::one(),
            -1,
            prec,
            mode,
        );
        let ln_half = half.ln_agm(prec, mode).expect("ln_agm(0.5)");
        let neg_ln2 = ln2_const(prec).expect("ln2").neg();
        assert!(
            approx_eq_bits(&ln_half, &neg_ln2, 60),
            "ln_agm(0.5) should be -ln(2), got: {}",
            ln_half.to_f64()
        );
    }
}
