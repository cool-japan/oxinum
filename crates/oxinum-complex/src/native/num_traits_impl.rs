//! `num_traits` compatibility for native [`BigComplex`].
//!
//! This module is only compiled when the `num-traits` feature is enabled.
//! It implements:
//!
//! - [`num_traits::Zero`], [`num_traits::One`] for `BigComplex`.
//!
//! Note: `Num`, `Signed`, and `Float` are **deliberately not** implemented.
//! The complex field is neither ordered nor signed: there is no order relation
//! compatible with its ring structure (`Signed`/`Float` both require an order
//! and a meaningful sign), and `Num::from_str_radix` presumes a single scalar
//! magnitude rather than a `(re, im)` pair. Those traits are omitted on
//! purpose rather than stubbed.
//!
//! Because `BigComplex` tracks runtime precision per component, the parameter-
//! free [`Zero::zero`] / [`One::one`] constructors materialise their values at
//! `DEFAULT_PREC` = 53 bits (banker's rounding / `HalfEven`), matching the
//! convention used by `oxinum_float::native::BigFloat`'s own `num_traits`
//! impl. `ConstZero`/`ConstOne` are not implemented because neither value can
//! be constructed in `const` context without a pre-chosen precision and a
//! heap-backed mantissa.

use num_traits::{One, Zero};

use crate::native::BigComplex;
use oxinum_float::native::{BigFloat, RoundingMode};

/// Default precision (in bits) used by `num_traits` convenience methods.
///
/// Matches the IEEE-754 double-precision mantissa width and the
/// `DEFAULT_PREC` used by `oxinum_float::native::BigFloat`. Callers that need
/// a different precision should construct `BigComplex` directly via
/// [`BigComplex::zero`] / [`BigComplex::one`].
const DEFAULT_PREC: u32 = 53;

// ---------------------------------------------------------------------------
// Zero / One
// ---------------------------------------------------------------------------

impl Zero for BigComplex {
    #[inline]
    fn zero() -> Self {
        BigComplex::zero(DEFAULT_PREC)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        BigComplex::is_zero(self)
    }
}

impl One for BigComplex {
    #[inline]
    fn one() -> Self {
        BigComplex::one(DEFAULT_PREC, RoundingMode::HalfEven)
    }

    #[inline]
    fn is_one(&self) -> bool {
        let one = BigFloat::from_i64(1, DEFAULT_PREC, RoundingMode::HalfEven);
        let zero = BigFloat::zero(DEFAULT_PREC);
        *self.re() == one && *self.im() == zero
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_zero() {
        let z = <BigComplex as Zero>::zero();
        assert!(z.is_zero());
    }

    #[test]
    fn one_is_one() {
        let one = <BigComplex as One>::one();
        assert!(one.is_one());
        assert_eq!(one.re().to_f64(), 1.0);
        assert_eq!(one.im().to_f64(), 0.0);
    }

    #[test]
    fn imaginary_unit_is_not_one() {
        let imag = BigComplex::i(DEFAULT_PREC, RoundingMode::HalfEven);
        assert!(!imag.is_one());
    }
}
