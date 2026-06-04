//! Trigonometric and hyperbolic functions for native [`BigComplex`].
//!
//! The real [`BigFloat`] foundation provides `sin`, `cos`, `tan`, but no
//! hyperbolic functions, so the private helpers [`bf_sinh`], [`bf_cosh`], and
//! [`bf_tanh`] derive them from the exponential:
//!
//! ```text
//! sinh x = (eˣ − e⁻ˣ) / 2
//! cosh x = (eˣ + e⁻ˣ) / 2
//! tanh x = sinh x / cosh x
//! ```
//!
//! Halving is done by multiplying with the exact constant `0.5` so the
//! panicking real `/` operator is never touched; the genuine division in
//! `tanh` goes through [`BigFloat::div_ref_with_mode`].
//!
//! With `z = a + b·i` the complex identities are:
//!
//! ```text
//! sin z  = sin a · cosh b + i · cos a · sinh b
//! cos z  = cos a · cosh b − i · sin a · sinh b
//! sinh z = sinh a · cos b + i · cosh a · sin b
//! cosh z = cosh a · cos b + i · sinh a · sin b
//! tan z  = sin z / cos z          (via checked_div)
//! tanh z = sinh z / cosh z        (via checked_div)
//! ```
//!
//! `tan`/`tanh` propagate [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero)
//! at their poles (where the complex denominator collapses to zero).

use oxinum_core::OxiNumResult;
use oxinum_float::native::{BigFloat, RoundingMode};

use super::BigComplex;

/// Working-precision headroom added on top of the requested `prec`.
const GUARD: u32 = 10;

// ---------------------------------------------------------------------------
// Private real hyperbolic helpers, derived from BigFloat::exp.
// ---------------------------------------------------------------------------

/// `sinh(x) = (eˣ − e⁻ˣ) / 2` at `prec` bits.
fn bf_sinh(x: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    let ex = x.exp(prec, mode)?;
    let e_neg = (-x).exp(prec, mode)?;
    let half = BigFloat::from_f64(0.5, prec)?;
    let diff = &ex - &e_neg;
    Ok((&diff * &half).with_precision(prec, mode))
}

/// `cosh(x) = (eˣ + e⁻ˣ) / 2` at `prec` bits.
fn bf_cosh(x: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    let ex = x.exp(prec, mode)?;
    let e_neg = (-x).exp(prec, mode)?;
    let half = BigFloat::from_f64(0.5, prec)?;
    let sum = &ex + &e_neg;
    Ok((&sum * &half).with_precision(prec, mode))
}

