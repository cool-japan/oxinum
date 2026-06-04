//! Arithmetic operator implementations for native [`BigComplex`].
//!
//! Every infallible binary operator — `Add`, `Sub`, `Mul` — provides all four
//! ownership variants (`&T op &T`, `T op T`, `T op &T`, `&T op T`), each
//! producing an owned [`BigComplex`]. The matching `*Assign` traits are wired
//! up for both owned and borrowed right-hand sides, and [`Neg`] is provided for
//! owned and borrowed receivers. Component-wise [`PartialEq`] is also provided.
//!
//! # Complex arithmetic
//!
//! With `a = a_re + a_im·i` and `b = b_re + b_im·i`:
//!
//! ```text
//! a + b = (a_re + b_re) + (a_im + b_im)·i
//! a − b = (a_re − b_re) + (a_im − b_im)·i
//! a · b = (a_re·b_re − a_im·b_im) + (a_re·b_im + a_im·b_re)·i
//! a / b = (a · conj(b)) / |b|²
//!       = (a_re·b_re + a_im·b_im)/|b|² + (a_im·b_re − a_re·b_im)/|b|²·i
//! ```
//!
//! # Why there is no `Div` operator and no `Default`
//!
//! * **No `Div` operator.** Native complex division is intrinsically
//!   precision-and-rounding aware (it must divide by the real `norm_sqr`), so
//!   it cannot be expressed through the precision-free `core::ops::Div`
//!   signature without smuggling in a default precision. Division is therefore
//!   exposed only as the explicit [`BigComplex::checked_div`], which takes
//!   `(prec, mode)` and returns [`OxiNumResult`], surfacing
//!   [`OxiNumError::DivByZero`] for a zero divisor rather than panicking.
//! * **No `Default`.** A `BigComplex` value is meaningless without a chosen
//!   working precision, and there is no single precision the library can pick
//!   that is correct for every caller. Rather than bake in an arbitrary
//!   constant, `Default` is intentionally *not* implemented; callers construct
//!   the additive identity explicitly via [`BigComplex::zero(prec)`].
//!
//! `Add`/`Sub`/`Neg` are infallible because the underlying [`BigFloat`]
//! operators are infallible; `Mul` routes through the private [`mul_core`],
//! which combines the four cross-products with banker's rounding at each
//! component's own precision.

use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use oxinum_core::OxiNumResult;
use oxinum_float::native::{BigFloat, RoundingMode};

use super::BigComplex;

impl BigComplex {
    /// The complex product `self · rhs` via the four real cross-products.
    ///
    /// `(a + b·i)(c + d·i) = (a·c − b·d) + (a·d + b·c)·i`.
    ///
    /// Each component is formed with banker's rounding ([`RoundingMode::HalfEven`])
    /// using the infallible reference multiply/add on [`BigFloat`].
    pub(crate) fn mul_core(&self, rhs: &BigComplex) -> BigComplex {
        let a = &self.re;
        let b = &self.im;
        let c = &rhs.re;
        let d = &rhs.im;

        // re = a·c − b·d
        let ac = a * c;
        let bd = b * d;
        let re = &ac - &bd;

        // im = a·d + b·c
        let ad = a * d;
        let bc = b * c;
        let im = &ad + &bc;

        BigComplex { re, im }
    }

    /// The complex quotient `self / rhs` at `prec` bits with the given rounding
    /// mode, computed as `(self · conj(rhs)) / |rhs|²`.
    ///
    /// Returns [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero)
    /// when `rhs` is the (component-wise) zero, detected via
    /// [`BigComplex::is_zero`] / a zero `norm_sqr`. This is the no-panic core
    /// shared by [`BigComplex::checked_div`].
    pub(crate) fn div_core(
        &self,
        rhs: &BigComplex,
        prec: u32,
        mode: RoundingMode,
    ) -> OxiNumResult<BigComplex> {
        // Zero-divisor guard: surface DivByZero rather than panicking. The
        // real div_ref below would itself return DivByZero, but checking the
        // norm first gives a single, unambiguous error site.
        if rhs.is_zero() {
            return Err(oxinum_core::OxiNumError::DivByZero);
        }

        let guard = prec.saturating_add(10);

        let a = &self.re;
        let b = &self.im;
        let c = &rhs.re;
        let d = &rhs.im;

        // denominator = c² + d² (real, strictly positive here).
        let denom = rhs.norm_sqr();

        // numerator real part: a·c + b·d
        let ac = a * c;
        let bd = b * d;
        let num_re = &ac + &bd;

        // numerator imag part: b·c − a·d
        let bc = b * c;
        let ad = a * d;
        let num_im = &bc - &ad;

        // `guard` documents the headroom carried by `norm_sqr` / the
        // cross-products (all formed at the operands' own precision, which the
        // callers set >= prec); the final components are delivered at `prec`.
        let _ = guard;
        let re = num_re
            .div_ref_with_mode(&denom, mode)?
            .with_precision(prec, mode);
        let im = num_im
            .div_ref_with_mode(&denom, mode)?
            .with_precision(prec, mode);

        Ok(BigComplex { re, im })
    }

