//! Inverse trigonometric and hyperbolic functions for [`crate::CBig`].
//!
//! Implements the six principal-branch inverse functions using closed forms
//! derived from existing `ln`/`sqrt`/`CBig` operations. All forms follow DLMF
//! principal branch conventions (consistent with C99 Annex G and `num-complex`),
//! so branch cuts are placed on the real axis:
//!
//! ```text
//! asin z  = −i · ln(i·z + √(1 − z²))        cut (−∞,−1] ∪ [1,+∞)
//! acos z  = −i · ln(z + i·√(1 − z²))         cut (−∞,−1] ∪ [1,+∞)
//! atan z  = (i/2)·[ln(1 − i·z) − ln(1 + i·z)]  cut (−i∞,−i] ∪ [i,+i∞)
//! asinh z = ln(z + √(z² + 1))                cut [i,+i∞) ∪ (−i∞,−i]
//! acosh z = ln(z + √(z−1)·√(z+1))            cut (−∞,1]
//! atanh z = ½·[ln(1 + z) − ln(1 − z)]        cut (−∞,−1] ∪ [1,+∞)
//! ```
//!
//! For `acosh` we deliberately factor `√(z²−1)` as `√(z−1)·√(z+1)` rather
//! than taking a single square root, which places the branch cut consistently
//! on `(−∞, 1]` (matching `num-complex` / C99).
//!
//! Every method works at an internal guard precision of `precision + GUARD` and
//! delegates all heavy lifting to the existing transcendental primitives.

use core::str::FromStr;

use crate::{CBig, DBig, OxiNumError, OxiNumResult};

/// Working-precision headroom added on top of the requested `precision`.
const GUARD: usize = 10;

/// Parse a decimal literal into a [`DBig`], mapping any failure to
/// [`OxiNumError::Parse`].
fn make_dbig(s: &str) -> OxiNumResult<DBig> {
    DBig::from_str(s).map_err(|e| OxiNumError::Parse(format!("{e}").into()))
}

/// Multiply `z` by `i`: `i·(a+bi) = −b + a·i`.
#[inline]
fn mul_i(z: CBig) -> CBig {
    CBig::from_parts(-z.im, z.re)
}

/// Multiply `z` by `−i`: `−i·(a+bi) = b − a·i`.
#[inline]
fn mul_neg_i(z: CBig) -> CBig {
    CBig::from_parts(z.im, -z.re)
}

impl CBig {
    /// The principal value of `arcsin z` at `precision` significant digits.
    ///
    /// Uses the identity `asin z = −i · ln(i·z + √(1 − z²))`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::sqrt`] / [`CBig::ln`].
    pub fn asin(&self, precision: usize) -> OxiNumResult<CBig> {
        if self.is_zero() {
            return Ok(CBig::zero());
        }
        let guard = precision.saturating_add(GUARD);

        // i·z
        let iz = mul_i(self.clone());
        // z²
        let z_sq = self * self;
        // 1 − z²
        let one_minus_z2 = &CBig::one() - &z_sq;
        // √(1 − z²)
        let sqrt_val = one_minus_z2.sqrt(guard)?;
        // i·z + √(1 − z²)
        let arg = &iz + &sqrt_val;
        // ln(...)
        let ln_val = arg.ln(guard)?;
        // −i · ln(...)
        Ok(mul_neg_i(ln_val))
    }

    /// The principal value of `arccos z` at `precision` significant digits.
    ///
    /// Uses the identity `acos z = −i · ln(z + i·√(1 − z²))`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::sqrt`] / [`CBig::ln`].
    pub fn acos(&self, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision.saturating_add(GUARD);

        // z²
        let z_sq = self * self;
        // 1 − z²
        let one_minus_z2 = &CBig::one() - &z_sq;
        // √(1 − z²)
        let sqrt_val = one_minus_z2.sqrt(guard)?;
        // i·√(1 − z²)
        let i_sqrt = mul_i(sqrt_val);
        // z + i·√(1 − z²)
        let arg = self + &i_sqrt;
        // ln(...)
        let ln_val = arg.ln(guard)?;
        // −i · ln(...)
        Ok(mul_neg_i(ln_val))
    }

