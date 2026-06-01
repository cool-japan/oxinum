//! Division for `BigFloat`.
//!
//! Strategy: rigorous integer division on scaled mantissas. Conceptually,
//!
//! ```text
//! a / b = (m_a * 2^e_a) / (m_b * 2^e_b)
//!       = (m_a / m_b) * 2^(e_a - e_b)
//! ```
//!
//! `m_a / m_b` is generally not an integer, so to get `target_prec` correct
//! bits in the result mantissa we left-shift `m_a` by `shift = target_prec +
//! guard` bits before integer-dividing — that promotes `guard` extra bits of
//! quotient resolution which the post-division rounding pass then consumes.
//!
//! Result exponent is `e_a - e_b - shift`. We use saturating arithmetic to
//! mirror the convention established in `float_add.rs` /
//! `round_to_precision_in_place`.
//!
//! This is the "simple reference division" path called out in the N4b plan
//! as the rigorous fallback for the Newton-Raphson reciprocal approach.
//! Trade-off: one big integer multiply (the shift) plus one big integer
//! division. For the precisions OxiNum targets (a few hundred to a few
//! thousand bits) this is comfortably fast, and crucially it is exact — no
//! seed-convergence questions to debug at extreme exponents.

use core::ops::{Div, DivAssign, Rem, RemAssign};

use oxinum_core::{OxiNumError, OxiNumResult, Sign};

use super::float::{BigFloat, RoundingMode};
use super::nonfinite::{nonfinite_binop, nonfinite_propagate, BinOp};

/// Extra guard bits used during the quotient computation so the final round
/// has well-defined round/sticky semantics. 8 bits ensures the round-bit and
/// at least 7 sticky bits are present.
const DIV_GUARD_BITS: u32 = 8;

impl BigFloat {
    /// Return `self / other` at `max(p_self, p_other)` precision using
    /// banker's rounding.
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::DivByZero`] if `other` is the canonical zero.
    pub fn div_ref(&self, other: &BigFloat) -> OxiNumResult<BigFloat> {
        self.div_ref_with_mode(other, RoundingMode::HalfEven)
    }

    /// Return `self / other` at `max(p_self, p_other)` precision with the
    /// given rounding mode.
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::DivByZero`] if `other` is the canonical zero.
    pub fn div_ref_with_mode(
        &self,
        other: &BigFloat,
        mode: RoundingMode,
    ) -> OxiNumResult<BigFloat> {
        // Non-finite fast-path: propagate NaN/Inf inputs per IEEE 754.
        // Does NOT generate Inf from finite/0 — that stays as Err(DivByZero).
        if let Some(result) = nonfinite_propagate(self, other, BinOp::Div) {
            return Ok(result);
        }
        if other.is_zero() {
            return Err(OxiNumError::DivByZero);
        }
        let target_prec = self.precision.max(other.precision);
        if self.is_zero() {
            // 0 / nonzero -> canonical zero at the target precision.
            return Ok(BigFloat::zero(target_prec));
        }
        // Result sign: equal signs -> Positive, opposite signs -> Negative.
        let out_sign = if self.sign == other.sign {
            Sign::Positive
        } else {
            Sign::Negative
        };
        // Shift m_a left by `target_prec + guard` bits, then floor-divide
        // by m_b. The quotient carries enough bits for from_parts to round
        // back to `target_prec` correctly.
        let shift_bits = (target_prec + DIV_GUARD_BITS) as u64;
        let scaled = self.mantissa.shl_bits(shift_bits);
        let quotient = &scaled / &other.mantissa;
        // Exponent of the bare integer quotient.
        // value = quotient * 2^(e_a - e_b - shift_bits)
        let exp_after_div = self
            .exponent
            .saturating_sub(other.exponent)
            .saturating_sub(shift_bits as i64);
        // Land at canonical form (normalize + round to target_prec).
        Ok(BigFloat::from_parts(
            out_sign,
            quotient,
            exp_after_div,
            target_prec,
            mode,
        ))
    }
}

// ---------------------------------------------------------------------------
// Operator impls — owned and borrowed.
//
// The trait `Div` returns `Self::Output` (no Result). The operators follow
// IEEE 754: finite/0 → ±Inf, 0/0 → NaN, Inf/Inf → NaN. Callers that need a
// `Result` (and the DivByZero error on finite/0) should call `div_ref` /
// `div_ref_with_mode` directly.
// ---------------------------------------------------------------------------

