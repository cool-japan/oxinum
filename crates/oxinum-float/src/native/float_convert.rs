//! Conversions to and from primitive numeric types.
//!
//! Provides:
//!
//! - [`BigFloat::from_i64`] — exact-then-rounded integer encoding.
//! - [`BigFloat::from_f64`] — exact decomposition of an IEEE-754 `f64` value.
//! - [`BigFloat::to_f64`] — IEEE-754-correct, round-to-nearest-even encoding.
//! - [`BigFloat::from_bigint`] — convert a [`BigInt`] to `BigFloat` at specified precision.
//! - [`BigFloat::from_biguint`] — convert a [`BigUint`] to `BigFloat` at specified precision.

use oxinum_core::{OxiNumError, OxiNumResult, Sign};
use oxinum_int::native::{BigInt, BigUint};

use super::float::{BigFloat, FloatClass, RoundingMode};

// ---------------------------------------------------------------------------
// BigFloat → BigInt conversions
// ---------------------------------------------------------------------------

impl BigFloat {
    /// Returns `true` if this value has a non-zero fractional part
    /// (i.e., is not an exact integer).
    ///
    /// Zero and values with `exponent >= 0` are always exact integers.
    fn has_fractional_part(&self) -> bool {
        if self.is_zero() || self.exponent >= 0 {
            return false;
        }
        let shift = (-self.exponent) as u64;
        // fractional bits live in the low `shift` bits of the mantissa.
        // Any of those bits being set means there is a fractional part.
        // Equivalent: trailing_zeros < shift.
        self.mantissa.trailing_zeros() < shift
    }

    /// Returns `true` if the fractional part is exactly 1/2
    /// (the half-bit is set and all lower bits are zero).
    fn half_exactly(&self) -> bool {
        if self.is_zero() || self.exponent >= 0 {
            return false;
        }
        let shift = (-self.exponent) as u64;
        // The half-bit is at position (shift - 1).
        // Exactly 1/2: bit (shift-1) == 1 AND all bits below it are 0.
        // i.e. trailing_zeros == shift - 1.
        self.mantissa.test_bit(shift - 1) && self.mantissa.trailing_zeros() == shift - 1
    }

    /// Returns `true` if the fractional part is strictly greater than 1/2.
    fn more_than_half(&self) -> bool {
        if self.is_zero() || self.exponent >= 0 {
            return false;
        }
        let shift = (-self.exponent) as u64;
        // The half-bit is at position (shift - 1).
        // More than 1/2: bit (shift-1) == 1 AND at least one lower bit is 1.
        // i.e. trailing_zeros < shift - 1.
        self.mantissa.test_bit(shift - 1) && self.mantissa.trailing_zeros() < shift - 1
    }

    /// Convert to [`BigInt`] by truncating toward zero (round-toward-zero).
    ///
    /// Returns the integer part of `self`, discarding any fractional bits.
    ///
    /// Non-finite values (NaN, ±Inf) have no integer representation.
    /// This method returns `BigInt::zero()` as a documented lossy fallback
    /// for those inputs; callers that may encounter non-finite values should
    /// use [`BigFloat::is_finite`] to guard before calling.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::BigFloat;
    /// use oxinum_int::native::BigInt;
    ///
    /// let x = BigFloat::from_f64(3.7, 64).expect("3.7");
    /// assert_eq!(x.to_bigint_trunc(), BigInt::from(3i64));
    ///
    /// let y = BigFloat::from_f64(-3.7, 64).expect("-3.7");
    /// assert_eq!(y.to_bigint_trunc(), BigInt::from(-3i64));
    ///
    /// // Non-finite values return zero (lossy).
    /// assert_eq!(BigFloat::nan(53).to_bigint_trunc(), BigInt::zero());
    /// assert_eq!(BigFloat::infinity(53).to_bigint_trunc(), BigInt::zero());
    /// ```
    pub fn to_bigint_trunc(&self) -> BigInt {
        // Non-finite BigFloat has no meaningful integer representation.
        // Return zero as a documented lossy fallback for NaN and ±Inf.
        if !self.is_finite() {
            return BigInt::zero();
        }
        if self.is_zero() {
            return BigInt::zero();
        }
        if self.exponent >= 0 {
            // No fractional bits: value is mantissa * 2^exponent (integer).
            let mag = self.mantissa.shl_bits(self.exponent as u64);
            return BigInt::from_parts(self.sign, mag);
        }
        // Shift right to drop the fractional part.
        let shift = (-self.exponent) as u64;
        let mag = self.mantissa.shr_bits(shift);
        if mag.is_zero() {
            BigInt::zero()
        } else {
            BigInt::from_parts(self.sign, mag)
        }
    }

