//! Arithmetic operator implementations for [`crate::CBig`].
//!
//! Every binary operator (`Add`, `Sub`, `Mul`, `Div`) provides all four
//! ownership variants — `&T op &T`, `T op T`, `T op &T`, `&T op T` — each
//! producing an owned [`CBig`]. The matching `*Assign` traits are wired up for
//! both owned and borrowed right-hand sides, and [`Neg`] is provided for owned
//! and borrowed receivers.
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
//! # Zero-divisor policy
//!
//! Mirroring every sibling operator in this workspace (e.g.
//! `oxinum_rational::native::BigRational`, whose `Div` panics on a zero
//! divisor while `recip`/constructors return [`OxiNumError::DivByZero`]):
//!
//! * the [`Div`] **operator** panics on a zero divisor. The panic originates
//!   inside `dashu-float`'s `DBig` `/`, which documents that "division by zero
//!   … panic[s] instead of returning infinities" — so this file contains no
//!   explicit `unwrap`/`expect`/`panic!`;
//! * the no-panic [`CBig::checked_div`] returns [`OxiNumError::DivByZero`]
//!   instead.

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_float::DBig;

use crate::CBig;

// ---------------------------------------------------------------------------
// Internal core helpers
// ---------------------------------------------------------------------------

/// Core `Add` body shared by all four ownership variants: component-wise.
#[inline]
fn add_core(a: &CBig, b: &CBig) -> CBig {
    CBig {
        re: &a.re + &b.re,
        im: &a.im + &b.im,
    }
}

/// Core `Sub` body shared by all four ownership variants: component-wise.
#[inline]
fn sub_core(a: &CBig, b: &CBig) -> CBig {
    CBig {
        re: &a.re - &b.re,
        im: &a.im - &b.im,
    }
}

/// Core `Mul` body: `(a_re·b_re − a_im·b_im) + (a_re·b_im + a_im·b_re)·i`.
#[inline]
fn mul_core(a: &CBig, b: &CBig) -> CBig {
    let re = (&a.re * &b.re) - (&a.im * &b.im);
    let im = (&a.re * &b.im) + (&a.im * &b.re);
    CBig { re, im }
}

/// The unreduced numerators of `a / b`, i.e. the real and imaginary parts of
/// `a · conj(b)`. The actual quotient divides each by `|b|²`.
///
/// Returned as `(num_re, num_im)` where
/// `num_re = a_re·b_re + a_im·b_im` and `num_im = a_im·b_re − a_re·b_im`.
/// Shared by [`div_core`] (which adds the explicit zero check) and the
/// [`Div`] operator (which lets `DBig`'s `/` panic on a zero denominator).
#[inline]
fn div_numerators(a: &CBig, b: &CBig) -> (crate::DBig, crate::DBig) {
    let num_re = (&a.re * &b.re) + (&a.im * &b.im);
    let num_im = (&a.im * &b.re) - (&a.re * &b.im);
    (num_re, num_im)
}

/// Core `Div` body with an explicit zero-divisor guard.
///
/// Computes `denom = |b|²`; if `b` is zero returns
/// [`OxiNumError::DivByZero`]. Otherwise `denom` is guaranteed non-zero, so
/// the two `DBig` divisions cannot panic.
fn div_core(a: &CBig, b: &CBig) -> OxiNumResult<CBig> {
    let denom = b.norm_sqr();
    if b.is_zero() {
        return Err(OxiNumError::DivByZero);
    }
    let (num_re, num_im) = div_numerators(a, b);
    Ok(CBig {
        re: &num_re / &denom,
        im: &num_im / &denom,
    })
}

/// Core `Div` body for the operator path: no early zero check.
///
/// When `b` is zero, `denom = |b|² = 0` and the `DBig` `/` panics, exactly as
/// the sibling `BigRational` `/` operator does. This function therefore
/// contains no explicit `unwrap`/`expect`/`panic!`.
#[inline]
fn div_op_core(a: &CBig, b: &CBig) -> CBig {
    let denom = b.norm_sqr();
    let (num_re, num_im) = div_numerators(a, b);
    CBig {
        re: &num_re / &denom,
        im: &num_im / &denom,
    }
}

