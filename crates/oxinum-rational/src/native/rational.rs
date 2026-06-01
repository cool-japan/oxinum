//! `BigRational` struct definition, invariants, constructors, accessors,
//! basic predicates, [`Neg`], [`Display`], and `From<primitive>` impls.
//!
//! The arithmetic operators (`Add`/`Sub`/`Mul`/`Div`/`Rem` and their
//! `*Assign` partners) plus `PartialOrd`/`Ord`/`Hash` live in
//! [`super::rational_ops`].

use core::cmp::Ordering;
use core::ops::Neg;
use std::fmt;

use oxinum_core::{OxiNumError, OxiNumResult, Sign};
use oxinum_int::native::{gcd, BigInt, BigUint};

/// Native arbitrary-precision rational number, always stored in lowest terms.
///
/// Internally represented as a signed numerator (`BigInt`) over a strictly
/// positive denominator (`BigUint`). The sign always lives on the numerator;
/// `den` is non-zero by invariant.
///
/// # Canonical form
///
/// - `gcd(|num|, den) == 1`
/// - `den > 0`
/// - Zero is the unique `{ num: BigInt::ZERO, den: BigUint::from_u64(1) }`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::native::BigRational;
/// use oxinum_int::native::{BigInt, BigUint};
///
/// let half = BigRational::from_parts(BigInt::from(1i64), BigUint::from_u64(2))
///     .expect("non-zero denominator");
/// assert_eq!(half.to_string(), "1/2");
/// ```
#[derive(Clone, Debug)]
pub struct BigRational {
    pub(super) num: BigInt,
    pub(super) den: BigUint,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl BigRational {
    /// Construct a `BigRational` from a numerator and a denominator.
    ///
    /// Reduces to lowest terms automatically. Returns
    /// [`OxiNumError::DivByZero`] when `den` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    /// let r = BigRational::from_parts(BigInt::from(6i64), BigUint::from_u64(4))
    ///     .expect("non-zero denominator");
    /// assert_eq!(r.to_string(), "3/2");
    /// ```
    pub fn from_parts(num: BigInt, den: BigUint) -> OxiNumResult<Self> {
        if den.is_zero() {
            return Err(OxiNumError::DivByZero);
        }
        Ok(Self::reduce_unchecked(num, den))
    }

    /// Construct a `BigRational` representing the integer `n`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::BigInt;
    /// let r = BigRational::from_integer(BigInt::from(7i64));
    /// assert_eq!(r.to_string(), "7");
    /// ```
    #[inline]
    pub fn from_integer(n: BigInt) -> Self {
        Self {
            num: n,
            den: BigUint::one(),
        }
    }

    /// Construct a `BigRational` from a signed 64-bit integer.
    #[inline]
    pub fn from_i64(n: i64) -> Self {
        Self::from_integer(BigInt::from(n))
    }

    /// The canonical zero, `0/1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// assert!(BigRational::zero().is_zero());
    /// ```
    #[inline]
    pub fn zero() -> Self {
        Self {
            num: BigInt::ZERO,
            den: BigUint::one(),
        }
    }

    /// The canonical one, `1/1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// assert!(BigRational::one().is_one());
    /// ```
    #[inline]
    pub fn one() -> Self {
        Self {
            num: BigInt::one(),
            den: BigUint::one(),
        }
    }

    // -------------------------------------------------------------------
    // Internal: reduce-without-checking-zero-denominator
    // -------------------------------------------------------------------

    /// Reduce `(num, den)` to lowest terms. Caller MUST guarantee
    /// `!den.is_zero()`.
    pub(super) fn reduce_unchecked(num: BigInt, den: BigUint) -> Self {
        // Fast path for zero numerator: canonical zero is `{0, 1}`.
        if num.is_zero() {
            return Self::zero();
        }
        // `gcd` takes ownership of both arguments; clone the magnitude and
        // the denominator before consuming them.
        let g = gcd(num.magnitude().clone(), den.clone());
        if g.is_one() {
            return Self { num, den };
        }
        // Divide both magnitude and denominator by the GCD.
        let (sign, mag) = num.into_parts();
        let new_mag = &mag / &g;
        let new_den = &den / &g;
        Self {
            num: BigInt::from_parts(sign, new_mag),
            den: new_den,
        }
    }
}

// ---------------------------------------------------------------------------
// Accessors and predicates
// ---------------------------------------------------------------------------

impl BigRational {
    /// Borrow the (signed) numerator.
    #[inline]
    pub fn num(&self) -> &BigInt {
        &self.num
    }