    /// Convert to [`BigInt`] by rounding toward negative infinity (floor).
    ///
    /// For negative values with a non-zero fractional part, the result is
    /// one less than the truncation (i.e. more negative).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::BigFloat;
    /// use oxinum_int::native::BigInt;
    ///
    /// let x = BigFloat::from_f64(3.7, 64).expect("3.7");
    /// assert_eq!(x.to_bigint_floor(), BigInt::from(3i64));
    ///
    /// let y = BigFloat::from_f64(-3.7, 64).expect("-3.7");
    /// assert_eq!(y.to_bigint_floor(), BigInt::from(-4i64));
    /// ```
    pub fn to_bigint_floor(&self) -> BigInt {
        let t = self.to_bigint_trunc();
        if self.sign == Sign::Negative && self.has_fractional_part() {
            &t - &BigInt::one()
        } else {
            t
        }
    }

    /// Convert to [`BigInt`] by rounding toward positive infinity (ceiling).
    ///
    /// For positive values with a non-zero fractional part, the result is
    /// one more than the truncation.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::BigFloat;
    /// use oxinum_int::native::BigInt;
    ///
    /// let x = BigFloat::from_f64(3.7, 64).expect("3.7");
    /// assert_eq!(x.to_bigint_ceil(), BigInt::from(4i64));
    ///
    /// let y = BigFloat::from_f64(-3.7, 64).expect("-3.7");
    /// assert_eq!(y.to_bigint_ceil(), BigInt::from(-3i64));
    /// ```
    pub fn to_bigint_ceil(&self) -> BigInt {
        let t = self.to_bigint_trunc();
        if self.sign == Sign::Positive && self.has_fractional_part() {
            &t + &BigInt::one()
        } else {
            t
        }
    }

    /// Convert to [`BigInt`] by rounding half-away-from-zero.
    ///
    /// - Fractional part < 1/2: truncate toward zero.
    /// - Fractional part == 1/2: round away from zero (ties-away).
    /// - Fractional part > 1/2: round away from zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::BigFloat;
    /// use oxinum_int::native::BigInt;
    ///
    /// let x = BigFloat::from_f64(3.7, 64).expect("3.7");
    /// assert_eq!(x.to_bigint_round(), BigInt::from(4i64));
    ///
    /// let y = BigFloat::from_f64(-3.7, 64).expect("-3.7");
    /// assert_eq!(y.to_bigint_round(), BigInt::from(-4i64));
    /// ```
    pub fn to_bigint_round(&self) -> BigInt {
        if !self.has_fractional_part() {
            return self.to_bigint_trunc();
        }
        // If fractional part >= 1/2 (half or more), round away from zero.
        if self.half_exactly() || self.more_than_half() {
            let t = self.to_bigint_trunc();
            if self.sign == Sign::Positive {
                &t + &BigInt::one()
            } else {
                &t - &BigInt::one()
            }
        } else {
            // Fractional part < 1/2: truncate toward zero.
            self.to_bigint_trunc()
        }
    }
}

