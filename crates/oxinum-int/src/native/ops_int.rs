//! Operator implementations for [`BigInt`]:
//! `Add`/`Sub`/`Mul`/`Div`/`Rem`/`Neg` (+`*Assign`) for owned and borrowed
//! combinations. All operations preserve the canonical-zero invariant.
//!
//! # Sign of remainder
//!
//! Division truncates toward zero (matching Rust's primitive `/`); the
//! remainder takes the **sign of the dividend** (matching Rust's primitive
//! `%` and `dashu_int::IBig`):
//!
//! ```text
//! a == (a / b) * b + (a % b)         (a != 0, b != 0)
//! sign(a % b) == sign(a)   when a % b != 0
//! ```

use super::int::BigInt;
use super::uint::BigUint;
use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};
use oxinum_core::Sign;

// ---------------------------------------------------------------------------
// Internal pure-data helpers (operate on (Sign, &BigUint) tuples)
// ---------------------------------------------------------------------------

/// Negate a `Sign`.
#[inline]
fn neg_sign(s: Sign) -> Sign {
    match s {
        Sign::Positive => Sign::Negative,
        Sign::Negative => Sign::Positive,
    }
}

/// Multiply two signs (XOR: positive iff both equal).
#[inline]
fn xor_sign(a: Sign, b: Sign) -> Sign {
    if a == b {
        Sign::Positive
    } else {
        Sign::Negative
    }
}

/// Core addition logic: `(sa * |a|) + (sb * |b|)`.
fn add_signed(sa: Sign, a: &BigUint, sb: Sign, b: &BigUint) -> BigInt {
    if sa == sb {
        // Same sign: add magnitudes.
        BigInt::from_parts(sa, a + b)
    } else if a >= b {
        // Different signs, |a| >= |b|: sign of a wins, mag = |a| - |b|.
        let diff = a
            .checked_sub(b)
            .expect("invariant: a >= b ensures non-negative subtraction");
        BigInt::from_parts(sa, diff)
    } else {
        // Different signs, |a| < |b|: sign of b wins, mag = |b| - |a|.
        let diff = b
            .checked_sub(a)
            .expect("invariant: b > a ensures non-negative subtraction");
        BigInt::from_parts(sb, diff)
    }
}

/// Multiplication core. Sign by XOR.
fn mul_signed(sa: Sign, a: &BigUint, sb: Sign, b: &BigUint) -> BigInt {
    BigInt::from_parts(xor_sign(sa, sb), a * b)
}

/// Division core (truncation toward zero). Returns `(quotient, remainder)`.
/// Panics if `b` is zero.
fn divrem_signed(sa: Sign, a: &BigUint, sb: Sign, b: &BigUint) -> (BigInt, BigInt) {
    if b.is_zero() {
        panic!("BigInt: division by zero");
    }
    let (q_mag, r_mag) = super::div::divrem(a, b);
    let q_sign = xor_sign(sa, sb);
    // Remainder sign = sign of dividend when remainder is non-zero.
    let r_sign = sa;
    (
        BigInt::from_parts(q_sign, q_mag),
        BigInt::from_parts(r_sign, r_mag),
    )
}

// ---------------------------------------------------------------------------
// Neg
// ---------------------------------------------------------------------------

impl Neg for BigInt {
    type Output = BigInt;
    #[inline]
    fn neg(self) -> BigInt {
        let (sign, mag) = self.into_parts();
        BigInt::from_parts(neg_sign(sign), mag)
    }
}

impl Neg for &BigInt {
    type Output = BigInt;
    #[inline]
    fn neg(self) -> BigInt {
        BigInt::from_parts(neg_sign(self.sign()), self.magnitude().clone())
    }
}

// ---------------------------------------------------------------------------
// Add
// ---------------------------------------------------------------------------

impl Add<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn add(self, rhs: &BigInt) -> BigInt {
        add_signed(self.sign(), self.magnitude(), rhs.sign(), rhs.magnitude())
    }
}

impl Add<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn add(self, rhs: BigInt) -> BigInt {
        (&self).add(&rhs)
    }
}

impl Add<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn add(self, rhs: &BigInt) -> BigInt {
        (&self).add(rhs)
    }
}

