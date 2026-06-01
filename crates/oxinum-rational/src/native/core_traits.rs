//! Core trait implementations for [`BigRational`]:
//! [`OxiNum`], [`OxiSigned`], and [`Pow<u32>`].

use oxinum_core::{OxiNum, OxiSigned, Pow, Sign};
use oxinum_int::native::{BigInt, BigUint};

use super::rational::BigRational;

// ---------------------------------------------------------------------------
// OxiNum
// ---------------------------------------------------------------------------

impl OxiNum for BigRational {
    #[inline]
    fn is_zero(&self) -> bool {
        BigRational::is_zero(self)
    }

    #[inline]
    fn is_one(&self) -> bool {
        BigRational::is_one(self)
    }
}

// ---------------------------------------------------------------------------
// OxiSigned
// ---------------------------------------------------------------------------

impl OxiSigned for BigRational {
    /// Returns the sign of this rational number.
    ///
    /// For zero, returns `Sign::Positive` (canonical convention matching
    /// `BigInt::signum`).
    #[inline]
    fn signum(&self) -> Sign {
        // BigInt::signum() returns Sign directly; zero is Sign::Positive
        // by canonical-zero invariant. The numerator carries the sign.
        self.num.signum()
    }

    #[inline]
    fn abs(&self) -> Self {
        BigRational::abs(self)
    }
}

// ---------------------------------------------------------------------------
// Pow<u32>
// ---------------------------------------------------------------------------

/// Compute `num^exp` for a `BigInt` value.
///
/// Uses the `BigUint::pow` method on the magnitude, then reattaches the
/// sign: negative base raised to an odd exponent stays negative, even
/// exponent is positive.
fn bigint_pow(base: &BigInt, exp: u32) -> BigInt {
    if exp == 0 {
        return BigInt::one();
    }
    let mag_pow = base.magnitude().pow(exp);
    // If exp is odd and base is negative, result is negative.
    if base.sign() == Sign::Negative && exp % 2 == 1 {
        BigInt::from_parts(Sign::Negative, mag_pow)
    } else {
        BigInt::from_parts(Sign::Positive, mag_pow)
    }
}

/// Compute `(num/den)^exp` returning a reduced `BigRational`.
fn rational_pow_u32(r: &BigRational, exp: u32) -> BigRational {
    if exp == 0 {
        return BigRational::one();
    }
    let num_pow: BigInt = bigint_pow(&r.num, exp);
    let den_pow: BigUint = r.den.pow(exp);
    BigRational::from_parts(num_pow, den_pow)
        .expect("rational pow: denominator is always non-zero after exponentiation")
}

impl Pow<u32> for BigRational {
    type Output = BigRational;

    #[inline]
    fn pow(&self, exp: u32) -> BigRational {
        rational_pow_u32(self, exp)
    }
}
