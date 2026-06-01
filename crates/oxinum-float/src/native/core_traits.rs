//! `OxiNum` and `OxiSigned` trait implementations for native [`BigFloat`].
//!
//! These impls integrate `BigFloat` into the OxiNum trait hierarchy, letting
//! generic code use [`OxiNum::is_zero`], [`OxiNum::is_one`],
//! [`OxiSigned::signum`], and [`OxiSigned::abs`] uniformly over all numeric
//! types in the ecosystem.

use oxinum_core::{OxiNum, OxiSigned, Sign};

use super::float::{BigFloat, RoundingMode};

impl OxiNum for BigFloat {
    /// Returns `true` if this value is the canonical zero.
    ///
    /// Delegates to the inherent [`BigFloat::is_zero`] to avoid any
    /// trait-method ambiguity.
    fn is_zero(&self) -> bool {
        BigFloat::is_zero(self)
    }

    /// Returns `true` if this value is exactly `1`.
    ///
    /// Uses normalized equality: a `BigFloat` at precision `P` represents `1`
    /// when its mantissa encodes `2^(P-1)` and its exponent is `-(P-1)` (the
    /// normalization invariant pins `mantissa.bit_length() == P`).  This is
    /// equivalent to comparing against `BigFloat::from_i64(1, P, HalfEven)`,
    /// which produces the identical normalized representation.
    ///
    /// The comparison must use `self.precision()` so that the constructed `1`
    /// has the same number of mantissa bits — two `BigFloat`s with different
    /// precisions representing the value `1` have different mantissa/exponent
    /// pairs and would compare unequal (precision is excluded from `PartialEq`
    /// on the struct fields, but the mathematical value is the same; the
    /// normalized encoding, however, pins the high bit, so the bit widths must
    /// agree for field-level equality).
    fn is_one(&self) -> bool {
        if BigFloat::is_zero(self) {
            return false;
        }
        self == &BigFloat::from_i64(1, self.precision(), RoundingMode::HalfEven)
    }
}

impl OxiSigned for BigFloat {
    /// Returns the sign of this value as [`Sign::Positive`] or
    /// [`Sign::Negative`].
    ///
    /// Delegates to the inherent `sign()` method (returns `Sign` directly)
    /// rather than the inherent `signum()` (returns `i32`) to avoid any
    /// implicit coercion and to satisfy the trait's `-> Sign` return type.
    fn signum(&self) -> Sign {
        BigFloat::sign(self)
    }

    /// Returns the absolute value (sign forced to [`Sign::Positive`]).
    ///
    /// Delegates to the inherent [`BigFloat::abs`].
    fn abs(&self) -> Self {
        BigFloat::abs(self)
    }
}
