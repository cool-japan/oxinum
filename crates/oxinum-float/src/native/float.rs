//! Native `BigFloat` — binary-base arbitrary-precision floating-point
//! number built as `(sign, mantissa, exponent, precision)`.
//!
//! The value of a `BigFloat` is
//!
//! ```text
//! (-1)^sign * mantissa * 2^exponent
//! ```
//!
//! where `mantissa` is a non-negative [`BigUint`] and `exponent` is a signed
//! 64-bit integer.
//!
//! # Invariants
//!
//! Every public constructor and arithmetic operation re-establishes the
//! following invariants:
//!
//! 1. `precision > 0`.
//! 2. If `mantissa.is_zero()` then the value is the canonical zero at
//!    `precision`: `{ Positive, 0, 0, precision }`.
//! 3. If `!mantissa.is_zero()` then the mantissa is *normalized*:
//!    `mantissa.bit_length() == precision` (the top bit is set).
//!
//! Two non-zero `BigFloat` values compare equal iff their `(sign, mantissa,
//! exponent)` triples match. **Precision is excluded from equality**: it is
//! a representation knob, not part of the mathematical value. A zero at
//! precision 10 equals a zero at precision 50.

use core::cmp::Ordering;
use core::fmt;

use oxinum_core::Sign;
use oxinum_int::native::BigUint;

/// Classification of a `BigFloat` value: finite, infinite, or NaN.
///
/// The sign of ±Inf is carried by the `BigFloat::sign` field. NaN has a
/// single canonical form (`sign = Positive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FloatClass {
    #[default]
    Finite,
    Infinite,
    Nan,
}

/// Rounding modes for native `BigFloat` arithmetic.
///
/// Mirrors the set of rounding policies natively supported by the binary
/// `BigFloat` core. The seven variants cover all IEEE-754 directed and
/// nearest-tie-break combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoundingMode {
    /// Round half to even (banker's rounding).
    HalfEven,
    /// Round half away from zero.
    HalfAway,
    /// Round half toward zero (truncate ties).
    HalfToZero,
    /// Truncate toward zero (drop fractional bits).
    ToZero,
    /// Round toward `+∞`.
    ToInf,
    /// Round toward `-∞`.
    ToNegInf,
    /// Round away from zero (round up in magnitude).
    AwayFromZero,
}

/// Native arbitrary-precision binary float.
///
/// `BigFloat` represents `(-1)^sign * mantissa * 2^exponent` with `precision`
/// significant bits. See the module-level documentation for the full
/// invariant list.
///
/// # Examples
///
/// ```
/// use oxinum_float::native::{BigFloat, RoundingMode};
///
/// let a = BigFloat::from_i64(3, 8, RoundingMode::HalfEven);
/// let b = BigFloat::from_i64(5, 8, RoundingMode::HalfEven);
/// let sum = &a + &b;
/// assert_eq!(sum.to_f64(), 8.0);
/// ```
#[derive(Clone)]
pub struct BigFloat {
    pub(crate) class: FloatClass,
    pub(crate) sign: Sign,
    pub(crate) mantissa: BigUint,
    pub(crate) exponent: i64,
    pub(crate) precision: u32,
}