    /// Checked complex division `self / rhs` at `prec` bits.
    ///
    /// This is the public, no-panic division entry point for native complex
    /// values (there is deliberately no `core::ops::Div` impl — see the module
    /// docs). The result components are rounded to `prec` bits with `mode`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero)
    /// if `rhs` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::{BigComplex, RoundingMode};
    /// // (2 + 0i) / (1 + i) = 1 − i.
    /// let num = BigComplex::from_f64(2.0, 0.0, 80).expect("finite");
    /// let den = BigComplex::from_f64(1.0, 1.0, 80).expect("finite");
    /// let q = num
    ///     .checked_div(&den, 80, RoundingMode::HalfEven)
    ///     .expect("non-zero divisor");
    /// assert!((q.re().to_f64() - 1.0).abs() < 1e-12);
    /// assert!((q.im().to_f64() + 1.0).abs() < 1e-12);
    /// ```
    pub fn checked_div(
        &self,
        rhs: &BigComplex,
        prec: u32,
        mode: RoundingMode,
    ) -> OxiNumResult<BigComplex> {
        self.div_core(rhs, prec, mode)
    }
}

// ---------------------------------------------------------------------------
// Add
// ---------------------------------------------------------------------------

impl Add<&BigComplex> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: &BigComplex) -> BigComplex {
        BigComplex {
            re: &self.re + &rhs.re,
            im: &self.im + &rhs.im,
        }
    }
}

impl Add<BigComplex> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: BigComplex) -> BigComplex {
        (&self).add(&rhs)
    }
}

impl Add<&BigComplex> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: &BigComplex) -> BigComplex {
        (&self).add(rhs)
    }
}

impl Add<BigComplex> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: BigComplex) -> BigComplex {
        self.add(&rhs)
    }
}

impl AddAssign<&BigComplex> for BigComplex {
    #[inline]
    fn add_assign(&mut self, rhs: &BigComplex) {
        *self = (&*self).add(rhs);
    }
}

impl AddAssign<BigComplex> for BigComplex {
    #[inline]
    fn add_assign(&mut self, rhs: BigComplex) {
        *self = (&*self).add(&rhs);
    }
}

// ---------------------------------------------------------------------------
// Sub
// ---------------------------------------------------------------------------

impl Sub<&BigComplex> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: &BigComplex) -> BigComplex {
        BigComplex {
            re: &self.re - &rhs.re,
            im: &self.im - &rhs.im,
        }
    }
}

impl Sub<BigComplex> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: BigComplex) -> BigComplex {
        (&self).sub(&rhs)
    }
}

impl Sub<&BigComplex> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: &BigComplex) -> BigComplex {
        (&self).sub(rhs)
    }
}

impl Sub<BigComplex> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: BigComplex) -> BigComplex {
        self.sub(&rhs)
    }
}

impl SubAssign<&BigComplex> for BigComplex {
    #[inline]
    fn sub_assign(&mut self, rhs: &BigComplex) {
        *self = (&*self).sub(rhs);
    }
}

impl SubAssign<BigComplex> for BigComplex {
    #[inline]
    fn sub_assign(&mut self, rhs: BigComplex) {
        *self = (&*self).sub(&rhs);
    }
}

// ---------------------------------------------------------------------------
// Mul
// ---------------------------------------------------------------------------

impl Mul<&BigComplex> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: &BigComplex) -> BigComplex {
        self.mul_core(rhs)
    }
}

impl Mul<BigComplex> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: BigComplex) -> BigComplex {
        self.mul_core(&rhs)
    }
}

impl Mul<&BigComplex> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: &BigComplex) -> BigComplex {
        self.mul_core(rhs)
    }
}

