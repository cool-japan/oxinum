//! Trigonometric functions for native `BigFloat`: sin, cos, tan.
//!
//! # Algorithms
//!
//! ## Argument reduction (quadrant)
//!
//! Given `x`, compute `q = round(x / (π/2))` and `u = x - q * (π/2)`.
//! The remainder `u ∈ [−π/4, π/4]` satisfies `|u| ≤ π/4 ≈ 0.785`.
//!
//! For large `|x|` where `|q|` would overflow `i64`, the quotient is computed
//! via `BigFloat` arithmetic: we extract the integer `q` from the BigFloat
//! representation by shifting the mantissa using the stored exponent.
//!
//! The quadrant `q mod 4` maps to the final signs and sin/cos exchange:
//!
//! | quadrant | sin(x)   | cos(x)   |
//! |----------|----------|----------|
//! | 0        | +sin(u)  | +cos(u)  |
//! | 1        | +cos(u)  | −sin(u)  |
//! | 2        | −sin(u)  | −cos(u)  |
//! | 3        | −cos(u)  | +sin(u)  |
//!
//! ## Taylor series for `|u| ≤ π/4`
//!
//! `sin(u) = u − u³/3! + u⁵/5! − …` with `n_terms = prec/2 + 10`.
//! `cos(u) = 1 − u²/2! + u⁴/4! − …` with `n_terms = prec/2 + 10`.
//!
//! These term counts ensure convergence because for `|u| ≤ 1` the ratio
//! `|u^(2k+1)| / (2k+1)!` decays faster than `(π/4)^(2k+1) / (2k+1)!`.
//! At 2k+1 ≈ prec/2 terms, this is well below `2^{-prec}`.

use oxinum_core::{OxiNumError, OxiNumResult};

use super::constants::pi;
use super::float::{BigFloat, RoundingMode};

// ---------------------------------------------------------------------------
// Taylor series helpers
// ---------------------------------------------------------------------------

/// Compute `sin(u)` via Taylor series for `|u| ≤ π/4`.
///
/// `sin(u) = u − u³/3! + u⁵/5! − …`
///
/// Iterative form: `term_0 = u`; `term_k = −term_{k−1} * u² / ((2k)(2k+1))`.
fn sin_taylor(u: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    if u.is_zero() {
        return Ok(BigFloat::zero(prec));
    }

    // u² at work precision.
    let u_sq = u.mul_ref_with_mode(u, mode).with_precision(prec, mode);

    // term = u; result = u.
    let mut term = u.clone().with_precision(prec, mode);
    let mut result = term.clone();

    // n_terms is a safe upper bound for convergence at |u| ≤ π/4.
    let n_terms: u64 = (prec as u64) / 2 + 10;

    for k in 1..=n_terms {
        // term *= u² / ((2k)(2k+1))
        // Denominator: (2k)(2k+1) fits in i64 for all practical k.
        let denom_val = (2 * k) * (2 * k + 1);
        // Guard against enormous k turning denom > i64::MAX (unreachable in
        // practice: prec ≤ 2^32 so n_terms ≤ 2^31+10, denom ≤ 4*(2^31)^2 > i64,
        // but also at that scale |u|^(2k+1)/(2k+1)! is exactly 0 in float).
        // Use saturating to avoid panic; if it saturates the term is negligible.
        let denom_i64 = denom_val.min(i64::MAX as u64) as i64;
        let denom_f = BigFloat::from_i64(denom_i64, prec, mode);
        // term = -(term * u_sq) / denom
        let numer = term
            .mul_ref_with_mode(&u_sq, mode)
            .with_precision(prec, mode);
        term = numer
            .div_ref_with_mode(&denom_f, mode)
            .map_err(|e| OxiNumError::Precision(format!("sin_taylor denom zero: {e}").into()))?;
        term = term.neg().with_precision(prec, mode);
        result = result
            .add_ref_with_mode(&term, mode)
            .with_precision(prec, mode);
    }

    Ok(result.with_precision(prec, mode))
}

