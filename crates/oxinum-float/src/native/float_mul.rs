//! Multiplication for `BigFloat`.
//!
//! Strategy: multiply the mantissas as plain [`BigUint`]s (which dispatches to
//! Karatsuba above the threshold and to schoolbook below), sum the exponents
//! (saturating, matching the convention used by addition's
//! `round_to_precision_in_place`), XOR the signs, and feed the parts through
//! [`BigFloat::from_parts`] to land at the canonical form at the target
//! precision `max(p_a, p_b)` under the chosen rounding mode.
//!
//! The default operator (`a * b`) uses [`RoundingMode::HalfEven`] — the same
//! choice the addition operator makes.

use core::ops::{Mul, MulAssign};

use oxinum_core::Sign;

use super::float::{BigFloat, RoundingMode};
use super::nonfinite::{nonfinite_propagate, BinOp};

impl BigFloat {
    /// Return `self * other`, rounding the result to
    /// `max(self.precision, other.precision)` using banker's rounding.
    pub fn mul_ref(&self, other: &BigFloat) -> BigFloat {
        self.mul_ref_with_mode(other, RoundingMode::HalfEven)
    }

    /// Return `self * other`, rounding the result to
    /// `max(self.precision, other.precision)` using the chosen rounding mode.
    pub fn mul_ref_with_mode(&self, other: &BigFloat, mode: RoundingMode) -> BigFloat {
        // Non-finite fast-path: propagate NaN/Inf per IEEE 754 (Inf*0=NaN, etc.).
        if let Some(result) = nonfinite_propagate(self, other, BinOp::Mul) {
            return result;
        }
        let target_prec = self.precision.max(other.precision);
        // Either operand zero -> canonical zero at the target precision.
        if self.is_zero() || other.is_zero() {
            return BigFloat::zero(target_prec);
        }
        // Result sign: equal signs -> Positive, opposite signs -> Negative.
        let out_sign = if self.sign == other.sign {
            Sign::Positive
        } else {
            Sign::Negative
        };
        // Mantissa product — Karatsuba above threshold via BigUint Mul.
        let out_mantissa = &self.mantissa * &other.mantissa;
        // Exponent sum — saturating, mirroring `round_to_precision_in_place`'s
        // approach to far-from-the-bound i64 exponent arithmetic.
        let out_exponent = self.exponent.saturating_add(other.exponent);
        BigFloat::from_parts(out_sign, out_mantissa, out_exponent, target_prec, mode)
    }
}

// ---------------------------------------------------------------------------
// Operator impls — owned and borrowed (matches the pattern in float_add.rs)
// ---------------------------------------------------------------------------

impl Mul<&BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn mul(self, rhs: &BigFloat) -> BigFloat {
        self.mul_ref(rhs)
    }
}

impl Mul<BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn mul(self, rhs: BigFloat) -> BigFloat {
        self.mul_ref(&rhs)
    }
}

impl Mul<&BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn mul(self, rhs: &BigFloat) -> BigFloat {
        self.mul_ref(rhs)
    }
}

impl Mul<BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn mul(self, rhs: BigFloat) -> BigFloat {
        self.mul_ref(&rhs)
    }
}

impl MulAssign<&BigFloat> for BigFloat {
    #[inline]
    fn mul_assign(&mut self, rhs: &BigFloat) {
        *self = self.mul_ref(rhs);
    }
}

impl MulAssign<BigFloat> for BigFloat {
    #[inline]
    fn mul_assign(&mut self, rhs: BigFloat) {
        *self = self.mul_ref(&rhs);
    }
}
