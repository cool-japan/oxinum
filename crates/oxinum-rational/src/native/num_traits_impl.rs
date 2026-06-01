//! `num_traits` compatibility for native [`BigRational`].
//!
//! This module is only compiled when the `num-traits` feature is enabled.
//! It implements:
//!
//! - [`num_traits::Zero`], [`num_traits::One`], [`num_traits::Num`],
//!   [`num_traits::Signed`] for `BigRational`.
//!
//! Note: `ConstZero`/`ConstOne` are **not** implemented because `BigRational`
//! depends on `BigUint::one()` for its denominator, and that constructor
//! requires a heap allocation (`vec![1]`) which is not available in `const`
//! context.

use num_traits::{Num, One, Signed, Zero};
use oxinum_core::Sign;
use oxinum_int::native::{BigInt, BigUint};

use super::BigRational;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Parse error for `BigRational::from_str_radix` via [`num_traits::Num`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBigRationalError(String);

impl std::fmt::Display for ParseBigRationalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseBigRationalError: {}", self.0)
    }
}

impl std::error::Error for ParseBigRationalError {}

// ---------------------------------------------------------------------------
// Zero / One
// ---------------------------------------------------------------------------

impl Zero for BigRational {
    #[inline]
    fn zero() -> Self {
        BigRational::zero()
    }

    #[inline]
    fn is_zero(&self) -> bool {
        BigRational::is_zero(self)
    }
}

impl One for BigRational {
    #[inline]
    fn one() -> Self {
        BigRational::one()
    }

    #[inline]
    fn is_one(&self) -> bool {
        BigRational::is_one(self)
    }
}

// ---------------------------------------------------------------------------
// Num
// ---------------------------------------------------------------------------

impl Num for BigRational {
    type FromStrRadixErr = ParseBigRationalError;

    /// Parse a rational number string in the form `"n"` or `"n/d"`.
    ///
    /// Only decimal (radix 10) representations are fully supported. Passing
    /// any other radix returns an error; this is an honest limitation that
    /// will be lifted once a general radix-N integer parser is wired through
    /// `BigInt::from_str_radix`.
    fn from_str_radix(s: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        if radix != 10 {
            return Err(ParseBigRationalError(format!(
                "BigRational::from_str_radix only supports radix 10, got {radix}"
            )));
        }
        if let Some(slash_pos) = s.find('/') {
            let num_str = &s[..slash_pos];
            let den_str = &s[slash_pos + 1..];
            let num = parse_bigint(num_str)?;
            let den = parse_biguint(den_str)?;
            BigRational::from_parts(num, den).map_err(|e| ParseBigRationalError(e.to_string()))
        } else {
            let num = parse_bigint(s)?;
            Ok(BigRational::from_integer(num))
        }
    }
}

fn parse_bigint(s: &str) -> Result<BigInt, ParseBigRationalError> {
    let (neg, rest) = if let Some(r) = s.strip_prefix('-') {
        (true, r)
    } else if let Some(r) = s.strip_prefix('+') {
        (false, r)
    } else {
        (false, s)
    };
    parse_biguint(rest).map(|mag| {
        if neg && !mag.is_zero() {
            BigInt::from_parts(Sign::Negative, mag)
        } else {
            BigInt::from_parts(Sign::Positive, mag)
        }
    })
}

fn parse_biguint(s: &str) -> Result<BigUint, ParseBigRationalError> {
    BigUint::from_str_radix(s, 10).map_err(|e| ParseBigRationalError(e.to_string()))
}

// ---------------------------------------------------------------------------
// Signed
// ---------------------------------------------------------------------------

impl Signed for BigRational {
    #[inline]
    fn abs(&self) -> Self {
        BigRational::abs(self)
    }

    fn abs_sub(&self, other: &Self) -> Self {
        if self > other {
            self.clone() - other.clone()
        } else {
            BigRational::zero()
        }
    }

    fn signum(&self) -> Self {
        let s = BigRational::signum(self);
        BigRational::from_integer(BigInt::from(s as i64))
    }

    fn is_positive(&self) -> bool {
        BigRational::signum(self) > 0
    }

    fn is_negative(&self) -> bool {
        BigRational::signum(self) < 0
    }
}