impl Add<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn add(self, rhs: BigInt) -> BigInt {
        self.add(&rhs)
    }
}

impl AddAssign<&BigInt> for BigInt {
    #[inline]
    fn add_assign(&mut self, rhs: &BigInt) {
        *self = (&*self).add(rhs);
    }
}

impl AddAssign<BigInt> for BigInt {
    #[inline]
    fn add_assign(&mut self, rhs: BigInt) {
        *self = (&*self).add(&rhs);
    }
}

// ---------------------------------------------------------------------------
// Sub
// ---------------------------------------------------------------------------

impl Sub<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn sub(self, rhs: &BigInt) -> BigInt {
        // a - b = a + (-b)
        add_signed(
            self.sign(),
            self.magnitude(),
            neg_sign(rhs.sign()),
            rhs.magnitude(),
        )
    }
}

impl Sub<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn sub(self, rhs: BigInt) -> BigInt {
        (&self).sub(&rhs)
    }
}

impl Sub<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn sub(self, rhs: &BigInt) -> BigInt {
        (&self).sub(rhs)
    }
}

impl Sub<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn sub(self, rhs: BigInt) -> BigInt {
        self.sub(&rhs)
    }
}

impl SubAssign<&BigInt> for BigInt {
    #[inline]
    fn sub_assign(&mut self, rhs: &BigInt) {
        *self = (&*self).sub(rhs);
    }
}

impl SubAssign<BigInt> for BigInt {
    #[inline]
    fn sub_assign(&mut self, rhs: BigInt) {
        *self = (&*self).sub(&rhs);
    }
}

// ---------------------------------------------------------------------------
// Mul
// ---------------------------------------------------------------------------

impl Mul<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn mul(self, rhs: &BigInt) -> BigInt {
        mul_signed(self.sign(), self.magnitude(), rhs.sign(), rhs.magnitude())
    }
}

impl Mul<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn mul(self, rhs: BigInt) -> BigInt {
        (&self).mul(&rhs)
    }
}

impl Mul<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn mul(self, rhs: &BigInt) -> BigInt {
        (&self).mul(rhs)
    }
}

impl Mul<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn mul(self, rhs: BigInt) -> BigInt {
        self.mul(&rhs)
    }
}

impl MulAssign<&BigInt> for BigInt {
    #[inline]
    fn mul_assign(&mut self, rhs: &BigInt) {
        *self = (&*self).mul(rhs);
    }
}

impl MulAssign<BigInt> for BigInt {
    #[inline]
    fn mul_assign(&mut self, rhs: BigInt) {
        *self = (&*self).mul(&rhs);
    }
}

// ---------------------------------------------------------------------------
// Div (truncate toward zero, panic on zero divisor)
// ---------------------------------------------------------------------------

impl Div<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn div(self, rhs: &BigInt) -> BigInt {
        divrem_signed(self.sign(), self.magnitude(), rhs.sign(), rhs.magnitude()).0
    }
}

impl Div<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn div(self, rhs: BigInt) -> BigInt {
        (&self).div(&rhs)
    }
}

impl Div<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn div(self, rhs: &BigInt) -> BigInt {
        (&self).div(rhs)
    }
}

impl Div<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn div(self, rhs: BigInt) -> BigInt {
        self.div(&rhs)
    }
}

impl DivAssign<&BigInt> for BigInt {
    #[inline]
    fn div_assign(&mut self, rhs: &BigInt) {
        *self = (&*self).div(rhs);
    }
}

impl DivAssign<BigInt> for BigInt {
    #[inline]
    fn div_assign(&mut self, rhs: BigInt) {
        *self = (&*self).div(&rhs);
    }
}

// ---------------------------------------------------------------------------
// Rem
// ---------------------------------------------------------------------------

impl Rem<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn rem(self, rhs: &BigInt) -> BigInt {
        divrem_signed(self.sign(), self.magnitude(), rhs.sign(), rhs.magnitude()).1
    }
}

impl Rem<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn rem(self, rhs: BigInt) -> BigInt {
        (&self).rem(&rhs)
    }
}