    /// Borrow the (strictly positive) denominator.
    #[inline]
    pub fn den(&self) -> &BigUint {
        &self.den
    }

    /// Returns `true` if this value equals zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Returns `true` if this value equals one.
    #[inline]
    pub fn is_one(&self) -> bool {
        self.num.is_one() && self.den.is_one()
    }

    /// Returns `true` if this value represents an integer (denominator is one).
    #[inline]
    pub fn is_integer(&self) -> bool {
        self.den.is_one()
    }

    /// Returns the sign as `+1`, `-1`, or `0`.
    ///
    /// Unlike piping [`BigInt::signum`] (which returns `Sign::Positive` for
    /// zero by canonical-zero invariant), this method actively distinguishes
    /// zero by checking the numerator first.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    /// let pos = BigRational::from_parts(BigInt::from(2i64), BigUint::from_u64(3))
    ///     .expect("non-zero denominator");
    /// let neg = BigRational::from_parts(BigInt::from(-2i64), BigUint::from_u64(3))
    ///     .expect("non-zero denominator");
    /// assert_eq!(pos.signum(), 1);
    /// assert_eq!(neg.signum(), -1);
    /// assert_eq!(BigRational::zero().signum(), 0);
    /// ```
    pub fn signum(&self) -> i32 {
        if self.num.is_zero() {
            0
        } else {
            match self.num.sign() {
                Sign::Positive => 1,
                Sign::Negative => -1,
            }
        }
    }

    /// Returns the absolute value (a non-negative copy).
    pub fn abs(&self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den.clone(),
        }
    }

    /// Returns the reciprocal `1/self`.
    ///
    /// Returns [`OxiNumError::DivByZero`] when `self` is zero.
    ///
    /// The reciprocal is always already reduced because the original was; the
    /// sign of the numerator is preserved (moving from the old numerator to
    /// the new numerator slot since the new denominator must be positive).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    /// let r = BigRational::from_parts(BigInt::from(-2i64), BigUint::from_u64(3))
    ///     .expect("non-zero denominator");
    /// let recip = r.recip().expect("non-zero source");
    /// assert_eq!(recip.to_string(), "-3/2");
    /// ```
    pub fn recip(&self) -> OxiNumResult<Self> {
        if self.num.is_zero() {
            return Err(OxiNumError::DivByZero);
        }
        // |num| becomes the new denominator, den becomes |new_num|, and the
        // sign of the original numerator transfers to the new numerator.
        let (sign, mag) = self.num.clone().into_parts();
        let new_num = BigInt::from_parts(sign, self.den.clone());
        let new_den = mag;
        Ok(Self {
            num: new_num,
            den: new_den,
        })
    }

    // -------------------------------------------------------------------
    // Crate-internal comparison helpers (shared with rational_ops)
    // -------------------------------------------------------------------

    /// Compare two rationals using cross-multiplication.
    ///
    /// Since both denominators are strictly positive, the sign of `a/b - c/d`
    /// equals the sign of `a*d - c*b`. We reduce to that single `BigInt`
    /// comparison after lifting the (unsigned) denominators into `BigInt`.
    pub(super) fn cmp_impl(&self, other: &Self) -> Ordering {
        let lhs_den_i = BigInt::from(self.den.clone());
        let rhs_den_i = BigInt::from(other.den.clone());
        let lhs = &self.num * &rhs_den_i;
        let rhs = &other.num * &lhs_den_i;
        lhs.cmp(&rhs)
    }
}

// ---------------------------------------------------------------------------
// Equality (canonical form makes this trivial)
// ---------------------------------------------------------------------------

impl PartialEq for BigRational {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Both sides are in canonical form, so structural equality is exact.
        self.num == other.num && self.den == other.den
    }
}