// ---------------------------------------------------------------------------
// Scalar cores — complex op real scalar, component-wise.
// ---------------------------------------------------------------------------

/// Convert an `i64` to an exact (unlimited-precision) `DBig`.
///
/// `DBig::from(n)` retains only the significant digits needed to render `n`,
/// which causes subsequent arithmetic to round at that narrow precision. Setting
/// precision `0` lifts that cap so that scalar products/sums stay exact.
#[inline]
fn i64_to_exact_dbig(n: i64) -> DBig {
    use oxinum_float::precision::with_precision;
    with_precision(&DBig::from(n), 0)
}

/// `z + d = (re + d, im)`.
#[inline]
fn add_scalar_dbig(z: &CBig, d: &DBig) -> CBig {
    CBig {
        re: &z.re + d,
        im: z.im.clone(),
    }
}

/// `z - d = (re - d, im)`.
#[inline]
fn sub_scalar_dbig(z: &CBig, d: &DBig) -> CBig {
    CBig {
        re: &z.re - d,
        im: z.im.clone(),
    }
}

/// `d - z = (d - re, -im)`.
#[inline]
fn rsub_scalar_dbig(d: &DBig, z: &CBig) -> CBig {
    CBig {
        re: d - &z.re,
        im: -z.im.clone(),
    }
}

/// `z * d = (re*d, im*d)`.
#[inline]
fn mul_scalar_dbig(z: &CBig, d: &DBig) -> CBig {
    CBig {
        re: &z.re * d,
        im: &z.im * d,
    }
}

/// `z / d = (re/d, im/d)` — panics if `d` is zero (mirrors the `CBig` `Div` operator).
#[inline]
fn div_scalar_dbig(z: &CBig, d: &DBig) -> CBig {
    CBig {
        re: &z.re / d,
        im: &z.im / d,
    }
}

/// Scalar `i64` cores — delegate to the `DBig` variants via an exact conversion.
#[inline]
fn add_scalar_i64(z: &CBig, n: i64) -> CBig {
    add_scalar_dbig(z, &i64_to_exact_dbig(n))
}
#[inline]
fn sub_scalar_i64(z: &CBig, n: i64) -> CBig {
    sub_scalar_dbig(z, &i64_to_exact_dbig(n))
}
#[inline]
fn rsub_scalar_i64(n: i64, z: &CBig) -> CBig {
    rsub_scalar_dbig(&i64_to_exact_dbig(n), z)
}
#[inline]
fn mul_scalar_i64(z: &CBig, n: i64) -> CBig {
    mul_scalar_dbig(z, &i64_to_exact_dbig(n))
}
#[inline]
fn div_scalar_i64(z: &CBig, n: i64) -> CBig {
    div_scalar_dbig(z, &i64_to_exact_dbig(n))
}

// ---------------------------------------------------------------------------
// CBig op DBig — Add
// ---------------------------------------------------------------------------

impl Add<DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn add(self, rhs: DBig) -> CBig {
        add_scalar_dbig(&self, &rhs)
    }
}

impl Add<&DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn add(self, rhs: &DBig) -> CBig {
        add_scalar_dbig(&self, rhs)
    }
}

impl Add<DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn add(self, rhs: DBig) -> CBig {
        add_scalar_dbig(self, &rhs)
    }
}

impl Add<&DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn add(self, rhs: &DBig) -> CBig {
        add_scalar_dbig(self, rhs)
    }
}

impl AddAssign<DBig> for CBig {
    #[inline]
    fn add_assign(&mut self, rhs: DBig) {
        *self = add_scalar_dbig(&*self, &rhs);
    }
}

impl AddAssign<&DBig> for CBig {
    #[inline]
    fn add_assign(&mut self, rhs: &DBig) {
        *self = add_scalar_dbig(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// CBig op DBig — Sub
// ---------------------------------------------------------------------------

impl Sub<DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: DBig) -> CBig {
        sub_scalar_dbig(&self, &rhs)
    }
}

impl Sub<&DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: &DBig) -> CBig {
        sub_scalar_dbig(&self, rhs)
    }
}