impl Rem<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn rem(self, rhs: &BigInt) -> BigInt {
        (&self).rem(rhs)
    }
}

impl Rem<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn rem(self, rhs: BigInt) -> BigInt {
        self.rem(&rhs)
    }
}

impl RemAssign<&BigInt> for BigInt {
    #[inline]
    fn rem_assign(&mut self, rhs: &BigInt) {
        *self = (&*self).rem(rhs);
    }
}

impl RemAssign<BigInt> for BigInt {
    #[inline]
    fn rem_assign(&mut self, rhs: BigInt) {
        *self = (&*self).rem(&rhs);
    }
}

// ---------------------------------------------------------------------------
// divrem free function (analogous to BigUint's)
// ---------------------------------------------------------------------------

/// Divide-with-remainder for signed `BigInt`. Returns `(quotient, remainder)`
/// where the quotient truncates toward zero and the remainder takes the sign
/// of the dividend.
///
/// # Panics
///
/// Panics if `b` is zero.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{divrem_int, BigInt};
/// let (q, r) = divrem_int(&BigInt::from(-17i64), &BigInt::from(5i64));
/// assert_eq!(q, BigInt::from(-3i64));
/// assert_eq!(r, BigInt::from(-2i64));
/// ```
pub fn divrem_int(a: &BigInt, b: &BigInt) -> (BigInt, BigInt) {
    divrem_signed(a.sign(), a.magnitude(), b.sign(), b.magnitude())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neg_zero_is_canonical() {
        let z = BigInt::zero();
        let nz = -z.clone();
        assert_eq!(nz, z);
        assert_eq!(nz.sign(), Sign::Positive);
    }

    #[test]
    fn add_opposite_sign_to_zero() {
        let a = BigInt::from(42i64);
        let na = -a.clone();
        let sum = &a + &na;
        assert!(sum.is_zero());
        assert_eq!(sum.sign(), Sign::Positive);
    }

    #[test]
    fn sub_basic() {
        let a = BigInt::from(10i64);
        let b = BigInt::from(15i64);
        assert_eq!(&a - &b, BigInt::from(-5i64));
        assert_eq!(&b - &a, BigInt::from(5i64));
    }

    #[test]
    fn mul_sign_table() {
        let p = BigInt::from(6i64);
        let n = BigInt::from(-6i64);
        assert_eq!(&p * &p, BigInt::from(36i64));
        assert_eq!(&p * &n, BigInt::from(-36i64));
        assert_eq!(&n * &p, BigInt::from(-36i64));
        assert_eq!(&n * &n, BigInt::from(36i64));
    }

    #[test]
    fn div_truncates_toward_zero() {
        let cases: &[(i64, i64, i64, i64)] = &[
            (17, 5, 3, 2),    // 17 = 3*5 + 2
            (-17, 5, -3, -2), // -17 = -3*5 + (-2)
            (17, -5, -3, 2),  // 17 = -3*(-5) + 2
            (-17, -5, 3, -2), // -17 = 3*(-5) + (-2)
        ];
        for &(a, b, expected_q, expected_r) in cases {
            let q = &BigInt::from(a) / &BigInt::from(b);
            let r = &BigInt::from(a) % &BigInt::from(b);
            assert_eq!(q, BigInt::from(expected_q), "q mismatch for {a}/{b}");
            assert_eq!(r, BigInt::from(expected_r), "r mismatch for {a}%{b}");
        }
    }

    #[test]
    #[should_panic(expected = "BigInt: division by zero")]
    fn div_by_zero_panics() {
        let _ = &BigInt::from(10i64) / &BigInt::zero();
    }

    #[test]
    fn add_assign_idempotent() {
        let mut a = BigInt::from(10i64);
        a += BigInt::from(5i64);
        assert_eq!(a, BigInt::from(15i64));
        a -= &BigInt::from(7i64);
        assert_eq!(a, BigInt::from(8i64));
        a *= BigInt::from(-2i64);
        assert_eq!(a, BigInt::from(-16i64));
        a /= &BigInt::from(3i64);
        assert_eq!(a, BigInt::from(-5i64));
        a %= BigInt::from(2i64);
        assert_eq!(a, BigInt::from(-1i64));
    }
}