    /// The principal value of `arctan z` at `precision` significant digits.
    ///
    /// Uses the identity `atan z = (i/2)·[ln(1 − i·z) − ln(1 + i·z)]`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::ln`].
    pub fn atan(&self, precision: usize) -> OxiNumResult<CBig> {
        if self.is_zero() {
            return Ok(CBig::zero());
        }
        let guard = precision.saturating_add(GUARD);
        let half = make_dbig("0.5")?;

        // i·z
        let iz = mul_i(self.clone());
        let one = CBig::one();
        // ln(1 − i·z)
        let ln_minus = (&one - &iz).ln(guard)?;
        // ln(1 + i·z)
        let ln_plus = (&one + &iz).ln(guard)?;
        // diff = ln(1 − i·z) − ln(1 + i·z)
        let diff = &ln_minus - &ln_plus;
        // multiply by i/2: first multiply by i, then halve each component
        let i_diff = mul_i(diff);
        let re = &i_diff.re * &half;
        let im = &i_diff.im * &half;
        Ok(CBig::from_parts(re, im))
    }

    /// The principal value of `arcsinh z` at `precision` significant digits.
    ///
    /// Uses the identity `asinh z = ln(z + √(z² + 1))`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::sqrt`] / [`CBig::ln`].
    pub fn asinh(&self, precision: usize) -> OxiNumResult<CBig> {
        if self.is_zero() {
            return Ok(CBig::zero());
        }
        let guard = precision.saturating_add(GUARD);

        // z² + 1
        let z_sq_plus_one = &(self * self) + &CBig::one();
        // √(z² + 1)
        let sqrt_val = z_sq_plus_one.sqrt(guard)?;
        // z + √(z² + 1)
        let arg = self + &sqrt_val;
        arg.ln(guard)
    }

    /// The principal value of `arccosh z` at `precision` significant digits.
    ///
    /// Uses `acosh z = ln(z + √(z−1)·√(z+1))` (factored, not `√(z²−1)`) to
    /// place the branch cut on `(−∞, 1]`, consistent with C99 / `num-complex`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::sqrt`] / [`CBig::ln`].
    pub fn acosh(&self, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision.saturating_add(GUARD);

        let one = CBig::one();
        // √(z − 1)
        let sq1 = (self - &one).sqrt(guard)?;
        // √(z + 1)
        let sq2 = (self + &one).sqrt(guard)?;
        // z + √(z−1)·√(z+1)
        let product = &sq1 * &sq2;
        let arg = self + &product;
        arg.ln(guard)
    }

    /// The principal value of `arctanh z` at `precision` significant digits.
    ///
    /// Uses the identity `atanh z = ½·[ln(1 + z) − ln(1 − z)]`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`CBig::ln`].
    pub fn atanh(&self, precision: usize) -> OxiNumResult<CBig> {
        if self.is_zero() {
            return Ok(CBig::zero());
        }
        let guard = precision.saturating_add(GUARD);
        let half = make_dbig("0.5")?;

        let one = CBig::one();
        // ln(1 + z)
        let ln_plus = (&one + self).ln(guard)?;
        // ln(1 − z)
        let ln_minus = (&one - self).ln(guard)?;
        // diff = ln(1+z) − ln(1−z)
        let diff = &ln_plus - &ln_minus;
        // ½ · diff
        let re = &diff.re * &half;
        let im = &diff.im * &half;
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

    const PREC: usize = 40;
    const TOL: f64 = 1e-9;

    fn c(re: f64, im: f64) -> CBig {
        CBig::from_f64(re, im).expect("finite parts")
    }

    fn pi() -> DBig {
        compute_pi(PREC + 10)
    }

    // ---- Zero fast-paths -------------------------------------------------------

    #[test]
    fn asin_zero_is_zero() {
        let r = CBig::zero().asin(PREC).expect("asin");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < TOL, "re = {re}");
        assert!(im.abs() < TOL, "im = {im}");
    }

