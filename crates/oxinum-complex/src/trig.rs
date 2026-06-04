//! Trigonometric and hyperbolic functions for [`crate::CBig`].
//!
//! The decimal [`DBig`] foundation (re-exported from [`oxinum_float`]) supplies
//! the four real building blocks [`sin`](oxinum_float::sin),
//! [`cos`](oxinum_float::cos), [`sinh`](oxinum_float::sinh), and
//! [`cosh`](oxinum_float::cosh) as free functions
//! `(x: &DBig, precision: usize) -> OxiNumResult<DBig>`. From them, with
//! `z = a + b·i`, the complex identities are assembled component-wise:
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
//! Every intermediate real scalar is evaluated at the working precision
//! `precision + GUARD` so that the final products carry guard digits; the
//! genuine divisions in `tan`/`tanh` route through the panic-free
//! [`CBig::checked_div`], which yields
//! [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero) exactly at
//! the poles where the complex denominator (`cos z`, resp. `cosh z`) collapses
//! to zero.

use oxinum_float::{cos, cosh, sin, sinh};

use crate::{CBig, OxiNumResult};

/// Working-precision headroom added on top of the requested `precision`.
const GUARD: usize = 10;

impl CBig {
    /// The complex sine
    /// `sin z = sin a · cosh b + i · cos a · sinh b` at `precision` digits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`sinh`/`cosh` routines.
    pub fn sin(&self, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision.saturating_add(GUARD);
        let a = &self.re;
        let b = &self.im;

        let sa = sin(a, guard)?;
        let ca = cos(a, guard)?;
        let shb = sinh(b, guard)?;
        let chb = cosh(b, guard)?;

        let re = &sa * &chb;
        let im = &ca * &shb;
        Ok(CBig::from_parts(re, im))
    }

    /// The complex cosine
    /// `cos z = cos a · cosh b − i · sin a · sinh b` at `precision` digits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`sinh`/`cosh` routines.
    pub fn cos(&self, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision.saturating_add(GUARD);
        let a = &self.re;
        let b = &self.im;

        let sa = sin(a, guard)?;
        let ca = cos(a, guard)?;
        let shb = sinh(b, guard)?;
        let chb = cosh(b, guard)?;

        let re = &ca * &chb;
        // im = −(sin a · sinh b)
        let im = -(&sa * &shb);
        Ok(CBig::from_parts(re, im))
    }

    /// The complex hyperbolic sine
    /// `sinh z = sinh a · cos b + i · cosh a · sin b` at `precision` digits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`sinh`/`cosh` routines.
    pub fn sinh(&self, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision.saturating_add(GUARD);
        let a = &self.re;
        let b = &self.im;

        let sha = sinh(a, guard)?;
        let cha = cosh(a, guard)?;
        let sb = sin(b, guard)?;
        let cb = cos(b, guard)?;

        let re = &sha * &cb;
        let im = &cha * &sb;
        Ok(CBig::from_parts(re, im))
    }

    /// The complex hyperbolic cosine
    /// `cosh z = cosh a · cos b + i · sinh a · sin b` at `precision` digits.
    ///
    /// # Errors
    ///
    /// Propagates errors from the real `sin`/`cos`/`sinh`/`cosh` routines.
    pub fn cosh(&self, precision: usize) -> OxiNumResult<CBig> {
        let guard = precision.saturating_add(GUARD);
        let a = &self.re;
        let b = &self.im;

        let sha = sinh(a, guard)?;
        let cha = cosh(a, guard)?;
        let sb = sin(b, guard)?;
        let cb = cos(b, guard)?;

        let re = &cha * &cb;
        let im = &sha * &sb;
        Ok(CBig::from_parts(re, im))
    }

    /// The complex tangent `tan z = sin z / cos z` at `precision` digits.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero) at the
    ///   poles where `cos z = 0`.
    /// - Propagates errors from the real `sin`/`cos`/`sinh`/`cosh` routines.
    pub fn tan(&self, precision: usize) -> OxiNumResult<CBig> {
        self.sin(precision)?.checked_div(&self.cos(precision)?)
    }