impl Mul<BigComplex> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: BigComplex) -> BigComplex {
        self.mul_core(&rhs)
    }
}

impl MulAssign<&BigComplex> for BigComplex {
    #[inline]
    fn mul_assign(&mut self, rhs: &BigComplex) {
        *self = self.mul_core(rhs);
    }
}

impl MulAssign<BigComplex> for BigComplex {
    #[inline]
    fn mul_assign(&mut self, rhs: BigComplex) {
        *self = self.mul_core(&rhs);
    }
}

// ---------------------------------------------------------------------------
// Neg
// ---------------------------------------------------------------------------

impl Neg for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn neg(self) -> BigComplex {
        BigComplex {
            re: -&self.re,
            im: -&self.im,
        }
    }
}

impl Neg for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn neg(self) -> BigComplex {
        (&self).neg()
    }
}

// ---------------------------------------------------------------------------
// PartialEq — component-wise (no Eq: BigFloat is not Eq because of NaN).
// ---------------------------------------------------------------------------

impl PartialEq for BigComplex {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.re == other.re && self.im == other.im
    }
}

// ---------------------------------------------------------------------------
// Scalar helpers for BigComplex + {BigFloat, i64}
// ---------------------------------------------------------------------------

/// `z + d = (re + d, im)`.
#[inline]
fn add_scalar_bf(z: &BigComplex, d: &BigFloat) -> BigComplex {
    BigComplex {
        re: &z.re + d,
        im: z.im.clone(),
    }
}

/// `z - d = (re - d, im)`.
#[inline]
fn sub_scalar_bf(z: &BigComplex, d: &BigFloat) -> BigComplex {
    BigComplex {
        re: &z.re - d,
        im: z.im.clone(),
    }
}

/// `d - z = (d - re, -im)`.
#[inline]
fn rsub_scalar_bf(d: &BigFloat, z: &BigComplex) -> BigComplex {
    BigComplex {
        re: d - &z.re,
        im: -&z.im,
    }
}

/// `z * d = (re*d, im*d)`.
#[inline]
fn mul_scalar_bf(z: &BigComplex, d: &BigFloat) -> BigComplex {
    BigComplex {
        re: &z.re * d,
        im: &z.im * d,
    }
}

/// Convert an `i64` to a `BigFloat` at precision `prec`.
#[inline]
fn bf_from_i64_at(n: i64, prec: u32) -> BigFloat {
    BigFloat::from_i64(n, prec, RoundingMode::HalfEven)
}

/// Scalar `i64` cores — delegate to the `BigFloat` variants after converting.
#[inline]
fn add_scalar_i64(z: &BigComplex, n: i64) -> BigComplex {
    add_scalar_bf(z, &bf_from_i64_at(n, z.re.precision()))
}
#[inline]
fn sub_scalar_i64(z: &BigComplex, n: i64) -> BigComplex {
    sub_scalar_bf(z, &bf_from_i64_at(n, z.re.precision()))
}
#[inline]
fn rsub_scalar_i64(n: i64, z: &BigComplex) -> BigComplex {
    rsub_scalar_bf(&bf_from_i64_at(n, z.re.precision()), z)
}
#[inline]
fn mul_scalar_i64(z: &BigComplex, n: i64) -> BigComplex {
    mul_scalar_bf(z, &bf_from_i64_at(n, z.re.precision()))
}

// ---------------------------------------------------------------------------
// BigComplex op BigFloat — Add
// ---------------------------------------------------------------------------

impl Add<BigFloat> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: BigFloat) -> BigComplex {
        add_scalar_bf(&self, &rhs)
    }
}

impl Add<&BigFloat> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: &BigFloat) -> BigComplex {
        add_scalar_bf(&self, rhs)
    }
}

impl Add<BigFloat> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: BigFloat) -> BigComplex {
        add_scalar_bf(self, &rhs)
    }
}

impl Add<&BigFloat> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: &BigFloat) -> BigComplex {
        add_scalar_bf(self, rhs)
    }
}

impl AddAssign<BigFloat> for BigComplex {
    #[inline]
    fn add_assign(&mut self, rhs: BigFloat) {
        *self = add_scalar_bf(self, &rhs);
    }
}

impl AddAssign<&BigFloat> for BigComplex {
    #[inline]
    fn add_assign(&mut self, rhs: &BigFloat) {
        *self = add_scalar_bf(self, rhs);
    }
}