impl Sub<DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: DBig) -> CBig {
        sub_scalar_dbig(self, &rhs)
    }
}

impl Sub<&DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: &DBig) -> CBig {
        sub_scalar_dbig(self, rhs)
    }
}

impl SubAssign<DBig> for CBig {
    #[inline]
    fn sub_assign(&mut self, rhs: DBig) {
        *self = sub_scalar_dbig(&*self, &rhs);
    }
}

impl SubAssign<&DBig> for CBig {
    #[inline]
    fn sub_assign(&mut self, rhs: &DBig) {
        *self = sub_scalar_dbig(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// DBig op CBig — reversed Sub (d - z)
// ---------------------------------------------------------------------------

impl Sub<CBig> for DBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: CBig) -> CBig {
        rsub_scalar_dbig(&self, &rhs)
    }
}

impl Sub<&CBig> for DBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: &CBig) -> CBig {
        rsub_scalar_dbig(&self, rhs)
    }
}

impl Sub<CBig> for &DBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: CBig) -> CBig {
        rsub_scalar_dbig(self, &rhs)
    }
}

impl Sub<&CBig> for &DBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: &CBig) -> CBig {
        rsub_scalar_dbig(self, rhs)
    }
}

// ---------------------------------------------------------------------------
// CBig op DBig — Mul
// ---------------------------------------------------------------------------

impl Mul<DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn mul(self, rhs: DBig) -> CBig {
        mul_scalar_dbig(&self, &rhs)
    }
}

impl Mul<&DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn mul(self, rhs: &DBig) -> CBig {
        mul_scalar_dbig(&self, rhs)
    }
}

impl Mul<DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn mul(self, rhs: DBig) -> CBig {
        mul_scalar_dbig(self, &rhs)
    }
}

impl Mul<&DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn mul(self, rhs: &DBig) -> CBig {
        mul_scalar_dbig(self, rhs)
    }
}

impl MulAssign<DBig> for CBig {
    #[inline]
    fn mul_assign(&mut self, rhs: DBig) {
        *self = mul_scalar_dbig(&*self, &rhs);
    }
}

