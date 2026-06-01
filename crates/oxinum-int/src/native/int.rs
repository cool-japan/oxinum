//! Native `BigInt` — signed arbitrary-precision integer built as a
//! `Sign` + `BigUint` magnitude pair.
//!
//! # Invariants
//!
//! - The canonical zero is the ONLY zero: when `mag.is_zero()`, the sign
//!   MUST be `Sign::Positive`. This is enforced by every public constructor
//!   and after every arithmetic operation via [`BigInt::canonicalize`].
//!   Consequence: `+0 == -0` for `Eq`, `Ord`, and `Hash`.
//! - Magnitude follows [`super::uint::BigUint`] invariants (little-endian
//!   limbs, no trailing zeros).
//!
//! # Examples
//!
//! ```
//! use oxinum_int::native::{BigInt, BigUint};
//! use oxinum_core::Sign;
//!
//! let a = BigInt::from(-5i64);
//! let b = BigInt::from(7i64);
//! assert_eq!(&a + &b, BigInt::from(2i64));
//! assert_eq!(a.signum(), Sign::Negative);
//! assert!((-BigInt::zero()).is_zero());
//! ```

use super::uint::BigUint;
use core::cmp::Ordering;
use oxinum_core::Sign;
use std::fmt;

/// Native arbitrary-precision signed integer.
///
/// Represented as a [`Sign`] plus a non-negative [`BigUint`] magnitude. The
/// canonical-zero invariant guarantees a unique representation of zero (always
/// `Sign::Positive` + `BigUint::ZERO`).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BigInt {
    #[cfg_attr(feature = "serde", serde(with = "sign_serde"))]
    pub(super) sign: Sign,
    pub(super) mag: BigUint,
}

/// Serde helper for [`Sign`], which doesn't itself derive `Serialize` /
/// `Deserialize` in `dashu-base 0.4`. We encode the sign as a `bool`,
/// matching `dashu`'s own `From<Sign> for bool` convention
/// (`true == Negative`).
#[cfg(feature = "serde")]
mod sign_serde {
    use oxinum_core::Sign;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(s: &Sign, ser: S) -> Result<S::Ok, S::Error> {
        bool::from(*s).serialize(ser)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Sign, D::Error> {
        bool::deserialize(de).map(Sign::from)
    }
}

impl Default for BigInt {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl BigInt {
    /// The canonical zero value (`+0`).
    pub const ZERO: BigInt = BigInt {
        sign: Sign::Positive,
        mag: BigUint::ZERO,
    };

    /// Construct a zero `BigInt`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    /// assert!(BigInt::zero().is_zero());
    /// ```
    #[inline]
    pub fn zero() -> Self {
        Self::ZERO
    }

    /// Construct a `BigInt` equal to `1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    /// assert!(BigInt::one().is_one());
    /// ```
    #[inline]
    pub fn one() -> Self {
        Self {
            sign: Sign::Positive,
            mag: BigUint::one(),
        }
    }

    /// Construct from an existing `(sign, magnitude)` pair. Re-canonicalizes
    /// zero so that `BigInt::from_parts(Sign::Negative, BigUint::ZERO)` is
    /// indistinguishable from `BigInt::zero()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::{BigInt, BigUint};
    /// use oxinum_core::Sign;
    /// let a = BigInt::from_parts(Sign::Negative, BigUint::from_u64(7));
    /// assert_eq!(format!("{a}"), "-7");
    ///
    /// // -0 canonicalizes to +0.
    /// let neg_zero = BigInt::from_parts(Sign::Negative, BigUint::ZERO);
    /// assert_eq!(neg_zero, BigInt::zero());
    /// ```
    pub fn from_parts(sign: Sign, mag: BigUint) -> Self {
        let mut out = Self { sign, mag };
        out.canonicalize();
        out
    }

    /// Decompose into `(sign, magnitude)`. For zero, the returned sign is
    /// always `Sign::Positive`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::{BigInt, BigUint};
    /// use oxinum_core::Sign;
    /// let n = BigInt::from(-42i64);
    /// let (s, m) = n.into_parts();
    /// assert_eq!(s, Sign::Negative);
    /// assert_eq!(m, BigUint::from_u64(42));
    /// ```
    #[inline]
    pub fn into_parts(self) -> (Sign, BigUint) {
        (self.sign, self.mag)
    }

    /// Returns the sign of this number. For zero, returns `Sign::Positive`
    /// (canonical-zero invariant).
    #[inline]
    pub fn sign(&self) -> Sign {
        self.sign
    }

    /// Returns the sign as a method that follows the standard `signum`
    /// convention: `+1`, `-1`, or `0`-as-`Positive`. Use [`Self::sign`] for
    /// the raw sign enum.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    /// use oxinum_core::Sign;
    /// assert_eq!(BigInt::from(5i64).signum(), Sign::Positive);
    /// assert_eq!(BigInt::from(-3i64).signum(), Sign::Negative);
    /// // Zero is canonically positive in dashu_base::Sign.
    /// assert_eq!(BigInt::zero().signum(), Sign::Positive);
    /// ```
    #[inline]
    pub fn signum(&self) -> Sign {
        self.sign
    }