// ---------------------------------------------------------------------------
// BigComplex op BigFloat — Sub
// ---------------------------------------------------------------------------

impl Sub<BigFloat> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: BigFloat) -> BigComplex {
        sub_scalar_bf(&self, &rhs)
    }
}

impl Sub<&BigFloat> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: &BigFloat) -> BigComplex {
        sub_scalar_bf(&self, rhs)
    }
}

impl Sub<BigFloat> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: BigFloat) -> BigComplex {
        sub_scalar_bf(self, &rhs)
    }
}

impl Sub<&BigFloat> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: &BigFloat) -> BigComplex {
        sub_scalar_bf(self, rhs)
    }
}

impl SubAssign<BigFloat> for BigComplex {
    #[inline]
    fn sub_assign(&mut self, rhs: BigFloat) {
        *self = sub_scalar_bf(self, &rhs);
    }
}

impl SubAssign<&BigFloat> for BigComplex {
    #[inline]
    fn sub_assign(&mut self, rhs: &BigFloat) {
        *self = sub_scalar_bf(self, rhs);
    }
}

// ---------------------------------------------------------------------------
// BigFloat op BigComplex — reversed Sub (d - z)
// ---------------------------------------------------------------------------

impl Sub<BigComplex> for BigFloat {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: BigComplex) -> BigComplex {
        rsub_scalar_bf(&self, &rhs)
    }
}

impl Sub<&BigComplex> for BigFloat {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: &BigComplex) -> BigComplex {
        rsub_scalar_bf(&self, rhs)
    }
}

impl Sub<BigComplex> for &BigFloat {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: BigComplex) -> BigComplex {
        rsub_scalar_bf(self, &rhs)
    }
}

impl Sub<&BigComplex> for &BigFloat {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: &BigComplex) -> BigComplex {
        rsub_scalar_bf(self, rhs)
    }
}

// ---------------------------------------------------------------------------
// BigComplex op BigFloat — Mul
// ---------------------------------------------------------------------------

impl Mul<BigFloat> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: BigFloat) -> BigComplex {
        mul_scalar_bf(&self, &rhs)
    }
}

impl Mul<&BigFloat> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: &BigFloat) -> BigComplex {
        mul_scalar_bf(&self, rhs)
    }
}

impl Mul<BigFloat> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: BigFloat) -> BigComplex {
        mul_scalar_bf(self, &rhs)
    }
}

impl Mul<&BigFloat> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: &BigFloat) -> BigComplex {
        mul_scalar_bf(self, rhs)
    }
}

impl MulAssign<BigFloat> for BigComplex {
    #[inline]
    fn mul_assign(&mut self, rhs: BigFloat) {
        *self = mul_scalar_bf(self, &rhs);
    }
}

impl MulAssign<&BigFloat> for BigComplex {
    #[inline]
    fn mul_assign(&mut self, rhs: &BigFloat) {
        *self = mul_scalar_bf(self, rhs);
    }
}

// ---------------------------------------------------------------------------
// BigComplex op i64 — Add
// ---------------------------------------------------------------------------

impl Add<i64> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: i64) -> BigComplex {
        add_scalar_i64(&self, rhs)
    }
}

impl Add<i64> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn add(self, rhs: i64) -> BigComplex {
        add_scalar_i64(self, rhs)
    }
}

impl AddAssign<i64> for BigComplex {
    #[inline]
    fn add_assign(&mut self, rhs: i64) {
        *self = add_scalar_i64(self, rhs);
    }
}

// ---------------------------------------------------------------------------
// BigComplex op i64 — Sub
// ---------------------------------------------------------------------------

impl Sub<i64> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: i64) -> BigComplex {
        sub_scalar_i64(&self, rhs)
    }
}

impl Sub<i64> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: i64) -> BigComplex {
        sub_scalar_i64(self, rhs)
    }
}

impl SubAssign<i64> for BigComplex {
    #[inline]
    fn sub_assign(&mut self, rhs: i64) {
        *self = sub_scalar_i64(self, rhs);
    }
}

// ---------------------------------------------------------------------------
// i64 op BigComplex — reversed Sub (n - z)
// ---------------------------------------------------------------------------

impl Sub<BigComplex> for i64 {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: BigComplex) -> BigComplex {
        rsub_scalar_i64(self, &rhs)
    }
}

