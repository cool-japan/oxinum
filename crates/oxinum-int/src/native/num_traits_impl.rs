//! `num_traits` compatibility for native [`BigUint`] and [`BigInt`].
//!
//! This module is only compiled when the `num-traits` feature is enabled.
//! It implements:
//!
//! - [`num_traits::Zero`], [`num_traits::One`], [`num_traits::Num`],
//!   [`num_traits::Unsigned`] for `BigUint`.
//! - [`num_traits::ConstZero`] for `BigUint` (ZERO uses the const `BigUint::ZERO`).
//! - [`num_traits::Zero`], [`num_traits::One`], [`num_traits::Num`],
//!   [`num_traits::Signed`] for `BigInt`.
//! - [`num_traits::ConstZero`] for `BigInt` (ZERO uses the const `BigInt::ZERO`).
//!
//! Note: `ConstOne` is not implemented for either type because the canonical
//! `one()` constructors require heap allocation (`vec![1]`) which is not
//! available in `const` context as of Rust 2024/stable.

use num_traits::{ConstZero, Num, One, Signed, Unsigned, Zero};
use oxinum_core::Sign;

use super::int::BigInt;
use super::uint::BigUint;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Parse error for `BigUint::from_str_radix` via [`num_traits::Num`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBigUintError(String);

impl std::fmt::Display for ParseBigUintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseBigUintError: {}", self.0)
    }
}

impl std::error::Error for ParseBigUintError {}

/// Parse error for `BigInt::from_str_radix` via [`num_traits::Num`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBigIntError(String);

impl std::fmt::Display for ParseBigIntError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseBigIntError: {}", self.0)
    }
}

impl std::error::Error for ParseBigIntError {}

// ---------------------------------------------------------------------------
// BigUint: Zero / One / ConstZero / Num / Unsigned
// ---------------------------------------------------------------------------

impl Zero for BigUint {
    #[inline]
    fn zero() -> Self {
        BigUint::zero()
    }

    #[inline]
    fn is_zero(&self) -> bool {
        BigUint::is_zero(self)
    }
}

impl One for BigUint {
    #[inline]
    fn one() -> Self {
        BigUint::one()
    }

    #[inline]
    fn is_one(&self) -> bool {
        BigUint::is_one(self)
    }
}

impl ConstZero for BigUint {
    const ZERO: Self = BigUint::ZERO;
}

impl Num for BigUint {
    type FromStrRadixErr = ParseBigUintError;

    fn from_str_radix(s: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        BigUint::from_str_radix(s, radix).map_err(|e| ParseBigUintError(e.to_string()))
    }
}

/// `BigUint` is always non-negative, so it satisfies [`Unsigned`].
impl Unsigned for BigUint {}

// ---------------------------------------------------------------------------
// BigInt: Zero / One / ConstZero / Num / Signed
// ---------------------------------------------------------------------------

impl Zero for BigInt {
    #[inline]
    fn zero() -> Self {
        BigInt::zero()
    }

    #[inline]
    fn is_zero(&self) -> bool {
        BigInt::is_zero(self)
    }
}

impl One for BigInt {
    #[inline]
    fn one() -> Self {
        BigInt::one()
    }

    #[inline]
    fn is_one(&self) -> bool {
        BigInt::is_one(self)
    }
}

impl ConstZero for BigInt {
    const ZERO: Self = BigInt::ZERO;
}

impl Num for BigInt {
    type FromStrRadixErr = ParseBigIntError;

    fn from_str_radix(s: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        let (neg, rest) = if let Some(r) = s.strip_prefix('-') {
            (true, r)
        } else if let Some(r) = s.strip_prefix('+') {
            (false, r)
        } else {
            (false, s)
        };
        BigUint::from_str_radix(rest, radix)
            .map(|mag| {
                if neg && !mag.is_zero() {
                    BigInt::from_parts(Sign::Negative, mag)
                } else {
                    BigInt::from_parts(Sign::Positive, mag)
                }
            })
            .map_err(|e| ParseBigIntError(e.to_string()))
    }
}

impl Signed for BigInt {
    #[inline]
    fn abs(&self) -> Self {
        BigInt::abs(self)
    }

    fn abs_sub(&self, other: &Self) -> Self {
        if self > other {
            self.clone() - other.clone()
        } else {
            BigInt::zero()
        }
    }

    fn signum(&self) -> Self {
        if self.is_zero() {
            BigInt::zero()
        } else if BigInt::is_negative(self) {
            -BigInt::one()
        } else {
            BigInt::one()
        }
    }

    #[inline]
    fn is_positive(&self) -> bool {
        BigInt::is_positive(self)
    }

    #[inline]
    fn is_negative(&self) -> bool {
        BigInt::is_negative(self)
    }
}