fn div_ieee(a: &BigFloat, b: &BigFloat) -> BigFloat {
    // Operator path: use nonfinite_binop which generates Inf from finite/0.
    if let Some(result) = nonfinite_binop(a, b, BinOp::Div) {
        return result;
    }
    // Both finite, b is non-zero: call div_ref (normal path).
    a.div_ref(b)
        .expect("div_ieee: finite non-zero divisor guaranteed by nonfinite_binop guard")
}

impl Div<&BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn div(self, rhs: &BigFloat) -> BigFloat {
        div_ieee(self, rhs)
    }
}

impl Div<BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn div(self, rhs: BigFloat) -> BigFloat {
        div_ieee(&self, &rhs)
    }
}

impl Div<&BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn div(self, rhs: &BigFloat) -> BigFloat {
        div_ieee(&self, rhs)
    }
}

impl Div<BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn div(self, rhs: BigFloat) -> BigFloat {
        div_ieee(self, &rhs)
    }
}

impl DivAssign<&BigFloat> for BigFloat {
    #[inline]
    fn div_assign(&mut self, rhs: &BigFloat) {
        *self = div_ieee(self, rhs);
    }
}

impl DivAssign<BigFloat> for BigFloat {
    #[inline]
    fn div_assign(&mut self, rhs: BigFloat) {
        *self = div_ieee(self, &rhs);
    }
}

// ---------------------------------------------------------------------------
// Rem (floating-point remainder: a - trunc(a/b)*b)
//
// This is the IEEE-style truncated remainder (not floor-based).  Follows
// IEEE 754: Inf%y=NaN, x%0=NaN, finite%Inf=finite (returns lhs unchanged),
// NaN propagates.
// ---------------------------------------------------------------------------

fn rem_core(a: &BigFloat, b: &BigFloat) -> BigFloat {
    // IEEE 754 non-finite table for remainder.
    if a.is_nan() || b.is_nan() || a.is_infinite() || b.is_zero() {
        return BigFloat::nan(a.precision.max(b.precision));
    }
    // finite % Inf = the finite value (IEEE rule).
    if b.is_infinite() {
        return a.clone();
    }
    // Both finite, b nonzero.
    if a.is_zero() {
        return BigFloat::zero(a.precision.max(b.precision));
    }
    // quotient = a / b  (exact enough at high precision)
    let prec = a.precision.max(b.precision);
    let q = div_ieee(a, b);
    // Truncate toward zero: drop the fractional part.
    let q_trunc = q.trunc_to_integer(prec, RoundingMode::ToZero);
    // remainder = a - trunc(a/b) * b
    a.clone() - q_trunc * b.clone()
}

impl BigFloat {
    /// Truncate mantissa to an integer value (discard bits below the binary
    /// point). The result has the same precision as `self` rounded to `prec`.
    fn trunc_to_integer(&self, prec: u32, mode: RoundingMode) -> Self {
        if self.is_zero() {
            return BigFloat::zero(prec);
        }
        // The value is  m * 2^exp  where m is the stored mantissa.
        // If exp >= 0 the number is already an integer.
        if self.exponent >= 0 {
            return self.clone().with_precision(prec, mode);
        }
        // exponent < 0: the fractional part occupies |exp| bits of the mantissa.
        let frac_bits = (-self.exponent) as u64;
        let mant_bits = self.mantissa.bit_length();
        if frac_bits >= mant_bits {
            // Entirely fractional — truncates to zero.
            return BigFloat::zero(prec);
        }
        // Drop the low `frac_bits` bits of the mantissa and set exponent = 0.
        let int_mantissa = self.mantissa.shr_bits(frac_bits);
        BigFloat::from_parts(self.sign, int_mantissa, 0, prec, mode)
    }
}

impl Rem<&BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn rem(self, rhs: &BigFloat) -> BigFloat {
        rem_core(self, rhs)
    }
}

impl Rem<BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn rem(self, rhs: BigFloat) -> BigFloat {
        rem_core(&self, &rhs)
    }
}

impl Rem<&BigFloat> for BigFloat {
    type Output = BigFloat;
    #[inline]
    fn rem(self, rhs: &BigFloat) -> BigFloat {
        rem_core(&self, rhs)
    }
}

impl Rem<BigFloat> for &BigFloat {
    type Output = BigFloat;
    #[inline]
    fn rem(self, rhs: BigFloat) -> BigFloat {
        rem_core(self, &rhs)
    }
}

impl RemAssign<&BigFloat> for BigFloat {
    #[inline]
    fn rem_assign(&mut self, rhs: &BigFloat) {
        *self = rem_core(self, rhs);
    }
}

impl RemAssign<BigFloat> for BigFloat {
    #[inline]
    fn rem_assign(&mut self, rhs: BigFloat) {
        *self = rem_core(self, &rhs);
    }
}