impl Sub<&BigComplex> for i64 {
    type Output = BigComplex;
    #[inline]
    fn sub(self, rhs: &BigComplex) -> BigComplex {
        rsub_scalar_i64(self, rhs)
    }
}

// ---------------------------------------------------------------------------
// BigComplex op i64 — Mul
// ---------------------------------------------------------------------------

impl Mul<i64> for BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: i64) -> BigComplex {
        mul_scalar_i64(&self, rhs)
    }
}

impl Mul<i64> for &BigComplex {
    type Output = BigComplex;
    #[inline]
    fn mul(self, rhs: i64) -> BigComplex {
        mul_scalar_i64(self, rhs)
    }
}

impl MulAssign<i64> for BigComplex {
    #[inline]
    fn mul_assign(&mut self, rhs: i64) {
        *self = mul_scalar_i64(self, rhs);
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
    fn add_sub_componentwise() {
        let z = &c(1.0, 2.0) + &c(3.0, -4.0);
        assert!((z.re().to_f64() - 4.0).abs() < 1e-12);
        assert!((z.im().to_f64() + 2.0).abs() < 1e-12);

        let w = &c(1.0, 2.0) - &c(3.0, -4.0);
        assert!((w.re().to_f64() + 2.0).abs() < 1e-12);
        assert!((w.im().to_f64() - 6.0).abs() < 1e-12);
    }

    #[test]
    fn mul_basic() {
        // (1 + 2i)(3 + 4i) = -5 + 10i
        let z = &c(1.0, 2.0) * &c(3.0, 4.0);
        assert!(
            (z.re().to_f64() + 5.0).abs() < 1e-12,
            "re = {}",
            z.re().to_f64()
        );
        assert!(
            (z.im().to_f64() - 10.0).abs() < 1e-12,
            "im = {}",
            z.im().to_f64()
        );
    }

    #[test]
    fn mul_i_squared() {
        // (1 + i)² = 2i
        let one_plus_i = c(1.0, 1.0);
        let z = &one_plus_i * &one_plus_i;
        assert!(z.re().to_f64().abs() < 1e-12, "re = {}", z.re().to_f64());
        assert!(
            (z.im().to_f64() - 2.0).abs() < 1e-12,
            "im = {}",
            z.im().to_f64()
        );
    }

    #[test]
    fn checked_div_basic() {
        // (2 + 0i) / (1 + i) = 1 − i
        let q = c(2.0, 0.0)
            .checked_div(&c(1.0, 1.0), PREC, MODE)
            .expect("non-zero divisor");
        assert!(
            (q.re().to_f64() - 1.0).abs() < 1e-12,
            "re = {}",
            q.re().to_f64()
        );
        assert!(
            (q.im().to_f64() + 1.0).abs() < 1e-12,
            "im = {}",
            q.im().to_f64()
        );
    }

    #[test]
    fn checked_div_general() {
        // (1 + 2i)/(3 + 4i) = (11 + 2i)/25 = 0.44 + 0.08i
        let q = c(1.0, 2.0)
            .checked_div(&c(3.0, 4.0), PREC, MODE)
            .expect("non-zero divisor");
        assert!(
            (q.re().to_f64() - 0.44).abs() < 1e-12,
            "re = {}",
            q.re().to_f64()
        );
        assert!(
            (q.im().to_f64() - 0.08).abs() < 1e-12,
            "im = {}",
            q.im().to_f64()
        );
    }

    #[test]
    fn checked_div_by_zero_is_err() {
        let q = c(1.0, 1.0).checked_div(&BigComplex::zero(PREC), PREC, MODE);
        assert!(
            matches!(q, Err(oxinum_core::OxiNumError::DivByZero)),
            "expected DivByZero, got {q:?}"
        );
    }

    #[test]
    fn neg_and_assign_ops() {
        let z = -&c(1.0, -2.0);
        assert!((z.re().to_f64() + 1.0).abs() < 1e-12);
        assert!((z.im().to_f64() - 2.0).abs() < 1e-12);

        let mut a = c(1.0, 1.0);
        a += &c(2.0, 3.0);
        assert!((a.re().to_f64() - 3.0).abs() < 1e-12);
        assert!((a.im().to_f64() - 4.0).abs() < 1e-12);

        let mut b = c(5.0, 5.0);
        b -= c(1.0, 2.0);
        assert!((b.re().to_f64() - 4.0).abs() < 1e-12);
        assert!((b.im().to_f64() - 3.0).abs() < 1e-12);

        let mut m = c(1.0, 2.0);
        m *= &c(3.0, 4.0);
        assert!((m.re().to_f64() + 5.0).abs() < 1e-12);
        assert!((m.im().to_f64() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn partial_eq_componentwise() {
        assert_eq!(c(1.0, 2.0), c(1.0, 2.0));
        assert_ne!(c(1.0, 2.0), c(1.0, 2.5));
    }

    // -----------------------------------------------------------------------
    // Scalar BigFloat ops
    // -----------------------------------------------------------------------

    #[test]
    fn add_bigfloat_scalar() {
        let z = BigComplex::from_f64(1.0, 2.0, PREC).expect("ok");
        let d = BigFloat::from_i64(3, PREC, MODE);
        let r = &z + &d;
        assert!((r.re().to_f64() - 4.0).abs() < 1e-12);
        assert!((r.im().to_f64() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn sub_bigfloat_scalar() {
        let z = BigComplex::from_f64(5.0, 3.0, PREC).expect("ok");
        let d = BigFloat::from_i64(2, PREC, MODE);
        let r = &z - &d;
        assert!((r.re().to_f64() - 3.0).abs() < 1e-12);
        assert!((r.im().to_f64() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn mul_bigfloat_scalar() {
        let z = BigComplex::from_f64(0.0, 1.0, PREC).expect("i");
        let d = BigFloat::from_i64(3, PREC, MODE);
        let r = z * d;
        assert!(r.re().to_f64().abs() < 1e-12);
        assert!((r.im().to_f64() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn sub_reversed_bigfloat_minus_z() {
        // 5 - (3 + 2i) = 2 - 2i
        let z = BigComplex::from_f64(3.0, 2.0, PREC).expect("ok");
        let d = BigFloat::from_i64(5, PREC, MODE);
        let r = d - &z;
        assert!((r.re().to_f64() - 2.0).abs() < 1e-12);
        assert!((r.im().to_f64() + 2.0).abs() < 1e-12);
    }

    #[test]
    fn bigfloat_scalar_assign_ops() {
        let mut z = BigComplex::from_f64(1.0, 2.0, PREC).expect("ok");
        let d3 = BigFloat::from_i64(3, PREC, MODE);
        z += &d3;
        assert!((z.re().to_f64() - 4.0).abs() < 1e-12);
        assert!((z.im().to_f64() - 2.0).abs() < 1e-12);

        let d1 = BigFloat::from_i64(1, PREC, MODE);
        z -= d1;
        assert!((z.re().to_f64() - 3.0).abs() < 1e-12);

        let d2 = BigFloat::from_i64(2, PREC, MODE);
        z *= &d2;
        assert!((z.re().to_f64() - 6.0).abs() < 1e-12);
        assert!((z.im().to_f64() - 4.0).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // Scalar i64 ops
    // -----------------------------------------------------------------------

    #[test]
    fn add_i64_scalar() {
        let z = BigComplex::from_f64(1.0, 2.0, PREC).expect("ok");
        let r = &z + 4i64;
        assert!((r.re().to_f64() - 5.0).abs() < 1e-12);
        assert!((r.im().to_f64() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn sub_i64_scalar() {
        let z = BigComplex::from_f64(5.0, 3.0, PREC).expect("ok");
        let r = z - 2i64;
        assert!((r.re().to_f64() - 3.0).abs() < 1e-12);
        assert!((r.im().to_f64() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn mul_i64_scalar_assign() {
        let mut z = BigComplex::from_f64(1.0, 2.0, PREC).expect("ok");
        z *= 3i64;
        assert!((z.re().to_f64() - 3.0).abs() < 1e-12);
        assert!((z.im().to_f64() - 6.0).abs() < 1e-12);
    }

    #[test]
    fn sub_reversed_i64_minus_z() {
        // 5 - (3 + 2i) = 2 - 2i
        let z = BigComplex::from_f64(3.0, 2.0, PREC).expect("ok");
        let r = 5i64 - &z;
        assert!((r.re().to_f64() - 2.0).abs() < 1e-12);
        assert!((r.im().to_f64() + 2.0).abs() < 1e-12);
    }

    #[test]
    fn i64_scalar_ref_and_owned_consistent() {
        let z = BigComplex::from_f64(2.0, 3.0, PREC).expect("ok");
        let r1 = &z * 4i64;
        let r2 = z.clone() * 4i64;
        assert_eq!(r1, r2);
    }
}