/// `tanh(x) = sinh(x) / cosh(x)` at `prec` bits.
///
/// `cosh(x) ≥ 1` for all real `x`, so the division never hits zero. Used by the
/// real-axis fast path of the complex [`BigComplex::tanh`] (where `b == 0`,
/// `tanh z` collapses to the real scalar `tanh a`), keeping that case off the
/// general complex-division route.
fn bf_tanh(x: &BigFloat, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat> {
    let sinh = bf_sinh(x, prec, mode)?;
    let cosh = bf_cosh(x, prec, mode)?;
    Ok(sinh
        .div_ref_with_mode(&cosh, mode)?
        .with_precision(prec, mode))
}

impl BigComplex {
    /// The complex sine
    /// `sin z = sin a · cosh b + i · cos a · sinh b` at `prec` bits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`exp` routines.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// let s = BigComplex::zero(80).sin(80, RoundingMode::HalfEven).expect("sin");
    /// assert!(s.re().to_f64().abs() < 1e-12);
    /// assert!(s.im().to_f64().abs() < 1e-12);
    /// ```
    pub fn sin(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);
        let a = self.re.clone().with_precision(guard, mode);
        let b = self.im.clone().with_precision(guard, mode);

        let sin_a = a.sin(guard, mode)?;
        let cos_a = a.cos(guard, mode)?;
        let cosh_b = bf_cosh(&b, guard, mode)?;
        let sinh_b = bf_sinh(&b, guard, mode)?;

        let re = (&sin_a * &cosh_b).with_precision(prec, mode);
        let im = (&cos_a * &sinh_b).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }

    /// The complex cosine
    /// `cos z = cos a · cosh b − i · sin a · sinh b` at `prec` bits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`exp` routines.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// let c = BigComplex::zero(80).cos(80, RoundingMode::HalfEven).expect("cos");
    /// assert!((c.re().to_f64() - 1.0).abs() < 1e-12);
    /// assert!(c.im().to_f64().abs() < 1e-12);
    /// ```
    pub fn cos(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);
        let a = self.re.clone().with_precision(guard, mode);
        let b = self.im.clone().with_precision(guard, mode);

        let cos_a = a.cos(guard, mode)?;
        let sin_a = a.sin(guard, mode)?;
        let cosh_b = bf_cosh(&b, guard, mode)?;
        let sinh_b = bf_sinh(&b, guard, mode)?;

        let re = (&cos_a * &cosh_b).with_precision(prec, mode);
        // im = −(sin a · sinh b)
        let prod = &sin_a * &sinh_b;
        let im = (-&prod).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }

    /// The complex hyperbolic sine
    /// `sinh z = sinh a · cos b + i · cosh a · sin b` at `prec` bits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`exp` routines.
    pub fn sinh(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);
        let a = self.re.clone().with_precision(guard, mode);
        let b = self.im.clone().with_precision(guard, mode);

        let sinh_a = bf_sinh(&a, guard, mode)?;
        let cosh_a = bf_cosh(&a, guard, mode)?;
        let cos_b = b.cos(guard, mode)?;
        let sin_b = b.sin(guard, mode)?;

        let re = (&sinh_a * &cos_b).with_precision(prec, mode);
        let im = (&cosh_a * &sin_b).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }

    /// The complex hyperbolic cosine
    /// `cosh z = cosh a · cos b + i · sinh a · sin b` at `prec` bits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`exp` routines.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// let c = BigComplex::zero(80).cosh(80, RoundingMode::HalfEven).expect("cosh");
    /// assert!((c.re().to_f64() - 1.0).abs() < 1e-12);
    /// assert!(c.im().to_f64().abs() < 1e-12);
    /// ```
    pub fn cosh(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);
        let a = self.re.clone().with_precision(guard, mode);
        let b = self.im.clone().with_precision(guard, mode);

        let cosh_a = bf_cosh(&a, guard, mode)?;
        let sinh_a = bf_sinh(&a, guard, mode)?;
        let cos_b = b.cos(guard, mode)?;
        let sin_b = b.sin(guard, mode)?;

        let re = (&cosh_a * &cos_b).with_precision(prec, mode);
        let im = (&sinh_a * &sin_b).with_precision(prec, mode);
        Ok(BigComplex { re, im })
    }

    /// The complex tangent `tan z = sin z / cos z` at `prec` bits.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero) at the
    ///   poles where `cos z = 0`.
    /// - Propagates errors from the real `sin`/`cos`/`exp` routines.
    pub fn tan(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);
        let sin_z = self.sin(guard, mode)?;
        let cos_z = self.cos(guard, mode)?;
        sin_z.checked_div(&cos_z, prec, mode)
    }

    /// The complex hyperbolic tangent `tanh z = sinh z / cosh z` at `prec` bits.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero) at the
    ///   poles where `cosh z = 0`.
    /// - Propagates errors from the real `sin`/`cos`/`exp` routines.
    pub fn tanh(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<BigComplex> {
        let guard = prec.saturating_add(GUARD);

        // Real-axis fast path: b == 0 ⇒ tanh z = tanh(a) (purely real).
        // Routes through the scalar `bf_tanh` instead of the general complex
        // division, avoiding any spurious imaginary component from rounding.
        if self.im.is_zero() {
            let a = self.re.clone().with_precision(guard, mode);
            let t = bf_tanh(&a, guard, mode)?.with_precision(prec, mode);
            return Ok(BigComplex {
                re: t,
                im: BigFloat::zero(prec),
            });
        }

        let sinh_z = self.sinh(guard, mode)?;
        let cosh_z = self.cosh(guard, mode)?;
        sinh_z.checked_div(&cosh_z, prec, mode)
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
    fn sin_zero_is_zero() {
        let s = BigComplex::zero(PREC).sin(PREC, MODE).expect("sin");
        assert!(s.re().to_f64().abs() < 1e-12, "re = {}", s.re().to_f64());
        assert!(s.im().to_f64().abs() < 1e-12, "im = {}", s.im().to_f64());
    }

    #[test]
    fn cos_zero_is_one() {
        let cz = BigComplex::zero(PREC).cos(PREC, MODE).expect("cos");
        assert!(
            (cz.re().to_f64() - 1.0).abs() < 1e-12,
            "re = {}",
            cz.re().to_f64()
        );
        assert!(cz.im().to_f64().abs() < 1e-12, "im = {}", cz.im().to_f64());
    }

    #[test]
    fn cosh_zero_is_one() {
        let cz = BigComplex::zero(PREC).cosh(PREC, MODE).expect("cosh");
        assert!(
            (cz.re().to_f64() - 1.0).abs() < 1e-12,
            "re = {}",
            cz.re().to_f64()
        );
        assert!(cz.im().to_f64().abs() < 1e-12, "im = {}", cz.im().to_f64());
    }

    #[test]
    fn sinh_zero_is_zero() {
        let s = BigComplex::zero(PREC).sinh(PREC, MODE).expect("sinh");
        assert!(s.re().to_f64().abs() < 1e-12);
        assert!(s.im().to_f64().abs() < 1e-12);
    }

    #[test]
    fn pythagorean_identity_general() {
        // sin²z + cos²z ≈ 1 at z = 0.5 + 0.3i.
        let z = c(0.5, 0.3);
        let s = z.sin(PREC, MODE).expect("sin");
        let co = z.cos(PREC, MODE).expect("cos");
        let s2 = s.mul_core(&s);
        let c2 = co.mul_core(&co);
        let sum = &s2 + &c2;
        assert!(
            (sum.re().to_f64() - 1.0).abs() < 1e-9,
            "re(sum) = {}",
            sum.re().to_f64()
        );
        assert!(
            sum.im().to_f64().abs() < 1e-9,
            "im(sum) = {}",
            sum.im().to_f64()
        );
    }

    #[test]
    fn tan_zero_is_zero() {
        let t = BigComplex::zero(PREC).tan(PREC, MODE).expect("tan");
        assert!(t.re().to_f64().abs() < 1e-12);
        assert!(t.im().to_f64().abs() < 1e-12);
    }

    #[test]
    fn tanh_zero_is_zero() {
        let t = BigComplex::zero(PREC).tanh(PREC, MODE).expect("tanh");
        assert!(t.re().to_f64().abs() < 1e-12);
        assert!(t.im().to_f64().abs() < 1e-12);
    }

    #[test]
    fn tan_matches_real_axis() {
        // tan(0.4 + 0i) ≈ tan(0.4) real.
        let t = c(0.4, 0.0).tan(PREC, MODE).expect("tan");
        let expected = 0.4_f64.tan();
        assert!(
            (t.re().to_f64() - expected).abs() < 1e-9,
            "re = {}",
            t.re().to_f64()
        );
        assert!(t.im().to_f64().abs() < 1e-9, "im = {}", t.im().to_f64());
    }
}
