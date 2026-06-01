//! Addition, subtraction, and negation for `BigFloat`.
//!
//! The strategy used here is exact-shift alignment + sign-aware mantissa
//! arithmetic. Because [`BigUint`] is arbitrary-precision, we can always
//! shift the higher-exponent mantissa *left* (no precision loss) so that
//! both operands share the smaller exponent. After the integer add or sub,
//! the result is rounded to the larger of the two operand precisions.

use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use oxinum_core::Sign;
use oxinum_int::native::BigUint;

use super::float::{BigFloat, RoundingMode};
use super::nonfinite::{nonfinite_propagate, BinOp};

impl BigFloat {
    /// Return `self + other`, rounding the result to
    /// `max(self.precision, other.precision)` using banker's rounding.
    pub fn add_ref(&self, other: &BigFloat) -> BigFloat {
        self.add_ref_with_mode(other, RoundingMode::HalfEven)
    }

    /// Return `self + other`, rounding the result to
    /// `max(self.precision, other.precision)` using the chosen rounding mode.
    pub fn add_ref_with_mode(&self, other: &BigFloat, mode: RoundingMode) -> BigFloat {
        // Non-finite fast-path: propagate NaN/Inf per IEEE 754.
        if let Some(result) = nonfinite_propagate(self, other, BinOp::Add) {
            return result;
        }
        let target_prec = self.precision.max(other.precision);
        if self.is_zero() {
            return other.clone().round_to_precision(target_prec, mode);
        }
        if other.is_zero() {
            return self.clone().round_to_precision(target_prec, mode);
        }

        // Step 1: Align to the smaller exponent by shifting the higher-exp
        // mantissa left by exp_diff.
        let (common_exp, lhs_mag, rhs_mag) = align_to_common_exp(self, other);

        // Step 2: Sign-aware add/sub of magnitudes.
        let (out_sign, out_mag) = match (self.sign, other.sign) {
            (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => {
                (self.sign, &lhs_mag + &rhs_mag)
            }
            _ => {
                // Magnitudes will be subtracted: we compute |lhs_mag - rhs_mag|
                // and pick the sign from the larger operand. By construction
                // lhs_mag corresponds to `self` and rhs_mag to `other`.
                if lhs_mag >= rhs_mag {
                    // |self| >= |other| -> sign of self.
                    let diff = lhs_mag.checked_sub(&rhs_mag).unwrap_or_else(BigUint::zero);
                    (self.sign, diff)
                } else {
                    let diff = rhs_mag.checked_sub(&lhs_mag).unwrap_or_else(BigUint::zero);
                    (other.sign, diff)
                }
            }
        };

        // Step 3: Land back at the canonical form at target precision.
        BigFloat::from_parts(out_sign, out_mag, common_exp, target_prec, mode)
    }

    /// Return `self - other` at `max(p_self, p_other)` precision, banker's
    /// rounding.
    pub fn sub_ref(&self, other: &BigFloat) -> BigFloat {
        self.sub_ref_with_mode(other, RoundingMode::HalfEven)
    }

    /// Return `self - other` at `max(p_self, p_other)` precision with the
    /// given rounding mode.
    pub fn sub_ref_with_mode(&self, other: &BigFloat, mode: RoundingMode) -> BigFloat {
        self.add_ref_with_mode(&other.neg(), mode)
    }
}

/// Align two non-zero `BigFloat`s so they share the smaller of their two
/// exponents. Returns `(common_exp, lhs_mantissa, rhs_mantissa)` such that
/// `lhs.value = lhs_mantissa * 2^common_exp` and likewise for `rhs`.
fn align_to_common_exp(a: &BigFloat, b: &BigFloat) -> (i64, BigUint, BigUint) {
    if a.exponent == b.exponent {
        (a.exponent, a.mantissa.clone(), b.mantissa.clone())
    } else if a.exponent > b.exponent {
        let shift = (a.exponent - b.exponent) as u64;
        let lhs = a.mantissa.shl_bits(shift);
        (b.exponent, lhs, b.mantissa.clone())
    } else {
        let shift = (b.exponent - a.exponent) as u64;
        let rhs = b.mantissa.shl_bits(shift);
        (a.exponent, a.mantissa.clone(), rhs)
    }
}

// ---------------------------------------------------------------------------
// Operator impls — owned and borrowed
// ---------------------------------------------------------------------------

impl Add<&BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn add(self, rhs: &BigFloat) -> BigFloat {
        self.add_ref(rhs)
    }
}

impl Add<BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn add(self, rhs: BigFloat) -> BigFloat {
        self.add_ref(&rhs)
    }
}

impl Add<&BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn add(self, rhs: &BigFloat) -> BigFloat {
        self.add_ref(rhs)
    }
}

impl Add<BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn add(self, rhs: BigFloat) -> BigFloat {
        self.add_ref(&rhs)
    }
}

impl Sub<&BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn sub(self, rhs: &BigFloat) -> BigFloat {
        self.sub_ref(rhs)
    }
}

impl Sub<BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn sub(self, rhs: BigFloat) -> BigFloat {
        self.sub_ref(&rhs)
    }
}

impl Sub<&BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn sub(self, rhs: &BigFloat) -> BigFloat {
        self.sub_ref(rhs)
    }
}

impl Sub<BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn sub(self, rhs: BigFloat) -> BigFloat {
        self.sub_ref(&rhs)
    }
}

impl AddAssign<&BigFloat> for BigFloat {
    #[inline]
    fn add_assign(&mut self, rhs: &BigFloat) {
        *self = self.add_ref(rhs);
    }
}

impl AddAssign<BigFloat> for BigFloat {
    #[inline]
    fn add_assign(&mut self, rhs: BigFloat) {
        *self = self.add_ref(&rhs);
    }
}

impl SubAssign<&BigFloat> for BigFloat {
    #[inline]
    fn sub_assign(&mut self, rhs: &BigFloat) {
        *self = self.sub_ref(rhs);
    }
}

impl SubAssign<BigFloat> for BigFloat {
    #[inline]
    fn sub_assign(&mut self, rhs: BigFloat) {
        *self = self.sub_ref(&rhs);
    }
}

impl Neg for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn neg(self) -> BigFloat {
        BigFloat::neg(&self)
    }
}

impl Neg for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn neg(self) -> BigFloat {
        BigFloat::neg(self)
    }
}