impl MulAssign<&DBig> for CBig {
    #[inline]
    fn mul_assign(&mut self, rhs: &DBig) {
        *self = mul_scalar_dbig(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// CBig op DBig — Div
// ---------------------------------------------------------------------------

impl Div<DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn div(self, rhs: DBig) -> CBig {
        div_scalar_dbig(&self, &rhs)
    }
}

impl Div<&DBig> for CBig {
    type Output = CBig;
    #[inline]
    fn div(self, rhs: &DBig) -> CBig {
        div_scalar_dbig(&self, rhs)
    }
}

impl Div<DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn div(self, rhs: DBig) -> CBig {
        div_scalar_dbig(self, &rhs)
    }
}

impl Div<&DBig> for &CBig {
    type Output = CBig;
    #[inline]
    fn div(self, rhs: &DBig) -> CBig {
        div_scalar_dbig(self, rhs)
    }
}

impl DivAssign<DBig> for CBig {
    #[inline]
    fn div_assign(&mut self, rhs: DBig) {
        *self = div_scalar_dbig(&*self, &rhs);
    }
}

impl DivAssign<&DBig> for CBig {
    #[inline]
    fn div_assign(&mut self, rhs: &DBig) {
        *self = div_scalar_dbig(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// CBig op i64 — Add
// ---------------------------------------------------------------------------

impl Add<i64> for CBig {
    type Output = CBig;
    #[inline]
    fn add(self, rhs: i64) -> CBig {
        add_scalar_i64(&self, rhs)
    }
}

impl Add<i64> for &CBig {
    type Output = CBig;
    #[inline]
    fn add(self, rhs: i64) -> CBig {
        add_scalar_i64(self, rhs)
    }
}

impl AddAssign<i64> for CBig {
    #[inline]
    fn add_assign(&mut self, rhs: i64) {
        *self = add_scalar_i64(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// CBig op i64 — Sub
// ---------------------------------------------------------------------------

impl Sub<i64> for CBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: i64) -> CBig {
        sub_scalar_i64(&self, rhs)
    }
}

impl Sub<i64> for &CBig {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: i64) -> CBig {
        sub_scalar_i64(self, rhs)
    }
}

impl SubAssign<i64> for CBig {
    #[inline]
    fn sub_assign(&mut self, rhs: i64) {
        *self = sub_scalar_i64(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// i64 op CBig — reversed Sub (n - z)
// ---------------------------------------------------------------------------

impl Sub<CBig> for i64 {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: CBig) -> CBig {
        rsub_scalar_i64(self, &rhs)
    }
}

impl Sub<&CBig> for i64 {
    type Output = CBig;
    #[inline]
    fn sub(self, rhs: &CBig) -> CBig {
        rsub_scalar_i64(self, rhs)
    }
}

// ---------------------------------------------------------------------------
// CBig op i64 — Mul
// ---------------------------------------------------------------------------

impl Mul<i64> for CBig {
    type Output = CBig;
    #[inline]
    fn mul(self, rhs: i64) -> CBig {
        mul_scalar_i64(&self, rhs)
    }
}

impl Mul<i64> for &CBig {
    type Output = CBig;
    #[inline]
    fn mul(self, rhs: i64) -> CBig {
        mul_scalar_i64(self, rhs)
    }
}

impl MulAssign<i64> for CBig {
    #[inline]
    fn mul_assign(&mut self, rhs: i64) {
        *self = mul_scalar_i64(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// CBig op i64 — Div
// ---------------------------------------------------------------------------

impl Div<i64> for CBig {
    type Output = CBig;
    #[inline]
    fn div(self, rhs: i64) -> CBig {
        div_scalar_i64(&self, rhs)
    }
}

impl Div<i64> for &CBig {
    type Output = CBig;
    #[inline]
    fn div(self, rhs: i64) -> CBig {
        div_scalar_i64(self, rhs)
    }
}

impl DivAssign<i64> for CBig {
    #[inline]
    fn div_assign(&mut self, rhs: i64) {
        *self = div_scalar_i64(&*self, rhs);
    }
}

// ---------------------------------------------------------------------------
// Public no-panic division
// ---------------------------------------------------------------------------

impl CBig {
    /// Divides `self` by `rhs` without panicking.
    ///
    /// Returns [`OxiNumError::DivByZero`] when `rhs` is zero; otherwise yields
    /// `self / rhs` computed as `(self · conj(rhs)) / |rhs|²`.
    ///
    /// This is the panic-free counterpart of the [`Div`] operator (which
    /// panics on a zero divisor, matching the rest of the workspace) and is
    /// the entry point sibling routines such as complex `tan`/`tanh` use.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::CBig;
    ///
    /// let a = CBig::from_f64(1.0, 1.0).expect("finite parts");
    /// let q = a.checked_div(&a).expect("non-zero divisor");
    /// assert_eq!(q.re().to_string(), "1");
    /// assert_eq!(q.im().to_string(), "0");
    ///
    /// assert!(a.checked_div(&CBig::zero()).is_err());
    /// ```
    pub fn checked_div(&self, rhs: &CBig) -> OxiNumResult<CBig> {
        div_core(self, rhs)
    }
}

// ---------------------------------------------------------------------------
// Macros — wire the four owned/borrowed variants and the *Assign traits to the
// *_core helpers.
// ---------------------------------------------------------------------------

macro_rules! impl_binop {
    ($Trait:ident, $method:ident, $core:ident) => {
        impl $Trait<&CBig> for &CBig {
            type Output = CBig;
            #[inline]
            fn $method(self, rhs: &CBig) -> CBig {
                $core(self, rhs)
            }
        }

        impl $Trait<CBig> for CBig {
            type Output = CBig;
            #[inline]
            fn $method(self, rhs: CBig) -> CBig {
                $core(&self, &rhs)
            }
        }

        impl $Trait<&CBig> for CBig {
            type Output = CBig;
            #[inline]
            fn $method(self, rhs: &CBig) -> CBig {
                $core(&self, rhs)
            }
        }

        impl $Trait<CBig> for &CBig {
            type Output = CBig;
            #[inline]
            fn $method(self, rhs: CBig) -> CBig {
                $core(self, &rhs)
            }
        }
    };
}

macro_rules! impl_assign {
    ($Trait:ident, $method:ident, $core:ident) => {
        impl $Trait<&CBig> for CBig {
            #[inline]
            fn $method(&mut self, rhs: &CBig) {
                *self = $core(&*self, rhs);
            }
        }

        impl $Trait<CBig> for CBig {
            #[inline]
            fn $method(&mut self, rhs: CBig) {
                *self = $core(&*self, &rhs);
            }
        }
    };
}

impl_binop!(Add, add, add_core);
impl_binop!(Sub, sub, sub_core);
impl_binop!(Mul, mul, mul_core);
impl_binop!(Div, div, div_op_core);

impl_assign!(AddAssign, add_assign, add_core);
impl_assign!(SubAssign, sub_assign, sub_core);
impl_assign!(MulAssign, mul_assign, mul_core);
impl_assign!(DivAssign, div_assign, div_op_core);

// ---------------------------------------------------------------------------
// Neg
// ---------------------------------------------------------------------------

impl Neg for CBig {
    type Output = CBig;
    #[inline]
    fn neg(self) -> CBig {
        CBig {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl Neg for &CBig {
    type Output = CBig;
    #[inline]
    fn neg(self) -> CBig {
        CBig {
            re: -&self.re,
            im: -&self.im,
        }
    }
}

// ---------------------------------------------------------------------------
// PartialEq (component-wise; no Eq — see crate-level docs)
// ---------------------------------------------------------------------------

impl PartialEq for CBig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.re == other.re && self.im == other.im
    }
}

// ---------------------------------------------------------------------------
// Default
// ---------------------------------------------------------------------------

impl Default for CBig {
    #[inline]
    fn default() -> Self {
        CBig::zero()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `CBig` from two small integer-valued `f64`s.
    fn c(re: f64, im: f64) -> CBig {
        CBig::from_f64(re, im).expect("finite parts")
    }

    /// Assert that `z` equals `re + im·i` via exact decimal string compare.
    fn assert_parts(z: &CBig, re: &str, im: &str) {
        assert_eq!(z.re().to_string(), re, "real part mismatch");
        assert_eq!(z.im().to_string(), im, "imag part mismatch");
    }

    #[test]
    fn add_component_wise() {
        // (1 + 2i) + (3 + 4i) = 4 + 6i
        let sum = c(1.0, 2.0) + c(3.0, 4.0);
        assert_parts(&sum, "4", "6");
    }

    #[test]
    fn sub_component_wise() {
        // (1 + 2i) − (3 + 4i) = -2 − 2i
        let diff = c(1.0, 2.0) - c(3.0, 4.0);
        assert_parts(&diff, "-2", "-2");
    }

    #[test]
    fn mul_hand_computed() {
        // (1 + 2i)(3 + 4i) = (3 − 8) + (4 + 6)i = -5 + 10i
        let prod = c(1.0, 2.0) * c(3.0, 4.0);
        assert_parts(&prod, "-5", "10");
    }

    #[test]
    fn mul_i_squared_is_minus_one() {
        // i · i = -1
        let prod = CBig::i() * CBig::i();
        assert_parts(&prod, "-1", "0");
    }

    #[test]
    fn mul_one_plus_i_squared_is_two_i() {
        // (1 + i)² = 1 + 2i + i² = 2i
        let z = c(1.0, 1.0);
        let sq = &z * &z;
        assert_parts(&sq, "0", "2");
    }

    #[test]
    fn div_self_is_one() {
        // (1 + i) / (1 + i) = 1
        let z = c(1.0, 1.0);
        let q = &z / &z;
        assert_parts(&q, "1", "0");
    }

    #[test]
    fn div_general() {
        // (3 + 4i) / (1 + 2i):
        //   conj denom math → num = (3·1 + 4·2) + (4·1 − 3·2)i = 11 − 2i
        //   |1 + 2i|² = 5  → (11/5) + (-2/5)i = 2.2 − 0.4i
        let q = c(3.0, 4.0) / c(1.0, 2.0);
        assert_parts(&q, "2.2", "-0.4");
    }

    #[test]
    fn checked_div_self_is_one() {
        let z = c(1.0, 1.0);
        let q = z.checked_div(&z).expect("non-zero divisor");
        assert_parts(&q, "1", "0");
    }

    #[test]
    fn checked_div_by_zero_is_err() {
        let z = c(1.0, 1.0);
        // `CBig` intentionally has no `Debug` (provided, if at all, by the
        // sibling `fmt` module), so match on the error variant directly rather
        // than unwrapping the `Ok` payload.
        assert!(matches!(
            z.checked_div(&CBig::zero()),
            Err(OxiNumError::DivByZero)
        ));
    }

    #[test]
    #[should_panic]
    fn div_operator_by_zero_panics() {
        let z = c(1.0, 1.0);
        let _ = z / CBig::zero();
    }

    #[test]
    fn norm_sqr_exact() {
        // |3 + 4i|² = 25 (exact, integer result)
        assert_eq!(c(3.0, 4.0).norm_sqr().to_string(), "25");
    }

    #[test]
    fn default_is_zero() {
        let d = CBig::default();
        assert!(d.is_zero());
        // Exercise `PartialEq` without relying on `Debug` (asserted via the
        // boolean form because `CBig` has no `Debug` in this module).
        assert!(d == CBig::zero());
    }

    #[test]
    fn partial_eq_component_wise() {
        assert!(c(1.5, -2.0) == c(1.5, -2.0));
        assert!(c(1.5, -2.0) != c(1.5, 2.0));
        assert!(c(1.5, -2.0) != c(-1.5, -2.0));
    }

    #[test]
    fn neg_owned_and_borrowed() {
        let z = c(2.0, -3.0);
        let n_ref = -&z;
        assert_parts(&n_ref, "-2", "3");
        let n_owned = -z;
        assert_parts(&n_owned, "-2", "3");
    }

    #[test]
    fn add_assign_accumulates() {
        let mut a = c(1.0, 2.0);
        a += c(3.0, 4.0);
        assert_parts(&a, "4", "6");
        a += &c(-4.0, -6.0);
        assert!(a.is_zero());
    }

    #[test]
    fn mul_assign_updates_in_place() {
        // (1 + i) *= (1 + i) → 2i
        let mut a = c(1.0, 1.0);
        a *= &c(1.0, 1.0);
        assert_parts(&a, "0", "2");
    }

    #[test]
    fn sub_and_div_assign() {
        let mut a = c(5.0, 5.0);
        a -= c(2.0, 1.0);
        assert_parts(&a, "3", "4");
        // (3 + 4i) /= (3 + 4i) → 1
        a /= c(3.0, 4.0);
        assert_parts(&a, "1", "0");
    }

    #[test]
    fn ownership_variants_consistent() {
        // All four flavours of Add agree. Compared via `PartialEq`'s boolean
        // form so the test needs no `Debug` impl for `CBig`.
        let a = c(1.0, 2.0);
        let b = c(3.0, 4.0);
        let target = c(4.0, 6.0);
        assert!(&a + &b == target);
        assert!(a.clone() + b.clone() == target);
        assert!(a.clone() + &b == target);
        assert!(&a + b.clone() == target);
    }

    // -----------------------------------------------------------------------
    // Scalar DBig ops
    // -----------------------------------------------------------------------

    #[test]
    fn add_dbig_scalar() {
        let z = CBig::from_f64(1.0, 2.0).expect("ok");
        let d = DBig::from(3u32);
        let r = z + &d;
        assert_eq!(r.re().to_string(), "4");
        assert_eq!(r.im().to_string(), "2");
    }

    #[test]
    fn mul_dbig_scalar() {
        let z = CBig::i(); // 0 + 1i
        let d = DBig::from(3u32);
        let r = &z * &d;
        assert_eq!(r.re().to_string(), "0");
        assert_eq!(r.im().to_string(), "3");
    }

    #[test]
    fn div_dbig_scalar() {
        let z = CBig::from_f64(6.0, 4.0).expect("ok");
        let d = DBig::from(2u32);
        let r = z / d;
        assert_eq!(r.re().to_string(), "3");
        assert_eq!(r.im().to_string(), "2");
    }

    #[test]
    fn sub_reversed_dbig_minus_z() {
        // 5 - (3 + 2i) = 2 - 2i
        let z = CBig::from_f64(3.0, 2.0).expect("ok");
        let d = DBig::from(5u32);
        let r = d - &z;
        assert_eq!(r.re().to_string(), "2");
        assert_eq!(r.im().to_string(), "-2");
    }

    #[test]
    fn dbig_scalar_assign_ops() {
        // AddAssign
        let mut z = CBig::from_f64(1.0, 2.0).expect("ok");
        z += DBig::from(3u32);
        assert_eq!(z.re().to_string(), "4");
        assert_eq!(z.im().to_string(), "2");

        // SubAssign
        z -= &DBig::from(1u32);
        assert_eq!(z.re().to_string(), "3");

        // MulAssign
        z *= DBig::from(2u32);
        assert_eq!(z.re().to_string(), "6");
        assert_eq!(z.im().to_string(), "4");

        // DivAssign
        z /= &DBig::from(2u32);
        assert_eq!(z.re().to_string(), "3");
        assert_eq!(z.im().to_string(), "2");
    }

    // -----------------------------------------------------------------------
    // Scalar i64 ops
    // -----------------------------------------------------------------------

    #[test]
    fn mul_i64_scalar() {
        let z = CBig::i(); // 0 + 1i
        let r = z * 5i64;
        assert_eq!(r.re().to_string(), "0");
        assert_eq!(r.im().to_string(), "5");
    }

    #[test]
    fn add_i64_scalar_assign() {
        let mut z = CBig::from_f64(1.0, 0.0).expect("ok");
        z += 4i64;
        assert_eq!(z.re().to_string(), "5");
        assert_eq!(z.im().to_string(), "0");
    }

    #[test]
    fn i64_scalar_product_is_exact() {
        // (3+4i) * 5 = 15+20i, exact
        let z: CBig = (3i64, 4i64).into();
        let r = z * 5i64;
        assert_eq!(r.re().to_string(), "15");
        assert_eq!(r.im().to_string(), "20");
    }

    #[test]
    fn sub_i64_scalar() {
        // (5 + 3i) - 2 = 3 + 3i
        let z = CBig::from_f64(5.0, 3.0).expect("ok");
        let r = z - 2i64;
        assert_eq!(r.re().to_string(), "3");
        assert_eq!(r.im().to_string(), "3");
    }

    #[test]
    fn sub_reversed_i64_minus_z() {
        // 5 - (3 + 2i) = 2 - 2i
        let z = CBig::from_f64(3.0, 2.0).expect("ok");
        let r = 5i64 - &z;
        assert_eq!(r.re().to_string(), "2");
        assert_eq!(r.im().to_string(), "-2");
    }

    #[test]
    fn div_i64_scalar() {
        // (6 + 4i) / 2 = 3 + 2i
        let z = CBig::from_f64(6.0, 4.0).expect("ok");
        let r = z / 2i64;
        assert_eq!(r.re().to_string(), "3");
        assert_eq!(r.im().to_string(), "2");
    }

    #[test]
    fn i64_scalar_ref_ops_consistent() {
        // &z * n == z.clone() * n
        let z = CBig::from_f64(2.0, 3.0).expect("ok");
        let r1 = &z * 4i64;
        let r2 = z.clone() * 4i64;
        assert!(r1 == r2);
    }
}
