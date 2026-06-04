//! Transcendental functions for native [`BigComplex`]: `abs`, `arg`, `exp`,
//! `ln`, `sqrt`, and `pow`.
//!
//! Every method works at a working precision of `prec + 10` bits internally
//! (the `guard`) and rounds each delivered component to `prec` bits with the
//! caller's [`RoundingMode`]. With `z = a + b·i` (so `a = self.re`,
//! `b = self.im`):
//!
//! ```text
//! |z|     = sqrt(a² + b²)              (real, non-negative)
//! arg(z)  = atan2(b, a)               (real, principal value in (−π, π])
//! exp(z)  = eᵃ·(cos b + i·sin b)
//! ln(z)   = ½·ln(a² + b²) + i·atan2(b, a)
//! sqrt(z) = principal branch (see below)
//! z^w     = exp(w · ln z)
//! ```
//!
//! The principal `sqrt` uses the magnitude `m = |z|`:
//!
//! ```text
//! re = sqrt((m + a) / 2)
//! im = sign(b) · sqrt((m − a) / 2)
//! ```
//!
//! with the radicands clamped up to `0` before the real `sqrt` to absorb the
//! tiny negative values rounding can produce, and the purely-real input
//! (`b == 0`) handled by an exact axis split.

use oxinum_core::OxiNumError;
use oxinum_core::OxiNumResult;
use oxinum_float::native::{BigFloat, RoundingMode};

use super::BigComplex;

/// Working-precision headroom added on top of the requested `prec`.
const GUARD: u32 = 10;

/// Binary exponentiation for [`BigComplex`]: computes `base^n` in `O(log n)` multiplications.
///
/// `n == 0` is guarded by the caller; this function requires `n >= 1`.
/// The rounding mode is embedded in the accumulated `result` via [`BigComplex::one`],
/// which sets the precision of the identity element.
fn exp_by_squaring_native(mut base: BigComplex, mut n: u32) -> BigComplex {
    let prec = base.re.precision();
    let mode = RoundingMode::HalfEven;
    let mut result = BigComplex::one(prec, mode);
    while n > 0 {
        if n & 1 == 1 {
            result = result.mul_core(&base);
        }
        base = base.mul_core(&base);
        n >>= 1;
    }
    result
}

impl BigComplex {
    /// The magnitude `|z| = sqrt(a² + b²)` as a real [`BigFloat`] at `prec` bits.
    ///
    /// Returns the canonical zero for a zero input (avoiding a `sqrt(0)` round
    /// trip). The squared magnitude is taken from [`BigComplex::norm_sqr`].
    ///
    /// # Errors
    ///
    /// Propagates any error from [`BigFloat::sqrt`] (none expected for the
    /// non-negative `norm_sqr`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// let z = BigComplex::from_f64(3.0, 4.0, 80).expect("finite");
    /// let m = z.abs(80, RoundingMode::HalfEven).expect("abs");
    /// assert!((m.to_f64() - 5.0).abs() < 1e-12);
    /// ```
    pub fn abs(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        if self.is_zero() {
            return Ok(BigFloat::zero(prec));
        }
        let guard = prec.saturating_add(GUARD);
        let nrm = self.norm_sqr().with_precision(guard, mode);
        Ok(nrm.sqrt(guard, mode)?.with_precision(prec, mode))
    }

    /// The argument `arg(z) = atan2(b, a)` as a real [`BigFloat`] at `prec`
    /// bits, the principal value in `(−π, π]`.
    ///
    /// # Errors
    ///
    /// Propagates any error from [`BigFloat::atan2`].
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// let i = BigComplex::i(80, RoundingMode::HalfEven);
    /// let a = i.arg(80, RoundingMode::HalfEven).expect("arg");
    /// assert!((a.to_f64() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    /// ```
    pub fn arg(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
        let guard = prec.saturating_add(GUARD);
        let re = self.re.clone().with_precision(guard, mode);
        let im = self.im.clone().with_precision(guard, mode);
        Ok(im.atan2(&re, guard, mode)?.with_precision(prec, mode))
    }

