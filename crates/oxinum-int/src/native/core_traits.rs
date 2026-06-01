//! Core trait implementations from `oxinum_core` for the native
//! [`BigUint`] and [`BigInt`] types.
//!
//! This module wires up:
//! - `OxiNum`, `OxiUnsigned`, `OxiSigned`
//! - `FromRadix`, `ToRadix`
//! - `Roots`, `Pow<u32>`
//! - `ModularArithmetic`
//! - `Primality`

use super::int::BigInt;
use super::uint::BigUint;
use super::{mod_arith, primality};
use oxinum_core::{
    FromRadix, ModularArithmetic, OxiNum, OxiNumResult, OxiSigned, OxiUnsigned, Pow, Primality,
    Roots, Sign, ToRadix,
};

// ============================================================================
// BigUint: OxiNum
// ============================================================================

impl OxiNum for BigUint {
    #[inline]
    fn is_zero(&self) -> bool {
        BigUint::is_zero(self)
    }

    #[inline]
    fn is_one(&self) -> bool {
        BigUint::is_one(self)
    }
}

// ============================================================================
// BigUint: OxiUnsigned (marker)
// ============================================================================

impl OxiUnsigned for BigUint {}

// ============================================================================
// BigUint: FromRadix / ToRadix
// ============================================================================

impl FromRadix for BigUint {
    fn from_radix(src: &str, radix: u32) -> OxiNumResult<Self> {
        BigUint::from_str_radix(src, radix)
    }
}

impl ToRadix for BigUint {
    fn to_radix(&self, radix: u32) -> OxiNumResult<String> {
        BigUint::to_radix(self, radix)
    }
}

// ============================================================================
// BigUint: Roots
// ============================================================================

impl Roots for BigUint {
    #[inline]
    fn sqrt(&self) -> Self {
        BigUint::sqrt(self)
    }

    #[inline]
    fn cbrt(&self) -> Self {
        // n=3 never fails in nth_root
        BigUint::nth_root(self, 3).expect("cbrt: n=3 is always valid")
    }

    #[inline]
    fn nth_root(&self, n: u32) -> Self {
        BigUint::nth_root(self, n).expect("nth_root: n must be >= 1")
    }
}

// ============================================================================
// BigUint: Pow<u32>
// ============================================================================

impl Pow<u32> for BigUint {
    type Output = BigUint;

    #[inline]
    fn pow(&self, exp: u32) -> BigUint {
        BigUint::pow(self, exp)
    }
}

// ============================================================================
// BigUint: ModularArithmetic
// ============================================================================

impl ModularArithmetic for BigUint {
    fn mod_add(&self, rhs: &Self, modulus: &Self) -> Self {
        assert!(!modulus.is_zero(), "mod_add: modulus must be non-zero");
        let sum = self + rhs;
        &sum % modulus
    }

    fn mod_sub(&self, rhs: &Self, modulus: &Self) -> Self {
        assert!(!modulus.is_zero(), "mod_sub: modulus must be non-zero");
        if self >= rhs {
            let diff = self - rhs;
            diff % modulus
        } else {
            let diff = rhs - self;
            let r = diff % modulus;
            if r.is_zero() {
                BigUint::ZERO
            } else {
                modulus - &r
            }
        }
    }

    fn mod_mul(&self, rhs: &Self, modulus: &Self) -> Self {
        mod_arith::mod_mul(self, rhs, modulus).expect("mod_mul: modulus must be non-zero")
    }

    fn mod_pow(&self, exp: &Self, modulus: &Self) -> Self {
        mod_arith::mod_pow(self, exp, modulus).expect("mod_pow: modulus must be non-zero")
    }
}

// ============================================================================
// BigUint: Primality
// ============================================================================

impl Primality for BigUint {
    fn is_probably_prime(&self, _witnesses: u32) -> bool {
        // `_witnesses` is ignored — our implementation always uses BPSW/
        // deterministic Miller-Rabin with proven witness sets; it subsumes
        // any user-specified witness count.
        primality::is_probably_prime(self)
    }

    fn next_prime(&self) -> Self {
        // Special case: input <= 1 → first prime is 2.
        if self.is_zero() || self.is_one() {
            return BigUint::from_u64(2);
        }
        // Start the search from self + 1 and iterate until we hit a prime.
        let one = BigUint::from_u64(1);
        let two = BigUint::from_u64(2);
        let mut candidate = self + &one;
        // Ensure we start from an odd number (all even numbers > 2 are composite).
        if !candidate.test_bit(0) {
            candidate += &one;
        }
        loop {
            if primality::is_probably_prime(&candidate) {
                return candidate;
            }
            // Advance by 2 to stay on odd numbers.
            candidate += &two;
        }
    }
}

// ============================================================================
// BigInt: OxiNum
// ============================================================================

impl OxiNum for BigInt {
    #[inline]
    fn is_zero(&self) -> bool {
        BigInt::is_zero(self)
    }

    #[inline]
    fn is_one(&self) -> bool {
        BigInt::is_one(self)
    }
}

// ============================================================================
// BigInt: OxiSigned
// ============================================================================

impl OxiSigned for BigInt {
    #[inline]
    fn signum(&self) -> Sign {
        BigInt::signum(self)
    }

    #[inline]
    fn abs(&self) -> Self {
        BigInt::abs(self)
    }
}

// ============================================================================
// BigInt: FromRadix / ToRadix
// ============================================================================

impl FromRadix for BigInt {
    fn from_radix(src: &str, radix: u32) -> OxiNumResult<Self> {
        // Strip optional leading '-', parse magnitude, then negate if needed.
        let (negative, digits) = match src.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, src),
        };
        let mag = BigUint::from_str_radix(digits, radix)?;
        let sign = if negative && !mag.is_zero() {
            Sign::Negative
        } else {
            Sign::Positive
        };
        Ok(BigInt::from_parts(sign, mag))
    }
}

impl ToRadix for BigInt {
    fn to_radix(&self, radix: u32) -> OxiNumResult<String> {
        let mag_str = self.magnitude().to_radix(radix)?;
        if self.is_negative() {
            Ok(format!("-{mag_str}"))
        } else {
            Ok(mag_str)
        }
    }
}

// ============================================================================
// BigInt: Pow<u32>
// ============================================================================

impl Pow<u32> for BigInt {
    type Output = BigInt;

    fn pow(&self, exp: u32) -> BigInt {
        let mag = BigUint::pow(self.magnitude(), exp);
        // A negative base raised to an odd exponent is negative; otherwise positive.
        let sign = if self.is_negative() && exp % 2 == 1 {
            Sign::Negative
        } else {
            Sign::Positive
        };
        BigInt::from_parts(sign, mag)
    }
}