impl BigFloat {
    /// Encode the integer `n` as a `BigFloat` at `prec` bits of precision.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let a = BigFloat::from_i64(-42, 16, RoundingMode::HalfEven);
    /// assert_eq!(a.to_f64(), -42.0);
    /// ```
    pub fn from_i64(n: i64, prec: u32, mode: RoundingMode) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        if n == 0 {
            return Self::zero(prec);
        }
        let (sign, mag_u64) = if n < 0 {
            // Avoid `-i64::MIN` overflow: take the two's-complement magnitude
            // via wrapping_neg, which is exact in unsigned space.
            (Sign::Negative, (n as i128).unsigned_abs() as u64)
        } else {
            (Sign::Positive, n as u64)
        };
        let mantissa = BigUint::from_u64(mag_u64);
        Self::from_parts(sign, mantissa, 0, prec, mode)
    }

    /// Decompose an IEEE-754 `f64` value into a `BigFloat` at `prec` bits.
    ///
    /// At `prec >= 53` the decomposition is exact (no rounding occurs). The
    /// rounding mode used for any narrowing is [`RoundingMode::HalfEven`].
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::Parse`] if `x` is `NaN` or infinite — native
    /// `BigFloat` does not yet model those special values.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::BigFloat;
    /// let half = BigFloat::from_f64(0.5, 1).expect("0.5 fits in 1 bit");
    /// assert_eq!(half.to_f64(), 0.5);
    /// ```
    pub fn from_f64(x: f64, prec: u32) -> OxiNumResult<Self> {
        assert!(prec > 0, "BigFloat precision must be > 0");
        if x.is_nan() {
            return Err(OxiNumError::Parse("cannot encode NaN as BigFloat".into()));
        }
        if x.is_infinite() {
            return Err(OxiNumError::Parse(
                "cannot encode infinity as BigFloat".into(),
            ));
        }
        if x == 0.0 {
            // Both +0.0 and -0.0 land at canonical zero.
            return Ok(Self::zero(prec));
        }
        // Decode the IEEE-754 layout.
        let bits = x.to_bits();
        let sign_bit = (bits >> 63) & 1;
        let biased_exp = ((bits >> 52) & 0x7FF) as i64;
        let fraction = bits & ((1u64 << 52) - 1);
        let sign = if sign_bit == 1 {
            Sign::Negative
        } else {
            Sign::Positive
        };
        let (mantissa_u64, unbiased_exp) = if biased_exp == 0 {
            // Subnormal: no implicit leading bit, exponent fixed at -1074.
            (fraction, -1074_i64)
        } else {
            // Normal: implicit leading bit at position 52, exponent unbiased.
            (fraction | (1u64 << 52), biased_exp - 1023 - 52)
        };
        let mantissa = BigUint::from_u64(mantissa_u64);
        Ok(Self::from_parts(
            sign,
            mantissa,
            unbiased_exp,
            prec,
            RoundingMode::HalfEven,
        ))
    }

    /// Round to the nearest [`f64`] (ties-to-even). Values whose magnitudes
    /// exceed `f64::MAX` saturate to ±∞, and underflows round to ±0.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let one = BigFloat::from_i64(1, 53, RoundingMode::HalfEven);
    /// assert_eq!(one.to_f64(), 1.0);
    /// ```
    pub fn to_f64(&self) -> f64 {
        // Non-finite values must be handled before the is_zero() check, since
        // NaN and Inf have mantissa=0 and would otherwise fall through to the
        // subnormal path, producing incorrect finite results.
        match self.class {
            FloatClass::Nan => return f64::NAN,
            FloatClass::Infinite => {
                return if self.sign == Sign::Negative {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
            }
            FloatClass::Finite => {}
        }
        if self.is_zero() {
            return 0.0;
        }
        // Pull out the magnitude of the mantissa and the exponent into a
        // representation where the top bit is at position 52 (= number of
        // fraction bits in an f64 normal).
        let bits = self.mantissa.bit_length();
        // Effective binary exponent of the value: exponent + (bit_length - 1)
        // is the position of the top bit relative to the value's "1.xxx" form.
        let top_bit_exp = self.exponent.saturating_add(bits as i64 - 1);
        // f64 normals have unbiased exponent in [-1022, 1023].
        // f64 subnormals have effective top-bit exponent down to -1074.
        if top_bit_exp > 1023 {
            return if self.sign == Sign::Negative {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        if top_bit_exp < -1074 {
            return if self.sign == Sign::Negative {
                -0.0
            } else {
                0.0
            };
        }
        // Strategy: produce a 64-bit representation `m` where the leading 1
        // is at position 52 (or below for subnormal), then assemble the f64.
        //
        // Step 1: Normalize the mantissa into a `desired`-bit integer with
        // round-to-nearest-even. The target precision for the f64 mantissa
        // is `desired_mantissa_bits`:
        //   - normal:    53 bits (top bit at position 52, then we strip the
        //                implicit leading 1).
        //   - subnormal: `top_bit_exp + 1075` bits — the integer mantissa
        //                IS the stored fraction.
        let desired_mantissa_bits: i64 = if top_bit_exp >= -1022 {
            53
        } else {
            // top_bit_exp in [-1074, -1023] => 1..=52 bits.
            top_bit_exp + 1075
        };
        let desired = desired_mantissa_bits as u64;
        let mut tmp = self.clone();
        // Round to `desired` bits (HalfEven).
        tmp.round_to_precision_in_place(desired as u32, RoundingMode::HalfEven);
        // After rounding, the carry-over case can promote a subnormal to
        // the smallest normal. We re-derive `new_top` from the rounded
        // mantissa and re-encode in the appropriate branch.
        if tmp.is_zero() {
            return if self.sign == Sign::Negative {
                -0.0
            } else {
                0.0
            };
        }
        let new_bits = tmp.mantissa.bit_length();
        let new_top = tmp.exponent.saturating_add(new_bits as i64 - 1);
        if new_top > 1023 {
            return if self.sign == Sign::Negative {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        let mantissa_full = match tmp.mantissa.to_u64() {
            Some(m) => m,
            None => {
                return if self.sign == Sign::Negative {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
            }
        };
        let sign_bit: u64 = if self.sign == Sign::Negative { 1 } else { 0 };
        if new_top >= -1022 {
            // Normal. mantissa_full has 53 bits; top bit is the implicit 1,
            // which we strip away.
            let biased_exp = (new_top + 1023) as u64;
            let fraction = mantissa_full & ((1u64 << 52) - 1);
            let bits_out = (sign_bit << 63) | (biased_exp << 52) | fraction;
            f64::from_bits(bits_out)
        } else {
            // Subnormal. The integer mantissa IS the fraction. mantissa_full
            // has `new_top + 1075` bits, which fits in the 52 fraction bits.
            let fraction = mantissa_full & ((1u64 << 52) - 1);
            let bits_out = (sign_bit << 63) | fraction;
            f64::from_bits(bits_out)
        }
    }
}

impl BigFloat {
    /// Convert a [`BigInt`] (signed arbitrary-precision integer) to `BigFloat`
    /// at `prec` bits of precision using the given rounding mode.
    ///
    /// The conversion is exact when `prec >= n.magnitude().bit_length()`;
    /// otherwise the result is rounded according to `mode`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// use oxinum_int::native::BigInt;
    ///
    /// let n = BigInt::from(-42i64);
    /// let f = BigFloat::from_bigint(&n, 64, RoundingMode::HalfEven);
    /// assert_eq!(f.to_f64(), -42.0);
    /// ```
    pub fn from_bigint(n: &BigInt, prec: u32, mode: RoundingMode) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        let sign = n.sign();
        let mag = n.magnitude().clone();
        let f = Self::from_biguint(&mag, prec, mode);
        if sign == Sign::Negative && !f.is_zero() {
            f.neg()
        } else {
            f
        }
    }

    /// Convert a [`BigUint`] (non-negative arbitrary-precision integer) to
    /// `BigFloat` at `prec` bits of precision using the given rounding mode.
    ///
    /// The conversion is exact when `prec >= n.bit_length()`; otherwise the
    /// result is rounded according to `mode`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// use oxinum_int::native::BigUint;
    ///
    /// let n = BigUint::from_u64(1024);
    /// let f = BigFloat::from_biguint(&n, 64, RoundingMode::HalfEven);
    /// assert_eq!(f.to_f64(), 1024.0);
    /// ```
    pub fn from_biguint(n: &BigUint, prec: u32, mode: RoundingMode) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        if n.is_zero() {
            return Self::zero(prec);
        }
        // `from_parts` calls `canonicalize_normalize` then `round_to_precision_in_place`,
        // which handles all the bit-length vs prec casework for us.
        Self::from_parts(Sign::Positive, n.clone(), 0, prec, mode)
    }
}

impl From<i64> for BigFloat {
    /// Encodes `n` at 64 bits of precision with banker's rounding.
    ///
    /// Use [`BigFloat::from_i64`] for explicit precision control.
    fn from(n: i64) -> Self {
        Self::from_i64(n, 64, RoundingMode::HalfEven)
    }
}

impl TryFrom<f64> for BigFloat {
    type Error = OxiNumError;
    fn try_from(x: f64) -> Result<Self, Self::Error> {
        // 53 bits captures every finite double exactly.
        Self::from_f64(x, 53)
    }
}
