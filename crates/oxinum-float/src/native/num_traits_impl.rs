//! `num_traits` compatibility for native [`BigFloat`].
//!
//! This module is only compiled when the `num-traits` feature is enabled.
//!
//! # Implemented traits
//!
//! - [`Zero`], [`One`], [`Num`], [`Signed`] — implemented.
//! - [`num_traits::float::FloatConst`] — implemented at `DEFAULT_PREC` = 53 bits
//!   (banker's rounding / `HalfEven`).
//! - [`num_traits::float::TotalOrder`] — implemented; delegates to the inherent
//!   `BigFloat::total_cmp`.
//!
//! # Inapplicable traits
//!
//! `num_traits::Float`, `FloatCore`, and `Real` are **not** implemented for two
//! independent reasons:
//!
//! 1. **`Copy` supertrait bound** — all three traits require `T: Copy`.
//!    `BigFloat` is heap-backed (`mantissa: BigUint` = `Vec<u64>`), hence
//!    non-`Copy`. This is a hard structural incompatibility.
//!
//! 2. **Ill-defined associated values** — `max_value()`, `min_value()`,
//!    `min_positive_value()`, `epsilon()`, and `integer_decode() -> (u64, i16, i8)`
//!    are all meaningless or lossy for an arbitrary-precision float with an unbounded
//!    exponent range and a variable-width mantissa.
//!
//! Use the inherent methods (`is_nan`, `is_infinite`, `is_finite`, `classify`,
//! `total_cmp`, `nan`, `infinity`, `neg_infinity`) and `FloatConst`/`TotalOrder`
//! for the IEEE surface that *is* well-defined at arbitrary precision.
//!
//! Note: `ConstZero`/`ConstOne` are **not** implemented because `BigFloat`
//! tracks an explicit runtime precision and neither value can be constructed
//! in `const` context without a pre-chosen precision.

use std::cmp::Ordering;

use num_traits::{Num, One, Signed, Zero};

use super::constants;
use super::float::{BigFloat, RoundingMode};

/// Default precision (in bits) used by `num_traits` convenience methods.
///
/// Matches the IEEE-754 double-precision mantissa width. Callers that need
/// different precision should construct `BigFloat` directly and use
/// [`BigFloat::with_precision`] to adjust.
const DEFAULT_PREC: u32 = 53;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Parse error for `BigFloat::from_str_radix` via [`num_traits::Num`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBigFloatError(String);

impl std::fmt::Display for ParseBigFloatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseBigFloatError: {}", self.0)
    }
}

impl std::error::Error for ParseBigFloatError {}

// ---------------------------------------------------------------------------
// Zero / One
// ---------------------------------------------------------------------------

impl Zero for BigFloat {
    #[inline]
    fn zero() -> Self {
        BigFloat::zero(DEFAULT_PREC)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        BigFloat::is_zero(self)
    }
}

impl One for BigFloat {
    #[inline]
    fn one() -> Self {
        BigFloat::from_i64(1, DEFAULT_PREC, RoundingMode::HalfEven)
    }
}

// ---------------------------------------------------------------------------
// Num
// ---------------------------------------------------------------------------

impl Num for BigFloat {
    type FromStrRadixErr = ParseBigFloatError;

