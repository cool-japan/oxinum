//! `num_traits` compatibility for [`CBig`].
//!
//! This module is only compiled when the `num-traits` feature is enabled.
//! It implements:
//!
//! - [`num_traits::Zero`], [`num_traits::One`] for `CBig`.
//!
//! Note: `Num`, `Signed`, and `Float` are **deliberately not** implemented.
//! The complex field is neither ordered nor signed: there is no order relation
//! compatible with its ring structure (`Signed`/`Float` both require an order
//! and a meaningful sign), and `Num::from_str_radix` presumes a single scalar
//! magnitude rather than a `(re, im)` pair. Mirroring the rational crate's
//! note on inapplicable traits, those are omitted on purpose rather than
//! stubbed.
//!
//! `ConstZero`/`ConstOne` are likewise not implemented because `CBig`'s
//! components are heap-backed `DBig` values that cannot be constructed in
//! `const` context.

use num_traits::{One, Zero};

use crate::CBig;
use crate::DBig;

// ---------------------------------------------------------------------------
// Zero / One
// ---------------------------------------------------------------------------

impl Zero for CBig {
    #[inline]
    fn zero() -> Self {
        CBig::zero()
    }

    #[inline]
    fn is_zero(&self) -> bool {
        CBig::is_zero(self)
    }
}

impl One for CBig {
    #[inline]
    fn one() -> Self {
        CBig::one()
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.re == DBig::from(1u32) && self.im == DBig::from(0u32)
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
        let z = <CBig as Zero>::zero();
        assert!(z.is_zero());
    }

    #[test]
    fn one_is_one() {
        let one = <CBig as One>::one();
        assert!(one.is_one());
    }

    #[test]
    fn imaginary_unit_is_not_one() {
        assert!(!CBig::i().is_one());
    }
}
