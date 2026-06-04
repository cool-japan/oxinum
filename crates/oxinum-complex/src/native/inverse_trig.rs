//! Inverse trigonometric and hyperbolic functions for native [`BigComplex`].
//!
//! Mirrors `src/inverse_trig.rs` exactly in formula choice; see that file for
//! the DLMF / C99 principal-branch derivations. Here each method takes an
//! explicit `(prec: u32, mode: RoundingMode)` pair following the `BigComplex`
//! API convention.
//!
//! All intermediate computations run at a guard precision of `prec + GUARD`
//! bits; the final result retains whatever precision the chain of
//! `ln`/`sqrt`/`mul_i` operations delivered (which is at least `prec` bits).

use oxinum_core::OxiNumResult;
use oxinum_float::native::{BigFloat, RoundingMode};

use super::BigComplex;

/// Working-precision headroom added on top of the requested `prec`.
const GUARD: u32 = 10;

/// Multiply `z` by `i`: `i·(a+bi) = −b + a·i`.
#[inline]
fn mul_i(z: BigComplex) -> BigComplex {
    BigComplex {
        re: -&z.im,
        im: z.re,
    }
}

/// Multiply `z` by `−i`: `−i·(a+bi) = b − a·i`.
#[inline]
fn mul_neg_i(z: BigComplex) -> BigComplex {
    BigComplex {
        re: z.im,
        im: -&z.re,
    }
}

impl BigComplex {
    /// The principal value of `arcsin z` at `prec` bits.
    ///
    /// Uses the identity `asin z = −i · ln(i·z + √(1 − z²))`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::sqrt`] / [`BigComplex::ln`].
    pub fn asin(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            return Ok(BigComplex::zero(prec));
        }
        let guard = prec.saturating_add(GUARD);