    /// Parse a decimal string representation as a `BigFloat` at
    /// `DEFAULT_PREC` (53) bits of precision.
    ///
    /// Only radix 10 is fully supported via f64 parsing. For all other radices
    /// this returns an error because native `BigFloat` does not yet have a
    /// general radix-N string parser (planned future milestone).
    fn from_str_radix(s: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        if radix != 10 {
            return Err(ParseBigFloatError(format!(
                "BigFloat::from_str_radix only supports radix 10, got {radix}"
            )));
        }
        let f: f64 = s
            .parse()
            .map_err(|e: std::num::ParseFloatError| ParseBigFloatError(e.to_string()))?;
        BigFloat::from_f64(f, DEFAULT_PREC).map_err(|e| ParseBigFloatError(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Signed
// ---------------------------------------------------------------------------

impl Signed for BigFloat {
    #[inline]
    fn abs(&self) -> Self {
        BigFloat::abs(self)
    }

    fn abs_sub(&self, other: &Self) -> Self {
        if self > other {
            self.clone() - other.clone()
        } else {
            BigFloat::zero(DEFAULT_PREC)
        }
    }

    fn signum(&self) -> Self {
        let s = BigFloat::signum(self);
        BigFloat::from_i64(s as i64, DEFAULT_PREC, RoundingMode::HalfEven)
    }

    fn is_positive(&self) -> bool {
        BigFloat::signum(self) > 0
    }

    fn is_negative(&self) -> bool {
        BigFloat::signum(self) < 0
    }
}

// ---------------------------------------------------------------------------
// FloatConst
// ---------------------------------------------------------------------------

impl num_traits::float::FloatConst for BigFloat {
    fn PI() -> Self {
        constants::pi(DEFAULT_PREC).expect("FloatConst PI: generation at 53 bits is infallible")
    }

    fn E() -> Self {
        constants::e_const(DEFAULT_PREC).expect("FloatConst E: generation at 53 bits is infallible")
    }

    fn LN_2() -> Self {
        constants::ln2(DEFAULT_PREC).expect("FloatConst LN_2: generation at 53 bits is infallible")
    }

    fn LN_10() -> Self {
        let ten = BigFloat::from_i64(10, DEFAULT_PREC, RoundingMode::HalfEven);
        ten.ln(DEFAULT_PREC, RoundingMode::HalfEven)
            .expect("FloatConst LN_10: ln(10) at 53 bits is infallible")
    }

    fn LOG2_E() -> Self {
        let one = BigFloat::from_i64(1, DEFAULT_PREC, RoundingMode::HalfEven);
        let ln2 = constants::ln2(DEFAULT_PREC)
            .expect("FloatConst LOG2_E: ln2 generation at 53 bits is infallible");
        one.div_ref(&ln2)
            .expect("FloatConst LOG2_E: 1/ln2 is infallible (ln2 is nonzero)")
    }

    fn LOG10_E() -> Self {
        let one = BigFloat::from_i64(1, DEFAULT_PREC, RoundingMode::HalfEven);
        let ln10 = Self::LN_10();
        one.div_ref(&ln10)
            .expect("FloatConst LOG10_E: 1/ln10 is infallible (ln10 is nonzero)")
    }

    fn SQRT_2() -> Self {
        BigFloat::from_i64(2, DEFAULT_PREC, RoundingMode::HalfEven)
            .sqrt(DEFAULT_PREC, RoundingMode::HalfEven)
            .expect("FloatConst SQRT_2: sqrt(2) at 53 bits is infallible")
    }

    fn FRAC_1_SQRT_2() -> Self {
        let one = BigFloat::from_i64(1, DEFAULT_PREC, RoundingMode::HalfEven);
        one.div_ref(&Self::SQRT_2())
            .expect("FloatConst FRAC_1_SQRT_2: 1/sqrt(2) is infallible")
    }

    fn FRAC_1_PI() -> Self {
        let one = BigFloat::from_i64(1, DEFAULT_PREC, RoundingMode::HalfEven);
        one.div_ref(&Self::PI())
            .expect("FloatConst FRAC_1_PI: 1/pi is infallible")
    }

    fn FRAC_2_PI() -> Self {
        let two = BigFloat::from_i64(2, DEFAULT_PREC, RoundingMode::HalfEven);
        two.div_ref(&Self::PI())
            .expect("FloatConst FRAC_2_PI: 2/pi is infallible")
    }

    fn FRAC_2_SQRT_PI() -> Self {
        let pi = Self::PI();
        let sqrt_pi = pi
            .sqrt(DEFAULT_PREC, RoundingMode::HalfEven)
            .expect("FloatConst FRAC_2_SQRT_PI: sqrt(pi) is infallible");
        let two = BigFloat::from_i64(2, DEFAULT_PREC, RoundingMode::HalfEven);
        two.div_ref(&sqrt_pi)
            .expect("FloatConst FRAC_2_SQRT_PI: 2/sqrt(pi) is infallible")
    }

    fn FRAC_PI_2() -> Self {
        let two = BigFloat::from_i64(2, DEFAULT_PREC, RoundingMode::HalfEven);
        Self::PI()
            .div_ref(&two)
            .expect("FloatConst FRAC_PI_2: pi/2 is infallible")
    }

    fn FRAC_PI_3() -> Self {
        let three = BigFloat::from_i64(3, DEFAULT_PREC, RoundingMode::HalfEven);
        Self::PI()
            .div_ref(&three)
            .expect("FloatConst FRAC_PI_3: pi/3 is infallible")
    }

    fn FRAC_PI_4() -> Self {
        let four = BigFloat::from_i64(4, DEFAULT_PREC, RoundingMode::HalfEven);
        Self::PI()
            .div_ref(&four)
            .expect("FloatConst FRAC_PI_4: pi/4 is infallible")
    }

    fn FRAC_PI_6() -> Self {
        let six = BigFloat::from_i64(6, DEFAULT_PREC, RoundingMode::HalfEven);
        Self::PI()
            .div_ref(&six)
            .expect("FloatConst FRAC_PI_6: pi/6 is infallible")
    }

    fn FRAC_PI_8() -> Self {
        let eight = BigFloat::from_i64(8, DEFAULT_PREC, RoundingMode::HalfEven);
        Self::PI()
            .div_ref(&eight)
            .expect("FloatConst FRAC_PI_8: pi/8 is infallible")
    }
}

// ---------------------------------------------------------------------------
// TotalOrder
// ---------------------------------------------------------------------------

impl num_traits::float::TotalOrder for BigFloat {
    #[inline]
    fn total_cmp(&self, other: &Self) -> Ordering {
        BigFloat::total_cmp(self, other)
    }
}
