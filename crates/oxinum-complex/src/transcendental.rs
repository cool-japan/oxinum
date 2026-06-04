//! Transcendental functions for [`crate::CBig`]: `abs`, `arg`, `exp`, `ln`,
//! `sqrt`, and `pow`.
//!
//! Every method works at an internal *guard* precision of `precision + 10`
//! significant decimal digits and delivers each component at the guard
//! precision returned by the underlying [`oxinum_float`] free functions. With
//! `z = a + b·i` (so `a = self.re()`, `b = self.im()`):
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
//! (`b == 0`) handled by an exact axis split (so `sqrt(-1) = +i`).
//!
//! This is the decimal-backed ([`DBig`]) mirror of the native binary
//! implementation in [`crate::native`]; the formulas and branch cuts match
//! exactly.

use core::str::FromStr;

use oxinum_float::{atan2, cos, exp, ln, pow, sin, sqrt};

use crate::{CBig, DBig, OxiNumError, OxiNumResult};

/// Working-precision headroom added on top of the requested `precision`.
const GUARD: usize = 10;

/// Parse a decimal literal into a [`DBig`], mapping any failure to
/// [`OxiNumError::Parse`].
fn make_dbig(s: &str) -> OxiNumResult<DBig> {
    DBig::from_str(s).map_err(|e| OxiNumError::Parse(format!("{e}").into()))
}

/// Binary exponentiation for [`CBig`]: computes `base^n` in `O(log n)` multiplications.
///
/// `n == 0` is guarded by the caller; this function requires `n >= 1`.
fn exp_by_squaring_cbig(mut base: CBig, mut n: u32) -> CBig {
    let mut result = CBig::one();
    while n > 0 {
        if n & 1 == 1 {
            result = &result * &base;
        }
        base = &base * &base;
        n >>= 1;
    }
    result
}

impl CBig {
    /// The magnitude `|z| = sqrt(a² + b²)` as a real [`DBig`] at `precision`
    /// significant digits.
    ///
    /// Returns an exact decimal zero for a zero input (avoiding a `sqrt(0)`
    /// round trip). The squared magnitude is taken from [`CBig::norm_sqr`].
    ///
    /// # Errors
    ///
    /// Propagates any error from [`oxinum_float::sqrt`] (none expected for the
    /// non-negative `norm_sqr`; an [`OxiNumError::Precision`] is returned if
    /// `precision == 0`).
    pub fn abs(&self, precision: usize) -> OxiNumResult<DBig> {
        if self.is_zero() {
            return make_dbig("0.0");
        }
        sqrt(&self.norm_sqr(), precision)
    }

    /// The argument `arg(z) = atan2(b, a)` as a real [`DBig`] at `precision`
    /// significant digits, the principal value in `(−π, π]`.
    ///
    /// # Errors
    ///
    /// Propagates any error from [`oxinum_float::atan2`].
    pub fn arg(&self, precision: usize) -> OxiNumResult<DBig> {
        atan2(&self.im, &self.re, precision)
    }

    /// The complex exponential `exp(z) = eᵃ·(cos b + i·sin b)` at `precision`
    /// significant digits.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`oxinum_float::exp`], [`oxinum_float::cos`], and
    /// [`oxinum_float::sin`].
    pub fn exp(&self, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision + GUARD;
        let ea = exp(&self.re, guard)?;
        let cosb = cos(&self.im, guard)?;
        let sinb = sin(&self.im, guard)?;
        Ok(CBig::from_parts(&ea * &cosb, &ea * &sinb))
    }

    /// The principal complex logarithm
    /// `ln(z) = ½·ln(a² + b²) + i·atan2(b, a)` at `precision` significant
    /// digits.
    ///
    /// Using `½·ln(norm_sqr)` for the real part avoids an extra `sqrt`.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] if `z` is zero (`ln(0)` is undefined).
    /// - Propagates errors from [`oxinum_float::ln`] / [`oxinum_float::atan2`].
    pub fn ln(&self, precision: usize) -> OxiNumResult<CBig> {
        if self.is_zero() {
            return Err(OxiNumError::Domain("ln(0) is undefined".into()));
        }
        let guard = precision + GUARD;
        let ns = self.norm_sqr();
        let re = &make_dbig("0.5")? * &ln(&ns, guard)?;
        let im = atan2(&self.im, &self.re, guard)?;
        Ok(CBig::from_parts(re, im))
    }