    /// Returns a reference to the magnitude (always non-negative).
    #[inline]
    pub fn magnitude(&self) -> &BigUint {
        &self.mag
    }

    /// Returns the absolute value as a non-negative `BigInt`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    /// assert_eq!(BigInt::from(-42i64).abs(), BigInt::from(42i64));
    /// assert_eq!(BigInt::from(42i64).abs(), BigInt::from(42i64));
    /// ```
    pub fn abs(&self) -> Self {
        Self {
            sign: Sign::Positive,
            mag: self.mag.clone(),
        }
    }

    /// Returns `true` if this value is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.mag.is_zero()
    }

    /// Returns `true` if this value is `+1` (sign positive AND magnitude one).
    #[inline]
    pub fn is_one(&self) -> bool {
        self.sign == Sign::Positive && self.mag.is_one()
    }

    /// Returns `true` if this value is strictly negative.
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.sign == Sign::Negative && !self.mag.is_zero()
    }

    /// Returns `true` if this value is strictly positive.
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.sign == Sign::Positive && !self.mag.is_zero()
    }

    /// Force the canonical-zero invariant: if `mag.is_zero()` then
    /// `sign = Sign::Positive`. This is a no-op for non-zero values.
    ///
    /// Called by every constructor and after every arithmetic operation.
    #[inline]
    pub(crate) fn canonicalize(&mut self) {
        if self.mag.is_zero() {
            self.sign = Sign::Positive;
        }
    }
}

// ---------------------------------------------------------------------------
// Equality, ordering, hashing
// ---------------------------------------------------------------------------

impl PartialEq for BigInt {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Thanks to the canonical-zero invariant, `+0 == -0` is automatic:
        // both have sign Positive and empty magnitude.
        self.sign == other.sign && self.mag == other.mag
    }
}

impl Eq for BigInt {}

impl std::hash::Hash for BigInt {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash on (sign, mag) — works correctly because zero is canonical.
        // For zero, the sign is always Positive, so +0 and -0 are unhashable
        // distinctly.
        self.sign.hash(state);
        self.mag.hash(state);
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.sign, other.sign) {
            // Same sign: compare magnitudes. For Negative, reverse the
            // ordering (larger magnitude means smaller value).
            (Sign::Positive, Sign::Positive) => self.mag.cmp(&other.mag),
            (Sign::Negative, Sign::Negative) => other.mag.cmp(&self.mag),
            // Mixed signs: if either side is zero, they are equal (canonical
            // zero would have made both Positive — so reaching here means at
            // least one side is non-zero). The strictly positive value wins.
            (Sign::Positive, Sign::Negative) => {
                // self is +x (x can be 0 but canonical-zero ensures other.mag
                // is non-zero whenever other.sign == Negative).
                if other.mag.is_zero() {
                    // Cannot happen under canonical-zero, but stay defensive.
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            }
            (Sign::Negative, Sign::Positive) => {
                if self.mag.is_zero() {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            }
        }
    }
}

impl PartialOrd for BigInt {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sign == Sign::Negative && !self.mag.is_zero() {
            f.write_str("-")?;
        }
        fmt::Display::fmt(&self.mag, f)
    }
}

impl fmt::Debug for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign_str = if self.sign == Sign::Negative && !self.mag.is_zero() {
            "-"
        } else {
            ""
        };
        write!(f, "BigInt({sign_str}{})", self.mag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_canonical_under_from_parts() {
        let pz = BigInt::from_parts(Sign::Positive, BigUint::ZERO);
        let nz = BigInt::from_parts(Sign::Negative, BigUint::ZERO);
        assert_eq!(pz, nz);
        assert_eq!(pz.sign(), Sign::Positive);
        assert_eq!(nz.sign(), Sign::Positive);
    }

    #[test]
    fn is_zero_one_negative_positive() {
        assert!(BigInt::zero().is_zero());
        assert!(BigInt::one().is_one());
        assert!(!BigInt::zero().is_negative());
        assert!(!BigInt::zero().is_positive());
        let neg = BigInt::from_parts(Sign::Negative, BigUint::from_u64(7));
        assert!(neg.is_negative());
        assert!(!neg.is_positive());
    }

    #[test]
    fn ord_negative_less_than_positive() {
        let n = BigInt::from_parts(Sign::Negative, BigUint::from_u64(100));
        let p = BigInt::from_parts(Sign::Positive, BigUint::from_u64(1));
        assert!(n < p);
    }

    #[test]
    fn ord_two_negatives_larger_mag_is_smaller() {
        let a = BigInt::from_parts(Sign::Negative, BigUint::from_u64(100));
        let b = BigInt::from_parts(Sign::Negative, BigUint::from_u64(1));
        assert!(a < b);
    }
}