        // i·z
        let iz = mul_i(self.clone());
        // z²
        let z_sq = self.mul_core(self);
        // 1 − z²
        let one_c = BigComplex::one(guard, mode);
        let one_minus_z2 = &one_c - &z_sq;
        // √(1 − z²)
        let sqrt_val = one_minus_z2.sqrt(guard, mode)?;
        // i·z + √(1 − z²)
        let arg = &iz + &sqrt_val;
        // ln(...)
        let ln_val = arg.ln(guard, mode)?;
        // −i · ln(...)
        Ok(mul_neg_i(ln_val))
    }

    /// The principal value of `arccos z` at `prec` bits.
    ///
    /// Uses the identity `acos z = −i · ln(z + i·√(1 − z²))`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::sqrt`] / [`BigComplex::ln`].
    pub fn acos(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);

        // z²
        let z_sq = self.mul_core(self);
        // 1 − z²
        let one_c = BigComplex::one(guard, mode);
        let one_minus_z2 = &one_c - &z_sq;
        // √(1 − z²)
        let sqrt_val = one_minus_z2.sqrt(guard, mode)?;
        // i·√(1 − z²)
        let i_sqrt = mul_i(sqrt_val);
        // z + i·√(1 − z²)
        let arg = self + &i_sqrt;
        // ln(...)
        let ln_val = arg.ln(guard, mode)?;
        // −i · ln(...)
        Ok(mul_neg_i(ln_val))
    }

    /// The principal value of `arctan z` at `prec` bits.
    ///
    /// Uses the identity `atan z = (i/2)·[ln(1 − i·z) − ln(1 + i·z)]`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::ln`].
    pub fn atan(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            return Ok(BigComplex::zero(prec));
        }
        let guard = prec.saturating_add(GUARD);
        let half = BigFloat::from_f64(0.5, guard)?;

        // i·z
        let iz = mul_i(self.clone());
        let one_c = BigComplex::one(guard, mode);
        // ln(1 − i·z)
        let ln_minus = (&one_c - &iz).ln(guard, mode)?;
        // ln(1 + i·z)
        let ln_plus = (&one_c + &iz).ln(guard, mode)?;
        // diff = ln(1 − i·z) − ln(1 + i·z)
        let diff = &ln_minus - &ln_plus;
        // multiply by i/2: first multiply by i, then scale each component by ½
        let i_diff = mul_i(diff);
        let re = (&i_diff.re * &half).with_precision(prec, mode);
        let im = (&i_diff.im * &half).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }

    /// The principal value of `arcsinh z` at `prec` bits.
    ///
    /// Uses the identity `asinh z = ln(z + √(z² + 1))`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::sqrt`] / [`BigComplex::ln`].
    pub fn asinh(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            return Ok(BigComplex::zero(prec));
        }
        let guard = prec.saturating_add(GUARD);

        let one_c = BigComplex::one(guard, mode);
        // z² + 1
        let z_sq_plus_one = &self.mul_core(self) + &one_c;
        // √(z² + 1)
        let sqrt_val = z_sq_plus_one.sqrt(guard, mode)?;
        // z + √(z² + 1)
        let arg = self + &sqrt_val;
        arg.ln(guard, mode)
    }

    /// The principal value of `arccosh z` at `prec` bits.
    ///
    /// Uses `acosh z = ln(z + √(z−1)·√(z+1))` (factored, not `√(z²−1)`) to
    /// place the branch cut on `(−∞, 1]`, consistent with C99 / `num-complex`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::sqrt`] / [`BigComplex::ln`].
    pub fn acosh(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);

        let one_c = BigComplex::one(guard, mode);
        // √(z − 1)
        let sq1 = (self - &one_c).sqrt(guard, mode)?;
        // √(z + 1)
        let sq2 = (self + &one_c).sqrt(guard, mode)?;
        // z + √(z−1)·√(z+1)
        let product = sq1.mul_core(&sq2);
        let arg = self + &product;
        arg.ln(guard, mode)
    }

    /// The principal value of `arctanh z` at `prec` bits.
    ///
    /// Uses the identity `atanh z = ½·[ln(1 + z) − ln(1 − z)]`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BigComplex::ln`].
    pub fn atanh(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        if self.is_zero() {
            return Ok(BigComplex::zero(prec));
        }
        let guard = prec.saturating_add(GUARD);
        let half = BigFloat::from_f64(0.5, guard)?;

        let one_c = BigComplex::one(guard, mode);
        // ln(1 + z)
        let ln_plus = (&one_c + self).ln(guard, mode)?;
        // ln(1 − z)
        let ln_minus = (&one_c - self).ln(guard, mode)?;
        // diff = ln(1+z) − ln(1−z)
        let diff = &ln_plus - &ln_minus;
        // ½ · diff
        let re = (&diff.re * &half).with_precision(prec, mode);
        let im = (&diff.im * &half).with_precision(prec, mode);
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
    const TOL: f64 = 1e-9;

    fn c(re: f64, im: f64) -> BigComplex {
        BigComplex::from_f64(re, im, PREC).expect("finite parts")
    }

    // ---- Zero fast-paths -------------------------------------------------------

    #[test]
    fn asin_zero_is_zero() {
        let r = BigComplex::zero(PREC).asin(PREC, MODE).expect("asin");
        assert!(r.re().to_f64().abs() < TOL, "re = {}", r.re().to_f64());
        assert!(r.im().to_f64().abs() < TOL, "im = {}", r.im().to_f64());
    }

    #[test]
    fn atan_zero_is_zero() {
        let r = BigComplex::zero(PREC).atan(PREC, MODE).expect("atan");
        assert!(r.re().to_f64().abs() < TOL, "re = {}", r.re().to_f64());
        assert!(r.im().to_f64().abs() < TOL, "im = {}", r.im().to_f64());
    }

    #[test]
    fn asinh_zero_is_zero() {
        let r = BigComplex::zero(PREC).asinh(PREC, MODE).expect("asinh");
        assert!(r.re().to_f64().abs() < TOL, "re = {}", r.re().to_f64());
        assert!(r.im().to_f64().abs() < TOL, "im = {}", r.im().to_f64());
    }

    #[test]
    fn atanh_zero_is_zero() {
        let r = BigComplex::zero(PREC).atanh(PREC, MODE).expect("atanh");
        assert!(r.re().to_f64().abs() < TOL, "re = {}", r.re().to_f64());
        assert!(r.im().to_f64().abs() < TOL, "im = {}", r.im().to_f64());
    }

    // ---- Known-value checks ---------------------------------------------------

    #[test]
    fn asin_one_is_half_pi() {
        // asin(1) = π/2.
        let r = c(1.0, 0.0).asin(PREC, MODE).expect("asin");
        let half_pi = std::f64::consts::FRAC_PI_2;
        assert!(
            (r.re().to_f64() - half_pi).abs() < TOL,
            "re = {}, expected π/2 ≈ {half_pi}",
            r.re().to_f64()
        );
        assert!(r.im().to_f64().abs() < TOL, "im = {}", r.im().to_f64());
    }

    #[test]
    fn atan_one_is_quarter_pi() {
        // atan(1) = π/4.
        let r = c(1.0, 0.0).atan(PREC, MODE).expect("atan");
        let quarter_pi = std::f64::consts::FRAC_PI_4;
        assert!(
            (r.re().to_f64() - quarter_pi).abs() < TOL,
            "re = {}, expected π/4 ≈ {quarter_pi}",
            r.re().to_f64()
        );
        assert!(r.im().to_f64().abs() < TOL, "im = {}", r.im().to_f64());
    }

    #[test]
    fn acosh_one_is_zero() {
        // acosh(1) = 0.
        let r = c(1.0, 0.0).acosh(PREC, MODE).expect("acosh");
        assert!(r.re().to_f64().abs() < TOL, "re = {}", r.re().to_f64());
        assert!(r.im().to_f64().abs() < TOL, "im = {}", r.im().to_f64());
    }

    // ---- Round-trip identities ------------------------------------------------

    #[test]
    fn sin_asin_roundtrip() {
        // sin(asin(0.3+0.4i)) ≈ 0.3+0.4i.
        let z = c(0.3, 0.4);
        let asin_z = z.asin(PREC, MODE).expect("asin");
        let r = asin_z.sin(PREC, MODE).expect("sin");
        assert!(
            (r.re().to_f64() - 0.3).abs() < TOL,
            "re = {}",
            r.re().to_f64()
        );
        assert!(
            (r.im().to_f64() - 0.4).abs() < TOL,
            "im = {}",
            r.im().to_f64()
        );
    }

    #[test]
    fn cos_acos_roundtrip() {
        // cos(acos(0.3+0.4i)) ≈ 0.3+0.4i.
        let z = c(0.3, 0.4);
        let acos_z = z.acos(PREC, MODE).expect("acos");
        let r = acos_z.cos(PREC, MODE).expect("cos");
        assert!(
            (r.re().to_f64() - 0.3).abs() < TOL,
            "re = {}",
            r.re().to_f64()
        );
        assert!(
            (r.im().to_f64() - 0.4).abs() < TOL,
            "im = {}",
            r.im().to_f64()
        );
    }

    #[test]
    fn tanh_atanh_roundtrip() {
        // tanh(atanh(0.2+0.1i)) ≈ 0.2+0.1i.
        let z = c(0.2, 0.1);
        let atanh_z = z.atanh(PREC, MODE).expect("atanh");
        let r = atanh_z.tanh(PREC, MODE).expect("tanh");
        assert!(
            (r.re().to_f64() - 0.2).abs() < TOL,
            "re = {}",
            r.re().to_f64()
        );
        assert!(
            (r.im().to_f64() - 0.1).abs() < TOL,
            "im = {}",
            r.im().to_f64()
        );
    }

    #[test]
    fn sinh_asinh_roundtrip() {
        // sinh(asinh(0.3+0.4i)) ≈ 0.3+0.4i.
        let z = c(0.3, 0.4);
        let asinh_z = z.asinh(PREC, MODE).expect("asinh");
        let r = asinh_z.sinh(PREC, MODE).expect("sinh");
        assert!(
            (r.re().to_f64() - 0.3).abs() < TOL,
            "re = {}",
            r.re().to_f64()
        );
        assert!(
            (r.im().to_f64() - 0.4).abs() < TOL,
            "im = {}",
            r.im().to_f64()
        );
    }
}