    /// The principal square root `sqrt(z)` at `precision` significant digits.
    ///
    /// Uses `re = sqrt((|z| + a)/2)`, `im = sign(b)·sqrt((|z| − a)/2)`, with the
    /// purely-real input handled by an exact axis split and the radicands
    /// clamped up to zero before the real `sqrt` to absorb rounding noise. The
    /// branch chosen has `re ≥ 0` and matches the IEEE-754 / `num-complex`
    /// principal value (so `sqrt(-1) = +i`).
    ///
    /// # Errors
    ///
    /// Propagates errors from [`oxinum_float::sqrt`] (none expected: radicands
    /// are clamped non-negative).
    pub fn sqrt(&self, precision: usize) -> OxiNumResult<CBig> {
        let zero = DBig::from(0u32);

        if self.is_zero() {
            return Ok(CBig::zero());
        }

        // Real-axis fast path: b == 0.
        if self.im == zero {
            if self.re >= zero {
                // a >= 0: re = √a, im = 0.
                return Ok(CBig::from_parts(sqrt(&self.re, precision)?, zero));
            }
            // a < 0: re = 0, im = √(−a)  (so sqrt(-1) = +i).
            let neg_a = -&self.re;
            return Ok(CBig::from_parts(zero, sqrt(&neg_a, precision)?));
        }

        // General case.
        let guard = precision + GUARD;
        let m = sqrt(&self.norm_sqr(), guard)?; // m = |z|
        let two = make_dbig("2.0")?;

        // re = sqrt((m + a) / 2), clamping a tiny-negative radicand up to 0.
        let mut r_re = &(&m + &self.re) / &two;
        if r_re < zero {
            r_re = DBig::from(0u32);
        }

        // im_mag = sqrt((m − a) / 2), same clamp.
        let mut r_im = &(&m - &self.re) / &two;
        if r_im < zero {
            r_im = DBig::from(0u32);
        }

        let re = sqrt(&r_re, precision)?;
        let im_mag = sqrt(&r_im, precision)?;

        // Apply sign(b): for b < 0 the imaginary part is negative.
        let im = if self.im < zero { -im_mag } else { im_mag };

        Ok(CBig::from_parts(re, im))
    }