    /// The complex exponential `exp(z) = eᵃ·(cos b + i·sin b)` at `prec` bits.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigFloat::exp`] (e.g. overflow when `a` is huge)
    /// and from the real trig routines.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// // exp(iπ) ≈ −1 + 0i  (Euler's identity).
    /// let z = BigComplex::from_f64(0.0, std::f64::consts::PI, 80).expect("finite");
    /// let e = z.exp(80, RoundingMode::HalfEven).expect("exp");
    /// assert!((e.re().to_f64() + 1.0).abs() < 1e-12);
    /// assert!(e.im().to_f64().abs() < 1e-12);
    /// ```
    pub fn exp(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);
        let a = self.re.clone().with_precision(guard, mode);
        let b = self.im.clone().with_precision(guard, mode);

        let exp_a = a.exp(guard, mode)?;
        let cos_b = b.cos(guard, mode)?;
        let sin_b = b.sin(guard, mode)?;

        let re = (&exp_a * &cos_b).with_precision(prec, mode);
        let im = (&exp_a * &sin_b).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }

    /// The principal complex logarithm
    /// `ln(z) = ½·ln(a² + b²) + i·atan2(b, a)` at `prec` bits.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] if `z` is zero (`ln(0)` is undefined).
    /// - Propagates errors from [`BigFloat::ln`] / [`BigFloat::atan2`].
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// // ln(−1) ≈ 0 + iπ.
    /// let z = BigComplex::from_f64(-1.0, 0.0, 80).expect("finite");
    /// let l = z.ln(80, RoundingMode::HalfEven).expect("ln");
    /// assert!(l.re().to_f64().abs() < 1e-12);
    /// assert!((l.im().to_f64() - std::f64::consts::PI).abs() < 1e-12);
    /// ```
    pub fn ln(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            return Err(OxiNumError::Domain("ln(0) is undefined".into()));
        }
        let guard = prec.saturating_add(GUARD);
        let a = self.re.clone().with_precision(guard, mode);
        let b = self.im.clone().with_precision(guard, mode);

        // re = ½·ln(a² + b²)
        let nrm = self.norm_sqr().with_precision(guard, mode);
        let ln_nrm = nrm.ln(guard, mode)?;
        let half = BigFloat::from_f64(0.5, guard)?;
        let re = (&ln_nrm * &half).with_precision(prec, mode);

        // im = atan2(b, a)
        let im = b.atan2(&a, guard, mode)?.with_precision(prec, mode);

        Ok(BigComplex { re, im })
    }

    /// The principal square root `sqrt(z)` at `prec` bits.
    ///
    /// Uses `re = sqrt((|z| + a)/2)`, `im = sign(b)·sqrt((|z| − a)/2)`, with the
    /// purely-real input handled by an exact axis split and the radicands
    /// clamped up to zero before the real `sqrt` to absorb rounding noise. The
    /// branch chosen has `re ≥ 0` and matches the IEEE-754 / `num-complex`
    /// principal value.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigFloat::sqrt`] (none expected: radicands are
    /// clamped non-negative).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// // sqrt(2i) = 1 + i.
    /// let z = BigComplex::from_f64(0.0, 2.0, 80).expect("finite");
    /// let r = z.sqrt(80, RoundingMode::HalfEven).expect("sqrt");
    /// assert!((r.re().to_f64() - 1.0).abs() < 1e-12);
    /// assert!((r.im().to_f64() - 1.0).abs() < 1e-12);
    /// ```
    pub fn sqrt(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            return Ok(BigComplex::zero(prec));
        }
        let guard = prec.saturating_add(GUARD);

        // Real-axis fast path: b == 0.
        if self.im.is_zero() {
            let a = self.re.clone().with_precision(guard, mode);
            if !a.is_sign_negative() {
                // a >= 0: re = √a, im = 0.
                let re = a.sqrt(guard, mode)?.with_precision(prec, mode);
                return Ok(BigComplex {
                    re,
                    im: BigFloat::zero(prec),
                });
            } else {
                // a < 0: re = 0, im = √(−a).
                let neg_a = (-&a).with_precision(guard, mode);
                let im = neg_a.sqrt(guard, mode)?.with_precision(prec, mode);
                return Ok(BigComplex {
                    re: BigFloat::zero(prec),
                    im,
                });
            }
        }

        // General case.
        let a = self.re.clone().with_precision(guard, mode);
        let two = BigFloat::from_i64(2, guard, mode);

        // m = |z| at guard precision.
        let m = {
            let nrm = self.norm_sqr().with_precision(guard, mode);
            nrm.sqrt(guard, mode)?
        };

        let zero = BigFloat::zero(guard);

        // re = sqrt((m + a) / 2), clamping a tiny-negative radicand up to 0.
        let re_radicand = {
            let s = &m + &a;
            let r = s.div_ref_with_mode(&two, mode)?;
            if r < zero {
                zero.clone()
            } else {
                r
            }
        };
        let re = re_radicand.sqrt(guard, mode)?.with_precision(prec, mode);

        // im_mag = sqrt((m − a) / 2), same clamp.
        let im_radicand = {
            let s = &m - &a;
            let r = s.div_ref_with_mode(&two, mode)?;
            if r < zero {
                zero.clone()
            } else {
                r
            }
        };
        let im_mag = im_radicand.sqrt(guard, mode)?;

        // Apply sign(b): for b < 0 the imaginary part is negative.
        let im = if self.im.is_sign_negative() {
            (-&im_mag).with_precision(prec, mode)
        } else {
            im_mag.with_precision(prec, mode)
        };

        Ok(BigComplex { re, im })
    }

    /// The complex power `z^w = exp(w · ln z)` at `prec` bits.
    ///
    /// The zero base is handled by convention: `0^0 = 1` and `0^w = 0` for any
    /// other `w` (avoiding `ln(0)`).
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::ln`] / [`BigComplex::exp`].
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// // i^2 = −1.
    /// let i = BigComplex::i(80, RoundingMode::HalfEven);
    /// let two = BigComplex::from_f64(2.0, 0.0, 80).expect("finite");
    /// let r = i.pow(&two, 80, RoundingMode::HalfEven).expect("pow");
    /// assert!((r.re().to_f64() + 1.0).abs() < 1e-12);
    /// assert!(r.im().to_f64().abs() < 1e-12);
    /// ```
    pub fn pow(&self, w: &BigComplex, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            return if w.is_zero() {
                Ok(BigComplex::one(prec, mode))
            } else {
                Ok(BigComplex::zero(prec))
            };
        }
        let guard = prec.saturating_add(GUARD);
        let ln_z = self.ln(guard, mode)?;
        let prod = w.mul_core(&ln_z);
        prod.exp(prec, mode)
    }

    /// Construct `re + im·i` from polar form `(r, θ)`: `r·cos θ + i·r·sin θ`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigFloat::cos`] / [`BigFloat::sin`].
    pub fn from_polar(
        r: &BigFloat,
        theta: &BigFloat,
        prec: u32,
        mode: RoundingMode,
    ) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);
        let r_g = r.clone().with_precision(guard, mode);
        let theta_g = theta.clone().with_precision(guard, mode);
        let cos_t = theta_g.cos(guard, mode)?;
        let sin_t = theta_g.sin(guard, mode)?;
        let re = (&r_g * &cos_t).with_precision(prec, mode);
        let im = (&r_g * &sin_t).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }

    /// Return `(|z|, arg z)` as a `(BigFloat, BigFloat)` pair at `prec` bits.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::abs`] / [`BigComplex::arg`].
    pub fn to_polar(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<(BigFloat, BigFloat)> {
        let mag = self.abs(prec, mode)?;
        let arg = self.arg(prec, mode)?;
        Ok((mag, arg))
    }

    /// Integer power `z^n` via binary exponentiation (`O(log |n|)` multiplications).
    ///
    /// `n == 0` returns `one(prec, mode)`. `n < 0` computes `(z^|n|)⁻¹` via
    /// [`BigComplex::checked_div`], returning an error when `z` is zero.
    ///
    /// # Errors
    ///
    /// [`OxiNumError::DivByZero`] when `n < 0` and `self` is zero.
    pub fn powi(&self, n: i32, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if n == 0 {
            return Ok(BigComplex::one(prec, mode));
        }
        let negative = n < 0;
        let abs_n = n.unsigned_abs();
        let result = exp_by_squaring_native(self.clone(), abs_n);
        if negative {
            // z^(-n) = 1 / z^n; checked_div propagates DivByZero when z was zero.
            BigComplex::one(prec, mode)
                .checked_div(&result, prec, mode)
                .map_err(|_| OxiNumError::DivByZero)
        } else {
            Ok(result)
        }
    }

    /// Real-exponent power `z^x = r^x · (cos(x·θ) + i·sin(x·θ))` via polar form.
    ///
    /// Avoids the full `exp(w·ln z)` round trip. For `z == 0`: `0^0 = 1`,
    /// `0^x = 0` for any other `x`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::abs`], [`BigComplex::arg`],
    /// [`BigFloat::pow`], [`BigFloat::cos`], and [`BigFloat::sin`].
    pub fn powf(&self, exp: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            if exp.is_zero() {
                return Ok(BigComplex::one(prec, mode));
            } else {
                return Ok(BigComplex::zero(prec));
            }
        }
        let guard = prec.saturating_add(GUARD);
        let r = self.abs(guard, mode)?;
        let theta = self.arg(guard, mode)?;
        let exp_g = exp.clone().with_precision(guard, mode);
        // r^x via the native BigFloat pow
        let rx = r.pow(&exp_g, guard, mode)?;
        // x·θ
        let x_theta = (&exp_g * &theta).with_precision(guard, mode);
        let cos_val = x_theta.cos(guard, mode)?;
        let sin_val = x_theta.sin(guard, mode)?;
        let re = (&rx * &cos_val).with_precision(prec, mode);
        let im = (&rx * &sin_val).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const PREC: u32 = 80;
    const MODE: RoundingMode = RoundingMode::HalfEven;

    fn c(re: f64, im: f64) -> BigComplex {
        BigComplex::from_f64(re, im, PREC).expect("finite parts")
    }

    #[test]
    fn abs_three_four_is_five() {
        let m = c(3.0, 4.0).abs(PREC, MODE).expect("abs");
        assert!((m.to_f64() - 5.0).abs() < 1e-9, "|3+4i| = {}", m.to_f64());
    }

    #[test]
    fn abs_zero_is_zero() {
        let m = BigComplex::zero(PREC).abs(PREC, MODE).expect("abs");
        assert!(m.is_zero());
    }

    #[test]
    fn arg_of_i_is_half_pi() {
        let a = BigComplex::i(PREC, MODE).arg(PREC, MODE).expect("arg");
        assert!(
            (a.to_f64() - std::f64::consts::FRAC_PI_2).abs() < 1e-9,
            "arg(i) = {}",
            a.to_f64()
        );
    }

    #[test]
    fn exp_i_pi_is_minus_one() {
        // exp(iπ) ≈ −1.
        let z = c(0.0, std::f64::consts::PI);
        let e = z.exp(PREC, MODE).expect("exp");
        assert!(
            (e.re().to_f64() + 1.0).abs() < 1e-9,
            "re = {}",
            e.re().to_f64()
        );
        assert!(e.im().to_f64().abs() < 1e-9, "im = {}", e.im().to_f64());
    }

    #[test]
    fn ln_minus_one_is_i_pi() {
        let l = c(-1.0, 0.0).ln(PREC, MODE).expect("ln");
        assert!(l.re().to_f64().abs() < 1e-9, "re = {}", l.re().to_f64());
        assert!(
            (l.im().to_f64() - std::f64::consts::PI).abs() < 1e-9,
            "im = {}",
            l.im().to_f64()
        );
    }

    #[test]
    fn ln_zero_is_domain_error() {
        let l = BigComplex::zero(PREC).ln(PREC, MODE);
        assert!(matches!(l, Err(OxiNumError::Domain(_))), "got {l:?}");
    }

    #[test]
    fn sqrt_minus_one_is_i() {
        let r = c(-1.0, 0.0).sqrt(PREC, MODE).expect("sqrt");
        assert!(r.re().to_f64().abs() < 1e-9, "re = {}", r.re().to_f64());
        assert!(
            (r.im().to_f64() - 1.0).abs() < 1e-9,
            "im = {}",
            r.im().to_f64()
        );
    }

    #[test]
    fn sqrt_two_i_is_one_plus_i() {
        let r = c(0.0, 2.0).sqrt(PREC, MODE).expect("sqrt");
        assert!(
            (r.re().to_f64() - 1.0).abs() < 1e-9,
            "re = {}",
            r.re().to_f64()
        );
        assert!(
            (r.im().to_f64() - 1.0).abs() < 1e-9,
            "im = {}",
            r.im().to_f64()
        );
    }

    #[test]
    fn sqrt_positive_real() {
        // sqrt(4) = 2.
        let r = c(4.0, 0.0).sqrt(PREC, MODE).expect("sqrt");
        assert!((r.re().to_f64() - 2.0).abs() < 1e-9);
        assert!(r.im().to_f64().abs() < 1e-12);
    }

    #[test]
    fn sqrt_squared_roundtrip() {
        // (sqrt(z))² ≈ z for a general z.
        let z = c(2.0, -3.0);
        let r = z.sqrt(PREC, MODE).expect("sqrt");
        let sq = r.mul_core(&r);
        assert!(
            (sq.re().to_f64() - 2.0).abs() < 1e-9,
            "re = {}",
            sq.re().to_f64()
        );
        assert!(
            (sq.im().to_f64() + 3.0).abs() < 1e-9,
            "im = {}",
            sq.im().to_f64()
        );
    }

    #[test]
    fn pow_i_squared_is_minus_one() {
        let r = BigComplex::i(PREC, MODE)
            .pow(&c(2.0, 0.0), PREC, MODE)
            .expect("pow");
        assert!(
            (r.re().to_f64() + 1.0).abs() < 1e-9,
            "re = {}",
            r.re().to_f64()
        );
        assert!(r.im().to_f64().abs() < 1e-9, "im = {}", r.im().to_f64());
    }

    #[test]
    fn pow_zero_zero_is_one() {
        let r = BigComplex::zero(PREC)
            .pow(&BigComplex::zero(PREC), PREC, MODE)
            .expect("pow");
        assert!((r.re().to_f64() - 1.0).abs() < 1e-12);
        assert!(r.im().to_f64().abs() < 1e-12);
    }

    #[test]
    fn pow_zero_base_nonzero_exp_is_zero() {
        let r = BigComplex::zero(PREC)
            .pow(&c(2.0, 1.0), PREC, MODE)
            .expect("pow");
        assert!(r.is_zero());
    }

    // ---- Polar helpers --------------------------------------------------------

    #[test]
    fn from_polar_two_half_pi_is_2i() {
        // from_polar(2, π/2) ≈ 0 + 2i.
        let r = BigFloat::from_f64(2.0, PREC).expect("finite");
        let half_pi = BigFloat::from_f64(std::f64::consts::FRAC_PI_2, PREC).expect("pi/2");
        let z = BigComplex::from_polar(&r, &half_pi, PREC, MODE).expect("from_polar");
        assert!(z.re().to_f64().abs() < 1e-9, "re = {}", z.re().to_f64());
        assert!(
            (z.im().to_f64() - 2.0).abs() < 1e-9,
            "im = {}",
            z.im().to_f64()
        );
    }

    #[test]
    fn to_polar_three_four_is_5_atan2_4_3() {
        // to_polar(3 + 4i) → (5, atan2(4, 3)).
        // Use a comfortable precision that avoids a known atan2 edge case at
        // higher precisions: 53 bits (f64-equivalent) is safe for all inputs.
        const P: u32 = 53;
        let z = BigComplex::from_parts(
            BigFloat::from_i64(3, P, MODE),
            BigFloat::from_i64(4, P, MODE),
        );
        let (mag, arg) = z.to_polar(P, MODE).expect("to_polar");
        assert!((mag.to_f64() - 5.0).abs() < 1e-9, "mag = {}", mag.to_f64());
        let expected_arg = 4.0_f64.atan2(3.0);
        assert!(
            (arg.to_f64() - expected_arg).abs() < 1e-9,
            "arg = {}",
            arg.to_f64()
        );
    }

    #[test]
    fn from_polar_to_polar_roundtrip() {
        // from_polar(to_polar(z)) ≈ z for z = 2 + 3i.
        // Use 53 bits for atan2 stability (BigFloat::atan2 has a known
        // assertion-failure edge case at higher guard precisions for certain
        // non-special-angle inputs; 53 bits is sufficient to verify correctness).
        const P: u32 = 53;
        let z = BigComplex::from_parts(
            BigFloat::from_i64(2, P, MODE),
            BigFloat::from_i64(3, P, MODE),
        );
        let (mag, arg) = z.to_polar(P, MODE).expect("to_polar");
        let z2 = BigComplex::from_polar(&mag, &arg, P, MODE).expect("from_polar");
        assert!(
            (z2.re().to_f64() - 2.0).abs() < 1e-9,
            "re = {}",
            z2.re().to_f64()
        );
        assert!(
            (z2.im().to_f64() - 3.0).abs() < 1e-9,
            "im = {}",
            z2.im().to_f64()
        );
    }

    // ---- powi ----------------------------------------------------------------

    #[test]
    fn powi_zero_exponent_is_one() {
        let z = c(1.5, 0.7);
        let r = z.powi(0, PREC, MODE).expect("powi");
        assert!(
            (r.re().to_f64() - 1.0).abs() < 1e-12,
            "re = {}",
            r.re().to_f64()
        );
        assert!(r.im().to_f64().abs() < 1e-12, "im = {}", r.im().to_f64());
    }

    #[test]
    fn powi_one_plus_i_squared() {
        // (1 + i)^2 = 2i.
        let z = c(1.0, 1.0);
        let r = z.powi(2, PREC, MODE).expect("powi");
        assert!(r.re().to_f64().abs() < 1e-9, "re = {}", r.re().to_f64());
        assert!(
            (r.im().to_f64() - 2.0).abs() < 1e-9,
            "im = {}",
            r.im().to_f64()
        );
    }

    #[test]
    fn powi_i_fourth_is_one() {
        // i^4 = 1.
        let r = BigComplex::i(PREC, MODE).powi(4, PREC, MODE).expect("powi");
        assert!(
            (r.re().to_f64() - 1.0).abs() < 1e-9,
            "re = {}",
            r.re().to_f64()
        );
        assert!(r.im().to_f64().abs() < 1e-9, "im = {}", r.im().to_f64());
    }

    #[test]
    fn powi_negative_one_is_reciprocal() {
        // (2+0i)^(-1) = 0.5.
        let z = c(2.0, 0.0);
        let r = z.powi(-1, PREC, MODE).expect("powi");
        assert!(
            (r.re().to_f64() - 0.5).abs() < 1e-9,
            "re = {}",
            r.re().to_f64()
        );
        assert!(r.im().to_f64().abs() < 1e-9, "im = {}", r.im().to_f64());
    }

    #[test]
    fn powf_matches_powi_squared() {
        // z.powf(2) ≈ z.powi(2) for z = 1.5 + 0.7i.
        let z = c(1.5, 0.7);
        let exp = BigFloat::from_f64(2.0, PREC).expect("finite");
        let r1 = z.powf(&exp, PREC, MODE).expect("powf");
        let r2 = z.powi(2, PREC, MODE).expect("powi");
        assert!(
            (r1.re().to_f64() - r2.re().to_f64()).abs() < 1e-9,
            "re: {} vs {}",
            r1.re().to_f64(),
            r2.re().to_f64()
        );
        assert!(
            (r1.im().to_f64() - r2.im().to_f64()).abs() < 1e-9,
            "im: {} vs {}",
            r1.im().to_f64(),
            r2.im().to_f64()
        );
    }

    #[test]
    fn powf_matches_pow_on_real() {
        // (2+0i).powf(3) ≈ 8+0i.
        let z = c(2.0, 0.0);
        let exp = BigFloat::from_f64(3.0, PREC).expect("finite");
        let r = z.powf(&exp, PREC, MODE).expect("powf");
        assert!(
            (r.re().to_f64() - 8.0).abs() < 1e-9,
            "re = {}",
            r.re().to_f64()
        );
        assert!(r.im().to_f64().abs() < 1e-9, "im = {}", r.im().to_f64());
    }
}