    #[test]
    fn atan_zero_is_zero() {
        let r = CBig::zero().atan(PREC).expect("atan");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < TOL, "re = {re}");
        assert!(im.abs() < TOL, "im = {im}");
    }

    #[test]
    fn asinh_zero_is_zero() {
        let r = CBig::zero().asinh(PREC).expect("asinh");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < TOL, "re = {re}");
        assert!(im.abs() < TOL, "im = {im}");
    }

    #[test]
    fn atanh_zero_is_zero() {
        let r = CBig::zero().atanh(PREC).expect("atanh");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < TOL, "re = {re}");
        assert!(im.abs() < TOL, "im = {im}");
    }

    // ---- Known-value checks ---------------------------------------------------

    #[test]
    fn asin_one_is_half_pi() {
        // asin(1) = π/2.
        let r = CBig::from_real(DBig::from(1u32)).asin(PREC).expect("asin");
        let (re, im) = r.to_f64_parts();
        let half_pi = pi().to_f64().value() / 2.0;
        assert!(
            (re - half_pi).abs() < TOL,
            "re = {re}, expected π/2 ≈ {half_pi}"
        );
        assert!(im.abs() < TOL, "im = {im}");
    }

    #[test]
    fn atan_one_is_quarter_pi() {
        // atan(1) = π/4.
        let r = CBig::from_real(DBig::from(1u32)).atan(PREC).expect("atan");
        let (re, im) = r.to_f64_parts();
        let quarter_pi = pi().to_f64().value() / 4.0;
        assert!(
            (re - quarter_pi).abs() < TOL,
            "re = {re}, expected π/4 ≈ {quarter_pi}"
        );
        assert!(im.abs() < TOL, "im = {im}");
    }

    #[test]
    fn acosh_one_is_zero() {
        // acosh(1) = 0.
        let r = CBig::from_real(DBig::from(1u32))
            .acosh(PREC)
            .expect("acosh");
        let (re, im) = r.to_f64_parts();
        assert!(re.abs() < TOL, "re = {re}");
        assert!(im.abs() < TOL, "im = {im}");
    }

    // ---- Round-trip identities ------------------------------------------------

    #[test]
    fn sin_asin_roundtrip() {
        // sin(asin(0.3+0.4i)) ≈ 0.3+0.4i.
        let z = c(0.3, 0.4);
        let asin_z = z.asin(PREC).expect("asin");
        let r = asin_z.sin(PREC).expect("sin");
        let (re, im) = r.to_f64_parts();
        assert!((re - 0.3).abs() < TOL, "re = {re}");
        assert!((im - 0.4).abs() < TOL, "im = {im}");
    }

    #[test]
    fn cos_acos_roundtrip() {
        // cos(acos(0.3+0.4i)) ≈ 0.3+0.4i.
        let z = c(0.3, 0.4);
        let acos_z = z.acos(PREC).expect("acos");
        let r = acos_z.cos(PREC).expect("cos");
        let (re, im) = r.to_f64_parts();
        assert!((re - 0.3).abs() < TOL, "re = {re}");
        assert!((im - 0.4).abs() < TOL, "im = {im}");
    }

    #[test]
    fn tanh_atanh_roundtrip() {
        // tanh(atanh(0.2+0.1i)) ≈ 0.2+0.1i.
        let z = c(0.2, 0.1);
        let atanh_z = z.atanh(PREC).expect("atanh");
        let r = atanh_z.tanh(PREC).expect("tanh");
        let (re, im) = r.to_f64_parts();
        assert!((re - 0.2).abs() < TOL, "re = {re}");
        assert!((im - 0.1).abs() < TOL, "im = {im}");
    }

    #[test]
    fn sinh_asinh_roundtrip() {
        // sinh(asinh(0.3+0.4i)) ≈ 0.3+0.4i.
        let z = c(0.3, 0.4);
        let asinh_z = z.asinh(PREC).expect("asinh");
        let r = asinh_z.sinh(PREC).expect("sinh");
        let (re, im) = r.to_f64_parts();
        assert!((re - 0.3).abs() < TOL, "re = {re}");
        assert!((im - 0.4).abs() < TOL, "im = {im}");
    }
}