    /// The complex hyperbolic tangent `tanh z = sinh z / cosh z` at
    /// `precision` digits.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero) at the
    ///   poles where `cosh z = 0`.
    /// - Propagates errors from the real `sin`/`cos`/`sinh`/`cosh` routines.
    pub fn tanh(&self, precision: usize) -> OxiNumResult<CBig> {
        self.sinh(precision)?.checked_div(&self.cosh(precision)?)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const PREC: usize = 40;

    /// Build a `CBig` from two `f64`s.
    fn c(re: f64, im: f64) -> CBig {
        CBig::from_f64(re, im).expect("finite parts")
    }

    #[test]
    fn sin_zero_is_zero() {
        let s = CBig::zero().sin(PREC).expect("sin");
        let (re, im) = s.to_f64_parts();
        assert!(re.abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn cos_zero_is_one() {
        let cz = CBig::zero().cos(PREC).expect("cos");
        let (re, im) = cz.to_f64_parts();
        assert!((re - 1.0).abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn sinh_zero_is_zero() {
        let s = CBig::zero().sinh(PREC).expect("sinh");
        let (re, im) = s.to_f64_parts();
        assert!(re.abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn cosh_zero_is_one() {
        let cz = CBig::zero().cosh(PREC).expect("cosh");
        let (re, im) = cz.to_f64_parts();
        assert!((re - 1.0).abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn pythagorean_identity_general() {
        // sin²z + cos²z ≈ 1 at z = 0.5 + 0.3i, using the public Mul/Add ops.
        let z = c(0.5, 0.3);
        let s = z.sin(PREC).expect("sin");
        let co = z.cos(PREC).expect("cos");
        let sum = &(&s * &s) + &(&co * &co);
        let (re, im) = sum.to_f64_parts();
        assert!((re - 1.0).abs() < 1e-9, "re(sum) = {re}");
        assert!(im.abs() < 1e-9, "im(sum) = {im}");
    }

    #[test]
    fn cosh_sq_minus_sinh_sq_is_one() {
        // cosh²z − sinh²z ≈ 1 at z = 0.4 + 0.7i, using the public ops.
        let z = c(0.4, 0.7);
        let ch = z.cosh(PREC).expect("cosh");
        let sh = z.sinh(PREC).expect("sinh");
        let diff = &(&ch * &ch) - &(&sh * &sh);
        let (re, im) = diff.to_f64_parts();
        assert!((re - 1.0).abs() < 1e-9, "re(diff) = {re}");
        assert!(im.abs() < 1e-9, "im(diff) = {im}");
    }

    #[test]
    fn tan_zero_is_zero() {
        let t = CBig::zero().tan(PREC).expect("tan");
        let (re, im) = t.to_f64_parts();
        assert!(re.abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn tanh_zero_is_zero() {
        let t = CBig::zero().tanh(PREC).expect("tanh");
        let (re, im) = t.to_f64_parts();
        assert!(re.abs() < 1e-12, "re = {re}");
        assert!(im.abs() < 1e-12, "im = {im}");
    }

    #[test]
    fn tan_matches_known_value() {
        // (0.5 + 0.3i).tan() ≈ 0.48759231649213874 + 0.3689103968255638i
        // (reference computed with the standard complex-tan formula).
        let z = c(0.5, 0.3);
        let t = z.tan(PREC).expect("tan");
        let (re, im) = t.to_f64_parts();
        assert!((re - 0.487_592_316_492_138_74).abs() < 1e-9, "re = {re}");
        assert!((im - 0.368_910_396_825_563_8).abs() < 1e-9, "im = {im}");
    }

    #[test]
    fn tan_is_sin_over_cos() {
        // tan z is sin z / cos z by construction; confirm the public path.
        let z = c(0.5, 0.3);
        let t = z.tan(PREC).expect("tan");
        let q = z
            .sin(PREC)
            .expect("sin")
            .checked_div(&z.cos(PREC).expect("cos"))
            .expect("non-zero cos");
        let (tre, tim) = t.to_f64_parts();
        let (qre, qim) = q.to_f64_parts();
        assert!((tre - qre).abs() < 1e-12, "re: {tre} vs {qre}");
        assert!((tim - qim).abs() < 1e-12, "im: {tim} vs {qim}");
    }
}