impl BigFloat {
    /// Canonical zero at `prec` bits of precision.
    ///
    /// # Panics
    ///
    /// Panics if `prec == 0` (the precision invariant requires `prec > 0`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::BigFloat;
    /// let z = BigFloat::zero(53);
    /// assert!(z.is_zero());
    /// assert_eq!(z.precision(), 53);
    /// ```
    pub fn zero(prec: u32) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        Self {
            class: FloatClass::Finite,
            sign: Sign::Positive,
            mantissa: BigUint::zero(),
            exponent: 0,
            precision: prec,
        }
    }

    /// Create a canonical NaN at `prec` bits. NaN's sign is always `Positive`.
    pub fn nan(prec: u32) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        Self {
            class: FloatClass::Nan,
            sign: Sign::Positive,
            mantissa: BigUint::zero(),
            exponent: 0,
            precision: prec,
        }
    }

    /// Create positive infinity (`+∞`) at `prec` bits.
    pub fn infinity(prec: u32) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        Self {
            class: FloatClass::Infinite,
            sign: Sign::Positive,
            mantissa: BigUint::zero(),
            exponent: 0,
            precision: prec,
        }
    }

    /// Create negative infinity (`−∞`) at `prec` bits.
    pub fn neg_infinity(prec: u32) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        Self {
            class: FloatClass::Infinite,
            sign: Sign::Negative,
            mantissa: BigUint::zero(),
            exponent: 0,
            precision: prec,
        }
    }

    /// Construct directly from already-validated parts.
    ///
    /// The result is normalized (trailing-zero bits migrated into the
    /// exponent) and then rounded to `prec` bits if the normalized mantissa
    /// is wider than `prec`. If the normalized mantissa is narrower, it is
    /// left-padded so `mantissa.bit_length() == prec` while preserving the
    /// mathematical value.
    ///
    /// Used by every higher-level constructor (`from_i64`, `from_f64`,
    /// arithmetic) — call this whenever you need to land back at the
    /// canonical invariant from arbitrary parts.
    pub fn from_parts(
        sign: Sign,
        mantissa: BigUint,
        exponent: i64,
        prec: u32,
        mode: RoundingMode,
    ) -> Self {
        assert!(prec > 0, "BigFloat precision must be > 0");
        if mantissa.is_zero() {
            return Self::zero(prec);
        }
        let mut out = Self {
            class: FloatClass::Finite,
            sign,
            mantissa,
            exponent,
            precision: prec,
        };
        out.canonicalize_normalize();
        out.round_to_precision_in_place(prec, mode);
        out
    }

    /// Returns the precision in bits.
    #[inline]
    pub fn precision(&self) -> u32 {
        self.precision
    }

    /// Returns the sign.
    ///
    /// For the canonical zero, the sign is always [`Sign::Positive`].
    #[inline]
    pub fn sign(&self) -> Sign {
        self.sign
    }

    /// Returns a reference to the mantissa.
    #[inline]
    pub fn mantissa(&self) -> &BigUint {
        &self.mantissa
    }

    /// Returns the binary exponent (the power of 2 the mantissa is scaled by).
    #[inline]
    pub fn exponent(&self) -> i64 {
        self.exponent
    }

    /// Returns `true` if this value is the canonical zero.
    ///
    /// NaN and Inf have `mantissa = 0` internally, so this check requires
    /// testing the `class` field first.
    #[inline]
    pub fn is_zero(&self) -> bool {
        matches!(self.class, FloatClass::Finite) && self.mantissa.is_zero()
    }

    /// Returns `true` if this value is finite (not NaN or Inf).
    #[inline]
    pub fn is_finite(&self) -> bool {
        matches!(self.class, FloatClass::Finite)
    }

    /// Returns `true` if this value is infinite (`+∞` or `−∞`).
    #[inline]
    pub fn is_infinite(&self) -> bool {
        matches!(self.class, FloatClass::Infinite)
    }

    /// Returns `true` if this value is NaN.
    #[inline]
    pub fn is_nan(&self) -> bool {
        matches!(self.class, FloatClass::Nan)
    }

    /// Returns `true` for finite non-zero values.
    ///
    /// Arbitrary-precision floats have no `Subnormal` category — every nonzero
    /// finite value is `Normal`.
    #[inline]
    pub fn is_normal(&self) -> bool {
        matches!(self.class, FloatClass::Finite) && !self.mantissa.is_zero()
    }

    /// Returns the IEEE 754 float class.
    ///
    /// `FpCategory::Subnormal` is never returned: there is no fixed exponent
    /// range in arbitrary-precision arithmetic, so every nonzero finite value
    /// is `Normal`.
    pub fn classify(&self) -> core::num::FpCategory {
        use core::num::FpCategory;
        match self.class {
            FloatClass::Nan => FpCategory::Nan,
            FloatClass::Infinite => FpCategory::Infinite,
            FloatClass::Finite if self.mantissa.is_zero() => FpCategory::Zero,
            FloatClass::Finite => FpCategory::Normal,
        }
    }

    /// Returns `true` for positive and NaN values (NaN has canonical positive sign).
    ///
    /// Note: unlike `f64`, the single canonical zero has `is_sign_positive() == true`.
    #[inline]
    pub fn is_sign_positive(&self) -> bool {
        self.sign == Sign::Positive
    }

    /// Returns `true` only for negative-sign values (negative Inf or negative finite).
    ///
    /// Note: canonical zero has `is_sign_negative() == false` (no signed zero).
    #[inline]
    pub fn is_sign_negative(&self) -> bool {
        self.sign == Sign::Negative
    }

    /// Returns `-1`, `0`, or `+1` depending on the sign of the value.
    pub fn signum(&self) -> i32 {
        if self.is_zero() {
            0
        } else if self.sign == Sign::Negative {
            -1
        } else {
            1
        }
    }

    /// Returns the absolute value (sign forced to [`Sign::Positive`]).
    pub fn abs(&self) -> Self {
        let mut out = self.clone();
        out.sign = Sign::Positive;
        out
    }

    /// Returns the additive inverse.
    ///
    /// Negating the canonical zero yields the canonical zero (sign stays
    /// `Positive`). Negating NaN returns NaN unchanged (the canonical NaN
    /// always has sign `Positive`).
    pub fn neg(&self) -> Self {
        // NaN: canonical NaN is always sign-positive; negating NaN returns NaN unchanged.
        if self.is_nan() {
            return self.clone();
        }
        if self.is_zero() {
            return self.clone();
        }
        let mut out = self.clone();
        out.sign = match self.sign {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
        };
        out
    }

    /// Change the precision, rounding the mantissa with `mode` if narrowing.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::native::{BigFloat, RoundingMode};
    /// let a = BigFloat::from_i64(7, 8, RoundingMode::HalfEven);
    /// let b = a.with_precision(64, RoundingMode::HalfEven);
    /// assert_eq!(b.precision(), 64);
    /// assert_eq!(b.to_f64(), 7.0);
    /// ```
    #[must_use]
    pub fn with_precision(self, prec: u32, mode: RoundingMode) -> Self {
        self.round_to_precision(prec, mode)
    }

    /// Round the mantissa so that, after normalization, it has exactly `prec`
    /// significant bits.
    ///
    /// If the current mantissa has fewer bits than `prec`, the result is
    /// left-padded; the mathematical value is unchanged. If it has more bits,
    /// the low bits are discarded according to `mode`.
    #[must_use]
    pub fn round_to_precision(mut self, prec: u32, mode: RoundingMode) -> Self {
        self.round_to_precision_in_place(prec, mode);
        self
    }

    /// In-place version of [`Self::round_to_precision`].
    pub(crate) fn round_to_precision_in_place(&mut self, prec: u32, mode: RoundingMode) {
        assert!(prec > 0, "BigFloat precision must be > 0");
        // Non-finite values (NaN, ±Inf) have no mantissa to round; just update precision.
        if !self.is_finite() {
            self.precision = prec;
            return;
        }
        self.precision = prec;
        if self.mantissa.is_zero() {
            // Canonical zero — nothing to round, just adopt the new precision.
            self.sign = Sign::Positive;
            self.exponent = 0;
            return;
        }
        // Strip trailing zero bits into the exponent so the operation is
        // performed on the unique normalized representation.
        self.absorb_trailing_zeros();

        let cur_bits = self.mantissa.bit_length();
        let target = prec as u64;
        match cur_bits.cmp(&target) {
            Ordering::Less => {
                // Pad left — value preserved exactly.
                let shift = target - cur_bits;
                self.mantissa = self.mantissa.shl_bits(shift);
                // Underflow guard: shifting left lowers exponent.
                // i64::MIN minus a positive number would saturate, but for
                // realistic precisions (prec <= u32::MAX) this is far from
                // the boundary. Defensive saturating sub keeps us safe.
                self.exponent = self.exponent.saturating_sub(shift as i64);
            }
            Ordering::Equal => { /* already normalized */ }
            Ordering::Greater => {
                let drop = cur_bits - target;
                self.round_drop_low_bits(drop, mode);
            }
        }
        debug_assert!(
            self.mantissa.is_zero() || self.mantissa.bit_length() == self.precision as u64,
            "BigFloat normalize invariant violated after round_to_precision",
        );
        debug_assert!(
            !self.mantissa.is_zero() || self.sign == Sign::Positive,
            "BigFloat canonical-zero invariant violated",
        );
    }

    // -----------------------------------------------------------------------
    // Internal: invariant-establishing helpers
    // -----------------------------------------------------------------------

    /// Strip trailing-zero bits from `mantissa` and add their count to
    /// `exponent`. No-op when the mantissa is zero.
    pub(crate) fn absorb_trailing_zeros(&mut self) {
        if self.mantissa.is_zero() {
            return;
        }
        let tz = self.mantissa.trailing_zeros();
        if tz > 0 {
            self.mantissa = self.mantissa.shr_bits(tz);
            // Adding a non-negative value to a possibly negative exponent.
            // Defensive saturating_add keeps us safe against pathological
            // mantissas with billions of trailing zeros.
            self.exponent = self.exponent.saturating_add(tz as i64);
        }
    }

    /// Canonicalize by stripping trailing zeros and (if needed) padding the
    /// mantissa to `self.precision` bits. After this call, either
    /// `mantissa.is_zero()` (and the value is canonical zero) or
    /// `mantissa.bit_length() == self.precision`.
    ///
    /// Does **not** round: assumes the caller is willing to grow the mantissa
    /// to the requested precision exactly.
    pub(crate) fn canonicalize_normalize(&mut self) {
        if self.mantissa.is_zero() {
            self.sign = Sign::Positive;
            self.exponent = 0;
            return;
        }
        self.absorb_trailing_zeros();
        let cur_bits = self.mantissa.bit_length();
        let target = self.precision as u64;
        if cur_bits < target {
            let shift = target - cur_bits;
            self.mantissa = self.mantissa.shl_bits(shift);
            self.exponent = self.exponent.saturating_sub(shift as i64);
        }
        // If cur_bits > target the caller (round_to_precision_in_place) handles
        // the truncation; otherwise we have an exact normalized representation.
    }

    /// Drop the `drop` least-significant bits of `mantissa`, applying the
    /// chosen rounding mode. Updates `exponent` accordingly. The result is
    /// then re-normalized to satisfy the precision invariant.
    fn round_drop_low_bits(&mut self, drop: u64, mode: RoundingMode) {
        debug_assert!(drop > 0);
        // Split mantissa = quotient * 2^drop + remainder.
        // quotient = mantissa >> drop
        // round_bit = bit (drop-1) of original mantissa
        // sticky    = OR of bits 0..(drop-1) of original mantissa
        let round_bit = self.mantissa.test_bit(drop - 1);
        // Sticky = does any lower bit (below round_bit) survive?
        let sticky = if drop >= 2 {
            // Detect via comparing mantissa to (quotient << drop | round_bit << (drop-1))
            // Cheaper: a value with the bottom (drop-1) bits zeroed is just
            // (mantissa >> (drop-1)) << (drop-1). If that doesn't equal mantissa
            // when restricted below the round bit, we have stickiness.
            // Concretely: sticky iff mantissa.trailing_zeros() < drop - 1.
            (self.mantissa.trailing_zeros()) < (drop - 1)
        } else {
            false
        };
        let mut quotient = self.mantissa.shr_bits(drop);
        // Determine increment based on mode.
        let negative = self.sign == Sign::Negative;
        let increment = match mode {
            RoundingMode::ToZero => false,
            RoundingMode::AwayFromZero => round_bit || sticky,
            RoundingMode::ToInf => !negative && (round_bit || sticky),
            RoundingMode::ToNegInf => negative && (round_bit || sticky),
            RoundingMode::HalfAway => round_bit,
            RoundingMode::HalfToZero => round_bit && sticky,
            RoundingMode::HalfEven => {
                if !round_bit {
                    false
                } else if sticky {
                    true
                } else {
                    // Exact half — round to even (LSB of quotient = 0).
                    quotient.test_bit(0)
                }
            }
        };
        if increment {
            let one = BigUint::one();
            quotient = &quotient + &one;
        }
        self.exponent = self.exponent.saturating_add(drop as i64);
        self.mantissa = quotient;
        if self.mantissa.is_zero() {
            // Whole number rounded to zero (e.g. all bits below round bit
            // forming an exact half rounding toward zero). Canonical zero
            // recapture.
            self.sign = Sign::Positive;
            self.exponent = 0;
            return;
        }
        // Re-normalize: rounding may have grown the mantissa by one bit
        // (carry on increment) or, in pathological cases, leave a sub-target
        // bit width.
        let cur_bits = self.mantissa.bit_length();
        let target = self.precision as u64;
        match cur_bits.cmp(&target) {
            Ordering::Equal => {}
            Ordering::Greater => {
                // Carry overflowed by exactly one bit. Shift right one and
                // bump the exponent.
                let extra = cur_bits - target;
                debug_assert_eq!(
                    extra, 1,
                    "rounding increment should overflow by at most one bit"
                );
                self.mantissa = self.mantissa.shr_bits(extra);
                self.exponent = self.exponent.saturating_add(extra as i64);
                // The newly-introduced trailing zero is allowed: the value is
                // still normalized at the requested precision.
            }
            Ordering::Less => {
                let shift = target - cur_bits;
                self.mantissa = self.mantissa.shl_bits(shift);
                self.exponent = self.exponent.saturating_sub(shift as i64);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Equality and ordering — NaN-aware (IEEE 754)
//
// `Eq` and `Ord` are NOT implemented: NaN breaks reflexivity (`NaN != NaN`)
// and totality. Use `partial_cmp` / `partial_ord` for IEEE comparisons, or
// `total_cmp` for a sort-stable total order.
// ---------------------------------------------------------------------------

impl PartialEq for BigFloat {
    fn eq(&self, other: &Self) -> bool {
        match (self.class, other.class) {
            // NaN never equals anything, including itself.
            (FloatClass::Nan, _) | (_, FloatClass::Nan) => false,
            // Infinities: equal iff same sign.
            (FloatClass::Infinite, FloatClass::Infinite) => self.sign == other.sign,
            (FloatClass::Infinite, _) | (_, FloatClass::Infinite) => false,
            // Both finite: precision-independent value equality.
            (FloatClass::Finite, FloatClass::Finite) => {
                if self.is_zero() && other.is_zero() {
                    return true;
                }
                if self.is_zero() != other.is_zero() {
                    return false;
                }
                self.sign == other.sign
                    && self.exponent == other.exponent
                    && self.mantissa == other.mantissa
            }
        }
    }
}

impl BigFloat {
    /// Compare the mathematical values of two **finite** `BigFloat`s.
    ///
    /// Caller must ensure both `self` and `other` are `Finite`.
    pub(crate) fn cmp_finite(&self, other: &Self) -> Ordering {
        match (self.is_zero(), other.is_zero()) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if other.sign == Sign::Negative {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if self.sign == Sign::Negative {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {}
        }
        match (self.sign, other.sign) {
            (Sign::Positive, Sign::Negative) => Ordering::Greater,
            (Sign::Negative, Sign::Positive) => Ordering::Less,
            (Sign::Positive, Sign::Positive) => cmp_magnitudes(self, other),
            (Sign::Negative, Sign::Negative) => cmp_magnitudes(other, self),
        }
    }

    /// IEEE 754-style total order, suitable for sorting.
    ///
    /// Sequence: `−Inf` < (negative finite) < zero < (positive finite) < `+Inf` < `NaN`.
    ///
    /// Because this type has a single canonical zero and a single canonical NaN,
    /// `total_cmp(NaN, NaN) == Equal` and `total_cmp(+Inf, NaN) == Less`.
    pub fn total_cmp(&self, other: &Self) -> Ordering {
        fn rank(x: &BigFloat) -> u8 {
            match x.class {
                FloatClass::Infinite if x.sign == Sign::Negative => 0,
                FloatClass::Finite => 1,
                FloatClass::Infinite => 2, // +Inf
                FloatClass::Nan => 3,
            }
        }
        let (ra, rb) = (rank(self), rank(other));
        if ra != rb {
            return ra.cmp(&rb);
        }
        match self.class {
            FloatClass::Finite => self.cmp_finite(other),
            _ => Ordering::Equal, // same non-finite rank → equal
        }
    }
}

impl PartialOrd for BigFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.class, other.class) {
            // NaN is unordered with everything.
            (FloatClass::Nan, _) | (_, FloatClass::Nan) => None,
            // Inf vs Inf: same sign → Equal; otherwise ± ordering.
            (FloatClass::Infinite, FloatClass::Infinite) => Some(match (self.sign, other.sign) {
                (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => {
                    Ordering::Equal
                }
                (Sign::Negative, Sign::Positive) => Ordering::Less,
                (Sign::Positive, Sign::Negative) => Ordering::Greater,
            }),
            // Inf vs finite.
            (FloatClass::Infinite, FloatClass::Finite) => Some(if self.sign == Sign::Negative {
                Ordering::Less
            } else {
                Ordering::Greater
            }),
            (FloatClass::Finite, FloatClass::Infinite) => Some(if other.sign == Sign::Negative {
                Ordering::Greater
            } else {
                Ordering::Less
            }),
            // Both finite.
            (FloatClass::Finite, FloatClass::Finite) => Some(self.cmp_finite(other)),
        }
    }
}

/// Compare the absolute values of two non-zero, normalized `BigFloat`s.
///
/// The normalization invariant lets us short-circuit on the *effective top
/// bit position* (`exponent + precision`) before falling back on a mantissa
/// comparison.
pub(crate) fn cmp_magnitudes(a: &BigFloat, b: &BigFloat) -> Ordering {
    // Effective top-bit position = exponent + (bit_length - 1).
    // Both are non-zero and normalized => bit_length == precision.
    // Compare top-bit positions first, then mantissas at common alignment.
    let top_a = a
        .exponent
        .saturating_add(a.mantissa.bit_length() as i64 - 1);
    let top_b = b
        .exponent
        .saturating_add(b.mantissa.bit_length() as i64 - 1);
    match top_a.cmp(&top_b) {
        Ordering::Equal => {
            // Same magnitude order — align mantissas to the smaller exponent
            // by shifting up the larger-exp mantissa.
            if a.exponent >= b.exponent {
                let shift = (a.exponent - b.exponent) as u64;
                let lhs = a.mantissa.shl_bits(shift);
                lhs.cmp(&b.mantissa)
            } else {
                let shift = (b.exponent - a.exponent) as u64;
                let rhs = b.mantissa.shl_bits(shift);
                a.mantissa.cmp(&rhs)
            }
        }
        non_eq => non_eq,
    }
}

// ---------------------------------------------------------------------------
// Hex-float Display ("0xb<binary>p<exp>")
// ---------------------------------------------------------------------------

impl fmt::Display for BigFloat {
    /// Hex-float-ish display. Always exact, always short:
    ///
    /// `<sign>0xb<binary-mantissa>p<exponent>`
    ///
    /// where `<binary-mantissa>` is the mantissa written in base 2 (MSB
    /// first). The `0xb` literal prefix is intentional: it visually marks the
    /// value as "binary hex-float", distinct from the C99 `0x<hex>p<exp>`
    /// format.
    ///
    /// Non-finite values display as `NaN`, `inf`, or `-inf`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Non-finite values — must come before `is_zero()` check because
        // NaN and Inf have mantissa=0 and would otherwise display as "0xb0p0".
        match self.class {
            FloatClass::Nan => return f.write_str("NaN"),
            FloatClass::Infinite => {
                return f.write_str(if self.sign == Sign::Negative {
                    "-inf"
                } else {
                    "inf"
                });
            }
            FloatClass::Finite => {}
        }
        // Existing hex-float body for finite values:
        if self.is_zero() {
            return f.write_str("0xb0p0");
        }
        if self.sign == Sign::Negative {
            f.write_str("-")?;
        }
        f.write_str("0xb")?;
        let bits = self.mantissa.bit_length();
        // Write MSB-first.
        for i in (0..bits).rev() {
            if self.mantissa.test_bit(i) {
                f.write_str("1")?;
            } else {
                f.write_str("0")?;
            }
        }
        write!(f, "p{}", self.exponent)
    }
}

impl fmt::Debug for BigFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BigFloat {{ class: {:?}, sign: {:?}, mantissa: {}, exponent: {}, precision: {} }}",
            self.class, self.sign, self.mantissa, self.exponent, self.precision
        )
    }
}