impl Eq for BigRational {}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for BigRational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_integer() {
            fmt::Display::fmt(&self.num, f)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

// ---------------------------------------------------------------------------
// Neg
// ---------------------------------------------------------------------------

impl Neg for BigRational {
    type Output = BigRational;
    #[inline]
    fn neg(self) -> BigRational {
        BigRational {
            num: -self.num,
            den: self.den,
        }
    }
}

impl Neg for &BigRational {
    type Output = BigRational;
    #[inline]
    fn neg(self) -> BigRational {
        BigRational {
            num: -&self.num,
            den: self.den.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Default
// ---------------------------------------------------------------------------

impl Default for BigRational {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

// ---------------------------------------------------------------------------
// From<primitive>
// ---------------------------------------------------------------------------

macro_rules! impl_from_signed_primitive {
    ($($t:ty),*) => {
        $(
            impl From<$t> for BigRational {
                #[inline]
                fn from(value: $t) -> Self {
                    Self::from_integer(BigInt::from(value))
                }
            }
        )*
    };
}

macro_rules! impl_from_unsigned_primitive {
    ($($t:ty),*) => {
        $(
            impl From<$t> for BigRational {
                #[inline]
                fn from(value: $t) -> Self {
                    Self::from_integer(BigInt::from(value))
                }
            }
        )*
    };
}

impl_from_signed_primitive!(i8, i16, i32, i64, i128, isize);
impl_from_unsigned_primitive!(u8, u16, u32, u64, u128, usize);

impl From<BigInt> for BigRational {
    #[inline]
    fn from(n: BigInt) -> Self {
        Self::from_integer(n)
    }
}

impl From<&BigInt> for BigRational {
    #[inline]
    fn from(n: &BigInt) -> Self {
        Self::from_integer(n.clone())
    }
}

// ---------------------------------------------------------------------------
// BigRational → BigInt conversions
// ---------------------------------------------------------------------------

impl BigRational {
    /// Converts this rational to a [`BigInt`] by truncating toward zero.
    ///
    /// Returns the integer part of `self` (`floor(|self|)` with the original
    /// sign). Equivalent to T-division (C/Rust integer division).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    ///
    /// let r = BigRational::from_parts(BigInt::from(7i64), BigUint::from_u64(2))
    ///     .expect("7/2");
    /// assert_eq!(r.to_bigint_trunc(), BigInt::from(3i64));
    ///
    /// let s = BigRational::from_parts(BigInt::from(-7i64), BigUint::from_u64(2))
    ///     .expect("-7/2");
    /// assert_eq!(s.to_bigint_trunc(), BigInt::from(-3i64));
    /// ```
    pub fn to_bigint_trunc(&self) -> BigInt {
        if self.is_integer() {
            return self.num.clone();
        }
        let den_int = BigInt::from(self.den.clone());
        let (q, _) = oxinum_int::native::divrem_int(&self.num, &den_int);
        q
    }

    /// Converts this rational to a [`BigInt`] by rounding toward negative
    /// infinity (floor division).
    ///
    /// For negative non-integer values the result is one less than the
    /// truncation.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    ///
    /// let r = BigRational::from_parts(BigInt::from(7i64), BigUint::from_u64(2))
    ///     .expect("7/2");
    /// assert_eq!(r.to_bigint_floor(), BigInt::from(3i64));
    ///
    /// let s = BigRational::from_parts(BigInt::from(-7i64), BigUint::from_u64(2))
    ///     .expect("-7/2");
    /// assert_eq!(s.to_bigint_floor(), BigInt::from(-4i64));
    /// ```
    pub fn to_bigint_floor(&self) -> BigInt {
        if self.is_integer() {
            return self.num.clone();
        }
        let den_int = BigInt::from(self.den.clone());
        let (q, r) = oxinum_int::native::divrem_int(&self.num, &den_int);
        // If negative with non-zero remainder, the truncation is above the
        // floor, so subtract 1.
        if self.num.is_negative() && !r.is_zero() {
            &q - &BigInt::one()
        } else {
            q
        }
    }

    /// Converts this rational to a [`BigInt`] by rounding toward positive
    /// infinity (ceiling division).
    ///
    /// For positive non-integer values the result is one more than the
    /// truncation.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    ///
    /// let r = BigRational::from_parts(BigInt::from(7i64), BigUint::from_u64(2))
    ///     .expect("7/2");
    /// assert_eq!(r.to_bigint_ceil(), BigInt::from(4i64));
    ///
    /// let s = BigRational::from_parts(BigInt::from(-7i64), BigUint::from_u64(2))
    ///     .expect("-7/2");
    /// assert_eq!(s.to_bigint_ceil(), BigInt::from(-3i64));
    /// ```
    pub fn to_bigint_ceil(&self) -> BigInt {
        if self.is_integer() {
            return self.num.clone();
        }
        let den_int = BigInt::from(self.den.clone());
        let (q, r) = oxinum_int::native::divrem_int(&self.num, &den_int);
        // If positive with non-zero remainder, the truncation is below the
        // ceiling, so add 1.
        if self.num.is_positive() && !r.is_zero() {
            &q + &BigInt::one()
        } else {
            q
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_parts_reduces_six_quarters() {
        let r = BigRational::from_parts(BigInt::from(6i64), BigUint::from_u64(4))
            .expect("non-zero denominator");
        assert_eq!(r.num(), &BigInt::from(3i64));
        assert_eq!(r.den(), &BigUint::from_u64(2));
    }

    #[test]
    fn from_parts_handles_negative_numerator() {
        let r = BigRational::from_parts(BigInt::from(-9i64), BigUint::from_u64(12))
            .expect("non-zero denominator");
        assert_eq!(r.to_string(), "-3/4");
    }

    #[test]
    fn from_parts_zero_over_anything_is_canonical_zero() {
        let r = BigRational::from_parts(BigInt::ZERO, BigUint::from_u64(5))
            .expect("non-zero denominator");
        assert_eq!(r.num(), &BigInt::ZERO);
        assert_eq!(r.den(), &BigUint::one());
    }

    #[test]
    fn from_parts_div_by_zero() {
        let err = BigRational::from_parts(BigInt::from(1i64), BigUint::ZERO);
        assert_eq!(err, Err(OxiNumError::DivByZero));
    }

    #[test]
    fn is_integer_predicate() {
        let i = BigRational::from_i64(7);
        assert!(i.is_integer());
        let f = BigRational::from_parts(BigInt::from(3i64), BigUint::from_u64(2))
            .expect("non-zero denominator");
        assert!(!f.is_integer());
    }

    #[test]
    fn display_integer_form() {
        let r = BigRational::from_i64(-7);
        assert_eq!(r.to_string(), "-7");
    }

    #[test]
    fn display_fraction_form() {
        let r = BigRational::from_parts(BigInt::from(22i64), BigUint::from_u64(7))
            .expect("non-zero denominator");
        assert_eq!(r.to_string(), "22/7");
    }

    #[test]
    fn signum_distinguishes_zero() {
        assert_eq!(BigRational::zero().signum(), 0);
        assert_eq!(BigRational::from_i64(5).signum(), 1);
        assert_eq!(BigRational::from_i64(-5).signum(), -1);
    }

    #[test]
    fn recip_of_zero_errors() {
        assert_eq!(BigRational::zero().recip(), Err(OxiNumError::DivByZero));
    }

    #[test]
    fn recip_preserves_sign() {
        let r = BigRational::from_parts(BigInt::from(-2i64), BigUint::from_u64(3))
            .expect("non-zero denominator");
        let recip = r.recip().expect("non-zero source");
        assert_eq!(recip.to_string(), "-3/2");
    }

    #[test]
    fn neg_owned_and_borrowed() {
        let r = BigRational::from_parts(BigInt::from(3i64), BigUint::from_u64(4))
            .expect("non-zero denominator");
        assert_eq!((-&r).to_string(), "-3/4");
        assert_eq!((-r).to_string(), "-3/4");
    }

    #[test]
    fn abs_works() {
        let r = BigRational::from_parts(BigInt::from(-5i64), BigUint::from_u64(7))
            .expect("non-zero denominator");
        let a = r.abs();
        assert_eq!(a.to_string(), "5/7");
    }

    #[test]
    fn from_primitive_signed_and_unsigned() {
        let a: BigRational = (-3i32).into();
        let b: BigRational = 7u32.into();
        assert_eq!(a.to_string(), "-3");
        assert_eq!(b.to_string(), "7");
    }

    #[test]
    fn default_is_zero() {
        let r = BigRational::default();
        assert!(r.is_zero());
        assert_eq!(r.to_string(), "0");
    }
}