    /// The complex power `z^w = exp(w · ln z)` at `precision` significant
    /// digits.
    ///
    /// The zero base is handled by convention: `0^0 = 1` and `0^w = 0` for any
    /// other `w` (avoiding `ln(0)`).
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::ln`] / [`CBig::exp`].
    pub fn pow(&self, w: &CBig, precision: usize) -> OxiNumResult<CBig> {
        if self.is_zero() {
            return if w.is_zero() {
                Ok(CBig::one())
            } else {
                Ok(CBig::zero())
            };
        }
        let guard = precision + GUARD;
        let lz = self.ln(guard)?;
        let prod = w * &lz;
        prod.exp(precision)
    }

    /// Construct `re + im·i` from polar form `(r, θ)`: `r·cos θ + i·r·sin θ`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`oxinum_float::cos`] / [`oxinum_float::sin`].
    pub fn from_polar(r: &DBig, theta: &DBig, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision.saturating_add(GUARD);
        let cos_t = cos(theta, guard)?;
        let sin_t = sin(theta, guard)?;
        Ok(CBig::from_parts(r * &cos_t, r * &sin_t))
    }

    /// Return `(|z|, arg z)` as a `(DBig, DBig)` pair at `precision` digits.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::abs`] / [`CBig::arg`].
    pub fn to_polar(&self, precision: usize) -> OxiNumResult<(DBig, DBig)> {
        let mag = self.abs(precision)?;
        let arg = self.arg(precision)?;
        Ok((mag, arg))
    }

    /// Integer power `z^n` via binary exponentiation (`O(log |n|)` multiplications).
    ///
    /// `n == 0` returns `one()`. `n < 0` computes `(z^|n|)⁻¹` via
    /// [`CBig::checked_div`], returning [`OxiNumError::DivByZero`] when `z` is zero.
    ///
    /// # Errors
    ///
    /// [`OxiNumError::DivByZero`] when `n < 0` and `self` is zero.
    pub fn powi(&self, n: i32, _precision: usize) -> OxiNumResult<CBig> {
        if n == 0 {
            return Ok(CBig::one());
        }
        let negative = n < 0;
        let abs_n = n.unsigned_abs();
        let result = exp_by_squaring_cbig(self.clone(), abs_n);
        if negative {
            CBig::one().checked_div(&result)
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
    /// Propagates errors from [`CBig::abs`], [`CBig::arg`], [`oxinum_float::pow`],
    /// [`oxinum_float::cos`], and [`oxinum_float::sin`].
    pub fn powf(&self, exp: &DBig, precision: usize) -> OxiNumResult<CBig> {
        let zero = DBig::from(0u32);
        if self.is_zero() {
            return if *exp == zero {
                Ok(CBig::one())
            } else {
                Ok(CBig::zero())
            };
        }
        let guard = precision.saturating_add(GUARD);
        let r = self.abs(guard)?;
        let theta = self.arg(guard)?;
        // r^x via the real `pow` free function
        let rx = pow(&r, exp, guard)?;
        // x·θ
        let x_theta = exp * &theta;
        let re = &rx * &cos(&x_theta, guard)?;
        let im = &rx * &sin(&x_theta, guard)?;
        Ok(CBig::from_parts(re, im))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oxinum_float::compute_pi;

    /// Build π at the given decimal precision.
    fn pi(precision: usize) -> DBig {
        compute_pi(precision)
    }

    #[test]
    fn exp_i_pi_is_minus_one() {
        // exp(iπ) ≈ −1 + 0i  (Euler's identity).
        let z = CBig::from_parts(DBig::from(0u32), pi(50));
        let r = z.exp(40).expect("exp");
        let (re, im) = r.to_f64_parts();
        assert!((re + 1.0).abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn ln_minus_one_is_i_pi() {
        let z = CBig::from_real(make_dbig("-1.0").expect("literal"));
        let r = z.ln(40).expect("ln");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < 1e-12, "re = {re}");
        assert!((im - std::f64::consts::PI).abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn ln_zero_is_domain_error() {
        let r = CBig::zero().ln(40);
        assert!(matches!(r, Err(OxiNumError::Domain(_))), "got {r:?}");
    }

    #[test]
    fn sqrt_minus_one_is_i() {
        let z = CBig::from_real(make_dbig("-1.0").expect("literal"));
        let r = z.sqrt(40).expect("sqrt");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < 1e-12, "re = {re}");
        assert!((im - 1.0).abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn sqrt_two_i_is_one_plus_i() {
        // sqrt(2i) = 1 + i.
        let z = CBig::from_parts(DBig::from(0u32), DBig::from(2u32));
        let r = z.sqrt(40).expect("sqrt");
        let (re, im) = r.to_f64_parts();
        assert!((re - 1.0).abs() < 1e-12, "re = {re}");
        assert!((im - 1.0).abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn sqrt_positive_real() {
        // sqrt(4) = 2.
        let z = CBig::from_real(DBig::from(4u32));
        let r = z.sqrt(40).expect("sqrt");
        let (re, im) = r.to_f64_parts();
        assert!((re - 2.0).abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn abs_three_four_is_five() {
        let z = CBig::from_parts(DBig::from(3u32), DBig::from(4u32));
        let m = z.abs(40).expect("abs");
        assert!(m.to_string().starts_with('5'), "|3+4i| = {m}");
    }

    #[test]
    fn abs_zero_is_zero() {
        let m = CBig::zero().abs(40).expect("abs");
        assert_eq!(m, DBig::from(0u32));
    }

    #[test]
    fn arg_of_i_is_half_pi() {
        let a = CBig::i().arg(40).expect("arg");
        let v = a.to_f64();
        assert!(
            (v.value() - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
            "arg(i) = {a}"
        );
    }

    #[test]
    fn pow_i_squared_is_minus_one() {
        // i² = −1.
        let r = CBig::i()
            .pow(&CBig::from_real(DBig::from(2u32)), 40)
            .expect("pow");
        let (re, im) = r.to_f64_parts();
        assert!((re + 1.0).abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn pow_to_one_is_identity() {
        // z^1 ≈ z on the real axis (arg = 0), where the round trip through
        // ln/exp is exact and independent of the underlying `atan2`.
        let z = CBig::from_real(make_dbig("2.0").expect("literal"));
        let r = z.pow(&CBig::one(), 40).expect("pow");
        let (re, im) = r.to_f64_parts();
        assert!((re - 2.0).abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn pow_zero_zero_is_one() {
        let r = CBig::zero().pow(&CBig::zero(), 40).expect("pow");
        assert!(r == CBig::one());
    }

    #[test]
    fn pow_zero_base_nonzero_exp_is_zero() {
        let r = CBig::zero()
            .pow(&CBig::from_parts(DBig::from(2u32), DBig::from(1u32)), 40)
            .expect("pow");
        assert!(r.is_zero());
    }

    // ---- Polar helpers --------------------------------------------------------

    #[test]
    fn from_polar_two_half_pi_is_2i() {
        // from_polar(2, π/2) ≈ 0 + 2i.
        let r = DBig::from(2u32);
        let theta = &pi(50) / &make_dbig("2.0").expect("2.0");
        let z = CBig::from_polar(&r, &theta, 40).expect("from_polar");
        let (re, im) = z.to_f64_parts();
        assert!(re.abs() < 1e-9, "re = {re}");
        assert!((im - 2.0).abs() < 1e-9, "im = {im}");
    }

    #[test]
    fn to_polar_three_four_is_5_atan2_4_3() {
        // to_polar(3 + 4i) → (5, atan2(4, 3)).
        // Use from_f64 so parts carry bounded precision (avoids DBig::from(n)
        // single-digit truncation in norm_sqr).
        let z = CBig::from_f64(3.0, 4.0).expect("finite");
        let (mag, arg) = z.to_polar(40).expect("to_polar");
        assert!((mag.to_f64().value() - 5.0).abs() < 1e-9, "mag = {mag}");
        let expected_arg = (4.0_f64).atan2(3.0);
        assert!(
            (arg.to_f64().value() - expected_arg).abs() < 1e-9,
            "arg = {arg}"
        );
    }

    #[test]
    fn from_polar_to_polar_roundtrip() {
        // from_polar(to_polar(z)) ≈ z for z = 2 + 3i.
        // Use from_f64 so parts carry bounded precision (avoids DBig::from(n)
        // single-digit truncation in norm_sqr).
        let z = CBig::from_f64(2.0, 3.0).expect("finite");
        let (mag, arg) = z.to_polar(50).expect("to_polar");
        let z2 = CBig::from_polar(&mag, &arg, 40).expect("from_polar");
        let (re, im) = z2.to_f64_parts();
        assert!((re - 2.0).abs() < 1e-9, "re = {re}");
        assert!((im - 3.0).abs() < 1e-9, "im = {im}");
    }

    // ---- powi ----------------------------------------------------------------

    #[test]
    fn powi_zero_exponent_is_one() {
        let z = CBig::from_f64(1.5, 0.7).expect("finite");
        let r = z.powi(0, 40).expect("powi");
        assert!(r == CBig::one());
    }

    #[test]
    fn powi_one_plus_i_squared() {
        // (1 + i)^2 = 2i.
        let z = CBig::from_f64(1.0, 1.0).expect("finite");
        let r = z.powi(2, 40).expect("powi");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < 1e-9, "re = {re}");
        assert!((im - 2.0).abs() < 1e-9, "im = {im}");
    }

    #[test]
    fn powi_i_fourth_is_one() {
        // i^4 = 1.
        let r = CBig::i().powi(4, 40).expect("powi");
        let (re, im) = r.to_f64_parts();
        assert!((re - 1.0).abs() < 1e-9, "re = {re}");
        assert!(im.abs() < 1e-9, "im = {im}");
    }

    #[test]
    fn powi_negative_one_is_reciprocal() {
        // (2+0i)^(-1) = 0.5.
        let z = CBig::from_real(DBig::from(2u32));
        let r = z.powi(-1, 40).expect("powi");
        let (re, im) = r.to_f64_parts();
        assert!((re - 0.5).abs() < 1e-9, "re = {re}");
        assert!(im.abs() < 1e-9, "im = {im}");
    }

    #[test]
    fn powf_matches_powi_squared() {
        // z.powf(2) ≈ z.powi(2) for z = 1.5 + 0.7i.
        let z = CBig::from_f64(1.5, 0.7).expect("finite");
        let exp = DBig::from(2u32);
        let r1 = z.powf(&exp, 40).expect("powf");
        let r2 = z.powi(2, 40).expect("powi");
        let (re1, im1) = r1.to_f64_parts();
        let (re2, im2) = r2.to_f64_parts();
        assert!((re1 - re2).abs() < 1e-9, "re: {re1} vs {re2}");
        assert!((im1 - im2).abs() < 1e-9, "im: {im1} vs {im2}");
    }

    #[test]
    fn powf_matches_pow_on_real() {
        // (2+0i).powf(3) ≈ 8+0i.
        let z = CBig::from_real(DBig::from(2u32));
        let exp = DBig::from(3u32);
        let r = z.powf(&exp, 40).expect("powf");
        let (re, im) = r.to_f64_parts();
        assert!((re - 8.0).abs() < 1e-9, "re = {re}");
        assert!(im.abs() < 1e-9, "im = {im}");
    }
}
