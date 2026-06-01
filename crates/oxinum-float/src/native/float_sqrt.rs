//! Square root for `BigFloat`.
//!
//! Strategy: integer floor square root of a *scaled* mantissa.
//!
//! For a value `v = m * 2^e` (sign positive — sqrt of a negative is
//! returned as a `Domain` error since the native `BigFloat` does not model
//! complex numbers or signed-NaN-like sentinels):
//!
//! ```text
//! sqrt(v) = sqrt(m * 2^e) = sqrt(m) * 2^(e/2)
//! ```
//!
//! The exponent split must be *integer*, so we first force `e` to be even:
//! if `e` is odd, shift the mantissa left by one bit and decrement the
//! exponent — a same-value transform.
//!
//! Then, to get *exactly* `target_precision` (or `target_precision + 1`)
//! significant bits in the integer floor sqrt, we left-shift `m` by `k` bits
//! before calling [`BigUint::sqrt`]. We choose `k` so the scaled mantissa
//! has approximately `2 * target_precision` bits — its integer floor sqrt
//! then has approximately `target_precision` bits.
//!
//! Specifically, with `b = m.bit_length()` and `P = target_precision`:
//!
//! - if `b` is even, choose `k = 2P - b` (even).
//! - if `b` is odd,  choose `k = 2P - b + 1` (even).
//!
//! Both branches keep `k` even — required because the sqrt's exponent
//! denominator is `(e - k) / 2`, which has to land in `i64` without losing
//! a bit. The integer sqrt of the scaled mantissa then has either `P` or
//! `P+1` bits; in either case, [`BigFloat::from_parts`] re-normalizes and
//! rounds to the requested precision under the chosen mode.

use oxinum_core::{OxiNumError, OxiNumResult, Sign};

use super::float::{BigFloat, RoundingMode};

impl BigFloat {
    /// Return `sqrt(self)` at `prec` bits using the chosen rounding mode.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Domain`] if `self < 0` (real-valued sqrt is
    ///   undefined for negative inputs).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let four = BigFloat::from_i64(4, 32, RoundingMode::HalfEven);
    /// let two = four.sqrt(32, RoundingMode::HalfEven).expect("sqrt(4) is real");
    /// assert_eq!(two.to_f64(), 2.0);
    /// ```
    pub fn sqrt(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<Self> {
        assert!(prec > 0, "BigFloat precision must be > 0");

        // --- IEEE-754 non-finite guards ---
        if self.is_nan() {
            return Ok(BigFloat::nan(prec));
        }
        if self.is_infinite() {
            return if self.sign == Sign::Negative {
                Ok(BigFloat::nan(prec)) // sqrt(-Inf) = NaN
            } else {
                Ok(BigFloat::infinity(prec)) // sqrt(+Inf) = +Inf
            };
        }

        if self.is_zero() {
            return Ok(Self::zero(prec));
        }
        if self.sign == Sign::Negative {
            return Err(OxiNumError::Domain(
                "sqrt of negative is undefined for real BigFloat".into(),
            ));
        }

        // --- Step 1: make the exponent even by shifting m left by one bit
        // if needed. Adds one bit of precision to the mantissa, decrements
        // the exponent — value preserved exactly.
        let (mut even_exp, mut work_mantissa, mut work_bits) = {
            let cur_e = self.exponent;
            let cur_bits = self.mantissa.bit_length();
            if cur_e.rem_euclid(2) == 0 {
                (cur_e, self.mantissa.clone(), cur_bits)
            } else {
                // exponent is odd — left-shift mantissa by 1, decrement exp.
                let shifted = self.mantissa.shl_bits(1);
                (cur_e.saturating_sub(1), shifted, cur_bits + 1)
            }
        };
        // After parity-fixup `even_exp` is even and `work_bits` reflects the
        // current mantissa.
        debug_assert_eq!(even_exp.rem_euclid(2), 0);

        // --- Step 2: scale work_mantissa so its bit length is approximately
        // 2*prec. The integer floor sqrt of a 2P-bit value lands in [2^(P-1),
        // 2^P + epsilon], so the result mantissa has bit_length P or P+1.
        let p = prec as u64;
        let target_scaled_bits: u64 = p.saturating_mul(2);
        let extra_shift: u64 = if target_scaled_bits > work_bits {
            let raw = target_scaled_bits - work_bits;
            // Round shift up to the next even number so the exponent
            // halving lands cleanly in i64 — required because final
            // exponent is (even_exp - extra_shift) / 2.
            if raw.rem_euclid(2) == 0 {
                raw
            } else {
                raw + 1
            }
        } else {
            // Mantissa already has more bits than 2*prec — no extra shift
            // needed. The integer sqrt will yield more than `prec` bits
            // which from_parts then rounds down. The extra shift must
            // still be even for clean exponent halving.
            0
        };
        if extra_shift > 0 {
            work_mantissa = work_mantissa.shl_bits(extra_shift);
            work_bits = work_bits.saturating_add(extra_shift);
            even_exp = even_exp.saturating_sub(extra_shift as i64);
        }
        debug_assert_eq!(extra_shift.rem_euclid(2), 0);
        debug_assert_eq!(even_exp.rem_euclid(2), 0);
        let _ = work_bits;

        // --- Step 3: integer floor sqrt of the scaled mantissa.
        let sqrt_mantissa = work_mantissa.sqrt();
        debug_assert!(
            !sqrt_mantissa.is_zero(),
            "non-zero input must yield non-zero sqrt"
        );

        // --- Step 4: the exponent of the sqrt is even_exp / 2.
        let new_exp = even_exp / 2;

        // --- Step 5: land at canonical form at the requested precision.
        Ok(BigFloat::from_parts(
            Sign::Positive,
            sqrt_mantissa,
            new_exp,
            prec,
            mode,
        ))
    }
}