/// Compute `cos(u)` via Taylor series for `|u| ≤ π/4`.
///
/// `cos(u) = 1 − u²/2! + u⁴/4! − …`
///
/// Iterative form: `term_0 = 1`; `term_k = −term_{k−1} * u² / ((2k−1)(2k))`.
fn cos_taylor(u: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    // u² at work precision.
    let u_sq = u.mul_ref_with_mode(u, mode).with_precision(prec, mode);

    // term = 1; result = 1.
    let one = BigFloat::from_i64(1, prec, mode);
    let mut term = one.clone();
    let mut result = one;

    let n_terms: u64 = (prec as u64) / 2 + 10;

    for k in 1..=n_terms {
        let denom_val = (2 * k - 1) * (2 * k);
        let denom_i64 = denom_val.min(i64::MAX as u64) as i64;
        let denom_f = BigFloat::from_i64(denom_i64, prec, mode);
        let numer = term
            .mul_ref_with_mode(&u_sq, mode)
            .with_precision(prec, mode);
        term = numer
            .div_ref_with_mode(&denom_f, mode)
            .map_err(|e| OxiNumError::Precision(format!("cos_taylor denom zero: {e}").into()))?;
        term = term.neg().with_precision(prec, mode);
        result = result
            .add_ref_with_mode(&term, mode)
            .with_precision(prec, mode);
    }

    Ok(result.with_precision(prec, mode))
}

// ---------------------------------------------------------------------------
// Argument reduction: extract integer quotient from BigFloat
// ---------------------------------------------------------------------------

/// Extract the integer part (floor toward zero) of a `BigFloat`.
///
/// Returns `None` if the value is too large to represent in `i64` (exponent
/// plus bit-length would overflow). Returns `0` for sub-unit values.
fn bigfloat_to_i64_round(x: &BigFloat) -> Option<i64> {
    // The value is (-1)^s * mantissa * 2^e.
    // The integer part is non-zero only when e + bit_length > 0 (i.e. the
    // value has magnitude >= 1).
    if x.is_zero() {
        return Some(0);
    }
    let e = x.exponent();
    let bits = x.mantissa().bit_length() as i64;
    // Effective position of the top bit in the integer: e + bits - 1.
    let top_pos = e.saturating_add(bits - 1);
    if top_pos < 0 {
        // |value| < 1, integer part is 0.
        return Some(0);
    }
    if top_pos >= 63 {
        // Too large to represent in i64.
        return None;
    }
    // The integer part has at most `top_pos + 1` bits which is < 63.
    // integer_part_biguint = mantissa >> (-e) when e < 0, or mantissa << e when e >= 0.
    // We operate in BigUint space so the shift is exact regardless of mantissa size.
    let int_biguint = if e >= 0 {
        // Left-shift: value = mantissa * 2^e. The integer part is the full value.
        // top_pos < 63 guarantees this fits in i64.
        x.mantissa().shl_bits(e as u64)
    } else {
        // Right-shift: integer part = mantissa >> (-e).
        let shift = (-e) as u64;
        x.mantissa().shr_bits(shift)
    };
    let int_mag = int_biguint.to_u64()?;
    // Apply sign.
    if x.signum() < 0 {
        // Negative: int_mag fits in i64 because top_pos < 63.
        Some(-(int_mag as i64))
    } else {
        Some(int_mag as i64)
    }
}

/// Compute `q = round(x / (π/2))` as an `i64` at work precision.
///
/// Returns `None` if `|q|` is too large to represent in `i64` (i.e. `|x| ≥
/// 2^62 * π/2 ≈ 7e18`). Callers should propagate a `Precision` error in that
/// case.
fn round_div_pi_over_2(
    x: &BigFloat,
    pi_over_2: &BigFloat,
    work_prec: u32,
    mode: RoundingMode,
) -> Option<i64> {
    let ratio = x
        .clone()
        .with_precision(work_prec, mode)
        .div_ref_with_mode(pi_over_2, mode)
        .ok()?;

    // Add ±0.5 and take the floor to implement rounding.
    let half = BigFloat::from_f64(0.5, work_prec).ok()?;
    let shifted = if ratio.signum() >= 0 {
        ratio.add_ref_with_mode(&half, mode)
    } else {
        ratio.sub_ref_with_mode(&half, mode)
    };
    // Extract integer part.
    bigfloat_to_i64_round(&shifted)
}

