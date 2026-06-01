//! Operator implementations for [`BigUint`]:
//! `Add`/`Sub`/`Mul`/`Div`/`Rem` (+`*Assign`) for owned and borrowed
//! combinations.
//!
//! The `Sub` operator panics on underflow (wrapping is not meaningful for an
//! arbitrary-precision unsigned type). Use [`BigUint::checked_sub`] when you
//! need a fallible subtraction.

use super::div::divrem;
use super::mul::mul;
use super::uint::BigUint;

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

// ---------------------------------------------------------------------------
// Add
// ---------------------------------------------------------------------------

impl Add<&BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn add(self, rhs: &BigUint) -> BigUint {
        BigUint::add_ref(self, rhs)
    }
}

impl Add<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn add(self, rhs: BigUint) -> BigUint {
        BigUint::add_ref(&self, &rhs)
    }
}

impl Add<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn add(self, rhs: &BigUint) -> BigUint {
        BigUint::add_ref(&self, rhs)
    }
}

impl Add<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn add(self, rhs: BigUint) -> BigUint {
        BigUint::add_ref(self, &rhs)
    }
}

impl AddAssign<&BigUint> for BigUint {
    #[inline]
    fn add_assign(&mut self, rhs: &BigUint) {
        *self = BigUint::add_ref(self, rhs);
    }
}

impl AddAssign<BigUint> for BigUint {
    #[inline]
    fn add_assign(&mut self, rhs: BigUint) {
        *self = BigUint::add_ref(self, &rhs);
    }
}

// ---------------------------------------------------------------------------
// Sub (panics on underflow — use checked_sub for fallible subtraction)
// ---------------------------------------------------------------------------

impl Sub<&BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn sub(self, rhs: &BigUint) -> BigUint {
        self.checked_sub(rhs)
            .expect("BigUint subtraction underflow")
    }
}

impl Sub<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn sub(self, rhs: BigUint) -> BigUint {
        self.checked_sub(&rhs)
            .expect("BigUint subtraction underflow")
    }
}

impl Sub<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn sub(self, rhs: &BigUint) -> BigUint {
        self.checked_sub(rhs)
            .expect("BigUint subtraction underflow")
    }
}

impl Sub<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn sub(self, rhs: BigUint) -> BigUint {
        self.checked_sub(&rhs)
            .expect("BigUint subtraction underflow")
    }
}

impl SubAssign<&BigUint> for BigUint {
    #[inline]
    fn sub_assign(&mut self, rhs: &BigUint) {
        *self = self
            .checked_sub(rhs)
            .expect("BigUint subtraction underflow");
    }
}

impl SubAssign<BigUint> for BigUint {
    #[inline]
    fn sub_assign(&mut self, rhs: BigUint) {
        *self = self
            .checked_sub(&rhs)
            .expect("BigUint subtraction underflow");
    }
}

// ---------------------------------------------------------------------------
// Mul
// ---------------------------------------------------------------------------

impl Mul<&BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn mul(self, rhs: &BigUint) -> BigUint {
        mul(self, rhs)
    }
}

impl Mul<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn mul(self, rhs: BigUint) -> BigUint {
        mul(&self, &rhs)
    }
}

impl Mul<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn mul(self, rhs: &BigUint) -> BigUint {
        mul(&self, rhs)
    }
}

impl Mul<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn mul(self, rhs: BigUint) -> BigUint {
        mul(self, &rhs)
    }
}

impl MulAssign<&BigUint> for BigUint {
    #[inline]
    fn mul_assign(&mut self, rhs: &BigUint) {
        *self = mul(self, rhs);
    }
}

impl MulAssign<BigUint> for BigUint {
    #[inline]
    fn mul_assign(&mut self, rhs: BigUint) {
        *self = mul(self, &rhs);
    }
}

// ---------------------------------------------------------------------------
// Div / Rem (panic on zero divisor)
// ---------------------------------------------------------------------------

impl Div<&BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn div(self, rhs: &BigUint) -> BigUint {
        divrem(self, rhs).0
    }
}

impl Div<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn div(self, rhs: BigUint) -> BigUint {
        divrem(&self, &rhs).0
    }
}

impl Div<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn div(self, rhs: &BigUint) -> BigUint {
        divrem(&self, rhs).0
    }
}

impl Div<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn div(self, rhs: BigUint) -> BigUint {
        divrem(self, &rhs).0
    }
}

impl DivAssign<&BigUint> for BigUint {
    #[inline]
    fn div_assign(&mut self, rhs: &BigUint) {
        *self = divrem(self, rhs).0;
    }
}

impl DivAssign<BigUint> for BigUint {
    #[inline]
    fn div_assign(&mut self, rhs: BigUint) {
        *self = divrem(self, &rhs).0;
    }
}

impl Rem<&BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn rem(self, rhs: &BigUint) -> BigUint {
        divrem(self, rhs).1
    }
}

impl Rem<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn rem(self, rhs: BigUint) -> BigUint {
        divrem(&self, &rhs).1
    }
}

impl Rem<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn rem(self, rhs: &BigUint) -> BigUint {
        divrem(&self, rhs).1
    }
}

impl Rem<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn rem(self, rhs: BigUint) -> BigUint {
        divrem(self, &rhs).1
    }
}

impl RemAssign<&BigUint> for BigUint {
    #[inline]
    fn rem_assign(&mut self, rhs: &BigUint) {
        *self = divrem(self, rhs).1;
    }
}

impl RemAssign<BigUint> for BigUint {
    #[inline]
    fn rem_assign(&mut self, rhs: BigUint) {
        *self = divrem(self, &rhs).1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_owned_and_borrowed() {
        let a = BigUint::from_u64(10);
        let b = BigUint::from_u64(20);
        assert_eq!(&a + &b, BigUint::from_u64(30));
        assert_eq!(a.clone() + b.clone(), BigUint::from_u64(30));
        assert_eq!(a.clone() + &b, BigUint::from_u64(30));
        assert_eq!(&a + b.clone(), BigUint::from_u64(30));
    }

    #[test]
    fn mul_owned_and_borrowed() {
        let a = BigUint::from_u64(7);
        let b = BigUint::from_u64(6);
        assert_eq!(&a * &b, BigUint::from_u64(42));
        assert_eq!(a.clone() * b.clone(), BigUint::from_u64(42));
    }

    #[test]
    fn div_rem_basic() {
        let a = BigUint::from_u64(100);
        let b = BigUint::from_u64(7);
        assert_eq!(&a / &b, BigUint::from_u64(14));
        assert_eq!(&a % &b, BigUint::from_u64(2));
    }

    #[test]
    fn assign_ops_work() {
        let mut a = BigUint::from_u64(10);
        a += BigUint::from_u64(5);
        assert_eq!(a, BigUint::from_u64(15));
        a *= BigUint::from_u64(3);
        assert_eq!(a, BigUint::from_u64(45));
        a /= BigUint::from_u64(4);
        assert_eq!(a, BigUint::from_u64(11));
        a %= BigUint::from_u64(5);
        assert_eq!(a, BigUint::from_u64(1));
    }

    #[test]
    #[should_panic(expected = "BigUint: division by zero")]
    fn div_by_zero_panics() {
        let _ = BigUint::from_u64(10) / BigUint::zero();
    }

    #[test]
    #[should_panic(expected = "BigUint: division by zero")]
    fn rem_by_zero_panics() {
        let _ = BigUint::from_u64(10) % BigUint::zero();
    }
}