// ---------------------------------------------------------------------------
// Core sin/cos implementation
// ---------------------------------------------------------------------------

/// Compute `(sin(x), cos(x))` at `prec` bits via argument reduction + Taylor.
pub(crate) fn sincos_impl(
    x: &BigFloat,
    prec: u32,
    mode: RoundingMode,
) -> OxiNumResult<(BigFloat, BigFloat)> {
    // Guard bits: extra bits to absorb cancellation during argument reduction.
    // The reduction computes u = x − q*(π/2). For large |x| we need extra
    // precision to get the low bits of u right. We use |exponent| + 32 as
    // a conservative guard.
    let exp_guard = if x.exponent() > 0 {
        (x.exponent() as u32).min(256)
    } else {
        0u32
    };
    let work_prec = prec.saturating_add(exp_guard).saturating_add(32);

    // π/2 at work precision.
    let pi_val = pi(work_prec)?;
    let two = BigFloat::from_i64(2, work_prec, mode);
    let pi_over_2 = pi_val.div_ref_with_mode(&two, mode)?;

    // Compute q = round(x / (π/2)).
    let q = match round_div_pi_over_2(x, &pi_over_2, work_prec, mode) {
        Some(v) => v,
        None => {
            return Err(OxiNumError::Precision(
                "sin/cos: argument magnitude too large (|x| > 2^62 * π/2); \
                 use argument reduction before calling"
                    .into(),
            ));
        }
    };
    let quadrant = q.rem_euclid(4) as u32;

    // u = x − q * (π/2).
    let q_bf = BigFloat::from_i64(q, work_prec, mode);
    let u = x
        .clone()
        .with_precision(work_prec, mode)
        .sub_ref_with_mode(&q_bf.mul_ref_with_mode(&pi_over_2, mode), mode);

    // Taylor / binary-splitting series for sin(u) and cos(u) at work precision.
    // Above BS_THRESHOLD_BITS, the binary-splitting path replaces both Taylor
    // calls and is strictly faster at high precision.
    let (sin_u, cos_u) = if prec >= super::bs_transcendental::BS_THRESHOLD_BITS {
        super::bs_transcendental::sincos_bs(&u, work_prec, mode)?
    } else {
        let s = sin_taylor(&u, work_prec, mode)?;
        let c = cos_taylor(&u, work_prec, mode)?;
        (s, c)
    };

    // Apply quadrant table.
    let (sin_x, cos_x) = match quadrant {
        0 => (sin_u, cos_u),
        1 => (cos_u, sin_u.neg()),
        2 => (sin_u.neg(), cos_u.neg()),
        3 => (cos_u.neg(), sin_u),
        _ => unreachable!("rem_euclid(4) always in 0..=3"),
    };

    Ok((
        sin_x.with_precision(prec, mode),
        cos_x.with_precision(prec, mode),
    ))
}

// ---------------------------------------------------------------------------
// Public BigFloat methods
// ---------------------------------------------------------------------------

impl BigFloat {
    /// Return `sin(self)` at `prec` bits using the given rounding mode.
    ///
    /// Uses argument reduction mod π/2 and a Taylor series convergent for
    /// `|u| ≤ π/4`.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Precision`] if `|self|` is too large for the internal
    ///   argument-reduction step (approximately `|x| > 2^62 · π/2 ≈ 7.2·10^18`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let zero = BigFloat::zero(64);
    /// let s = zero.sin(64, RoundingMode::HalfEven).expect("sin(0)");
    /// assert!(s.is_zero());
    /// ```
    pub fn sin(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        // IEEE-754: sin(NaN) = NaN, sin(±Inf) = NaN
        if self.is_nan() || self.is_infinite() {
            return Ok(BigFloat::nan(prec));
        }
        if self.is_zero() {
            return Ok(BigFloat::zero(prec));
        }
        let (s, _c) = sincos_impl(self, prec, mode)?;
        Ok(s)
    }

    /// Return `cos(self)` at `prec` bits using the given rounding mode.
    ///
    /// # Errors
    ///
    /// Same as [`BigFloat::sin`].
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let zero = BigFloat::zero(64);
    /// let c = zero.cos(64, RoundingMode::HalfEven).expect("cos(0)");
    /// assert_eq!(c.to_f64(), 1.0);
    /// ```
    pub fn cos(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        // IEEE-754: cos(NaN) = NaN, cos(±Inf) = NaN
        if self.is_nan() || self.is_infinite() {
            return Ok(BigFloat::nan(prec));
        }
        let (_s, c) = sincos_impl(self, prec, mode)?;
        Ok(c)
    }

    /// Return `tan(self)` at `prec` bits using the given rounding mode.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] if `cos(self) = 0` (i.e. `self = π/2 + k·π`).
    /// - [`OxiNumError::Precision`] for very large `|self|`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let pi_over_4 = {
    ///     use oxinum_float::native::pi;
    ///     let p = pi(64).expect("pi");
    ///     let four = BigFloat::from_i64(4, 64, RoundingMode::HalfEven);
    ///     p.div_ref(&four).expect("div")
    /// };
    /// let t = pi_over_4.tan(64, RoundingMode::HalfEven).expect("tan(π/4)");
    /// assert!((t.to_f64() - 1.0).abs() < 1e-14);
    /// ```
    pub fn tan(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        // IEEE-754: tan(NaN) = NaN, tan(±Inf) = NaN
        if self.is_nan() || self.is_infinite() {
            return Ok(BigFloat::nan(prec));
        }
        let (s, c) = sincos_impl(self, prec, mode)?;
        if c.is_zero() {
            return Err(OxiNumError::Domain("tan undefined at π/2 + k·π".into()));
        }
        let result = s.div_ref_with_mode(&c, mode)?;
        Ok(result.with_precision(prec, mode))
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
    fn sin_zero_is_zero() {
        let z = mk(0, 64);
        let s = z.sin(64, RoundingMode::HalfEven).expect("sin(0)");
        assert!(s.is_zero());
    }

    #[test]
    fn cos_zero_is_one() {
        let z = mk(0, 64);
        let c = z.cos(64, RoundingMode::HalfEven).expect("cos(0)");
        assert!((c.to_f64() - 1.0).abs() < 1e-14, "cos(0) = {}", c.to_f64());
    }

    #[test]
    fn sin_pi_over_2_is_one() {
        let prec = 64u32;
        let p = pi(prec).expect("pi");
        let two = mk(2, prec);
        let pi_over_2 = p
            .div_ref_with_mode(&two, RoundingMode::HalfEven)
            .expect("pi/2");
        let s = pi_over_2
            .sin(prec, RoundingMode::HalfEven)
            .expect("sin(pi/2)");
        assert!(
            (s.to_f64() - 1.0).abs() < 1e-14,
            "sin(π/2) = {}",
            s.to_f64()
        );
    }

    #[test]
    fn pythagorean_identity_at_f64_angle() {
        let prec = 100u32;
        for x_f64 in [0.3f64, 1.0, 2.7, -1.5, 5.0, -2.9] {
            let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
            let s = x.sin(prec, RoundingMode::HalfEven).expect("sin");
            let c = x.cos(prec, RoundingMode::HalfEven).expect("cos");
            let sum = s
                .mul_ref_with_mode(&s, RoundingMode::HalfEven)
                .add_ref_with_mode(
                    &c.mul_ref_with_mode(&c, RoundingMode::HalfEven),
                    RoundingMode::HalfEven,
                );
            let err = (sum.to_f64() - 1.0).abs();
            assert!(
                err < 1e-25,
                "sin²+cos² error = {:.2e} for x = {}",
                err,
                x_f64
            );
        }
    }
}
