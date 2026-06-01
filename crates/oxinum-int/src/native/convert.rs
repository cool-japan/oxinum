//! Primitive integer conversions for [`BigInt`] and [`BigUint`].
//!
//! - `From<u{8,16,32,64,128}>`, `From<i{8,16,32,64,128}>`, `From<usize>`,
//!   `From<isize>` for `BigInt`.
//! - Same `From` set for `BigUint` (signed flavors converted via
//!   `unsigned_abs` and rejected only when the value is strictly negative —
//!   `From` is infallible by signature, so negative signed inputs are *not*
//!   provided as `From`; use `TryFrom` instead).
//! - `TryFrom<&BigInt>` / `TryFrom<&BigUint>` for the same primitives, with
//!   range checks returning [`OxiNumError::Overflow`] on out-of-range.

use super::int::BigInt;
use super::uint::BigUint;
use crate::OxiNumError;
use oxinum_core::Sign;

// ---------------------------------------------------------------------------
// From<*> for BigUint
// ---------------------------------------------------------------------------

impl From<u8> for BigUint {
    #[inline]
    fn from(v: u8) -> Self {
        BigUint::from_u64(v as u64)
    }
}

impl From<u16> for BigUint {
    #[inline]
    fn from(v: u16) -> Self {
        BigUint::from_u64(v as u64)
    }
}

impl From<u32> for BigUint {
    #[inline]
    fn from(v: u32) -> Self {
        BigUint::from_u64(v as u64)
    }
}

impl From<u64> for BigUint {
    #[inline]
    fn from(v: u64) -> Self {
        BigUint::from_u64(v)
    }
}

impl From<u128> for BigUint {
    #[inline]
    fn from(v: u128) -> Self {
        BigUint::from_u128(v)
    }
}

impl From<usize> for BigUint {
    #[inline]
    fn from(v: usize) -> Self {
        BigUint::from_u64(v as u64)
    }
}

// ---------------------------------------------------------------------------
// From<*> for BigInt (signed and unsigned primitives)
// ---------------------------------------------------------------------------

#[inline]
fn from_signed<T>(value: T) -> BigInt
where
    T: SignedPrimitive,
{
    if value.is_negative_signed() {
        BigInt {
            sign: Sign::Negative,
            mag: BigUint::from_u128(value.unsigned_abs_u128()),
        }
        .normalize_zero()
    } else {
        BigInt {
            sign: Sign::Positive,
            mag: BigUint::from_u128(value.unsigned_abs_u128()),
        }
        .normalize_zero()
    }
}

#[inline]
fn from_unsigned(value: u128) -> BigInt {
    BigInt {
        sign: Sign::Positive,
        mag: BigUint::from_u128(value),
    }
    .normalize_zero()
}

/// Implementation detail: capture `unsigned_abs` as a `u128` for all signed
/// primitive widths so that we have one code path. The `i128::MIN` case is
/// handled correctly because `i128::unsigned_abs() -> u128` returns
/// `1u128 << 127`.
trait SignedPrimitive {
    fn is_negative_signed(&self) -> bool;
    fn unsigned_abs_u128(&self) -> u128;
}

macro_rules! impl_signed_prim {
    ($($t:ty),*) => {
        $(
            impl SignedPrimitive for $t {
                #[inline]
                fn is_negative_signed(&self) -> bool {
                    *self < 0
                }
                #[inline]
                fn unsigned_abs_u128(&self) -> u128 {
                    self.unsigned_abs() as u128
                }
            }
        )*
    };
}

impl_signed_prim!(i8, i16, i32, i64, isize);

// i128 needs special handling because `unsigned_abs` already returns u128.
impl SignedPrimitive for i128 {
    #[inline]
    fn is_negative_signed(&self) -> bool {
        *self < 0
    }
    #[inline]
    fn unsigned_abs_u128(&self) -> u128 {
        self.unsigned_abs()
    }
}

impl BigInt {
    /// Internal helper: ensure canonical-zero after construction without
    /// re-borrowing through `canonicalize`. Kept private to this module.
    #[inline]
    fn normalize_zero(mut self) -> Self {
        if self.mag.is_zero() {
            self.sign = Sign::Positive;
        }
        self
    }
}

impl From<u8> for BigInt {
    #[inline]
    fn from(v: u8) -> Self {
        from_unsigned(v as u128)
    }
}

impl From<u16> for BigInt {
    #[inline]
    fn from(v: u16) -> Self {
        from_unsigned(v as u128)
    }
}

impl From<u32> for BigInt {
    #[inline]
    fn from(v: u32) -> Self {
        from_unsigned(v as u128)
    }
}

impl From<u64> for BigInt {
    #[inline]
    fn from(v: u64) -> Self {
        from_unsigned(v as u128)
    }
}

impl From<u128> for BigInt {
    #[inline]
    fn from(v: u128) -> Self {
        from_unsigned(v)
    }
}

impl From<usize> for BigInt {
    #[inline]
    fn from(v: usize) -> Self {
        from_unsigned(v as u128)
    }
}

impl From<i8> for BigInt {
    #[inline]
    fn from(v: i8) -> Self {
        from_signed(v)
    }
}

impl From<i16> for BigInt {
    #[inline]
    fn from(v: i16) -> Self {
        from_signed(v)
    }
}

impl From<i32> for BigInt {
    #[inline]
    fn from(v: i32) -> Self {
        from_signed(v)
    }
}

impl From<i64> for BigInt {
    #[inline]
    fn from(v: i64) -> Self {
        from_signed(v)
    }
}

impl From<i128> for BigInt {
    #[inline]
    fn from(v: i128) -> Self {
        from_signed(v)
    }
}

impl From<isize> for BigInt {
    #[inline]
    fn from(v: isize) -> Self {
        from_signed(v)
    }
}

impl From<BigUint> for BigInt {
    #[inline]
    fn from(m: BigUint) -> Self {
        BigInt::from_parts(Sign::Positive, m)
    }
}

impl From<&BigUint> for BigInt {
    #[inline]
    fn from(m: &BigUint) -> Self {
        BigInt::from_parts(Sign::Positive, m.clone())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&BigUint> for primitives
// ---------------------------------------------------------------------------

#[inline]
fn biguint_to_u128(value: &BigUint) -> Result<u128, OxiNumError> {
    let limbs = value.as_limbs();
    match limbs.len() {
        0 => Ok(0),
        1 => Ok(limbs[0] as u128),
        2 => Ok(((limbs[1] as u128) << 64) | (limbs[0] as u128)),
        _ => Err(OxiNumError::Overflow(
            format!("BigUint with {} limbs does not fit in u128", limbs.len()).into(),
        )),
    }
}

macro_rules! impl_try_from_biguint_unsigned {
    ($($t:ident),*) => {
        $(
            impl TryFrom<&BigUint> for $t {
                type Error = OxiNumError;
                fn try_from(value: &BigUint) -> Result<Self, Self::Error> {
                    let v = biguint_to_u128(value)?;
                    if v > Self::MAX as u128 {
                        return Err(OxiNumError::Overflow(format!(
                            "BigUint {value} does not fit in {}",
                            stringify!($t)
                        ).into()));
                    }
                    Ok(v as Self)
                }
            }

            impl TryFrom<BigUint> for $t {
                type Error = OxiNumError;
                fn try_from(value: BigUint) -> Result<Self, Self::Error> {
                    Self::try_from(&value)
                }
            }
        )*
    };
}

// u128 needs a custom impl because comparing to u128::MAX is identity.
impl TryFrom<&BigUint> for u128 {
    type Error = OxiNumError;
    fn try_from(value: &BigUint) -> Result<Self, Self::Error> {
        biguint_to_u128(value)
    }
}

impl TryFrom<BigUint> for u128 {
    type Error = OxiNumError;
    fn try_from(value: BigUint) -> Result<Self, Self::Error> {
        biguint_to_u128(&value)
    }
}

impl_try_from_biguint_unsigned!(u8, u16, u32, u64, usize);

macro_rules! impl_try_from_biguint_signed {
    ($($t:ident),*) => {
        $(
            impl TryFrom<&BigUint> for $t {
                type Error = OxiNumError;
                fn try_from(value: &BigUint) -> Result<Self, Self::Error> {
                    let v = biguint_to_u128(value)?;
                    if v > Self::MAX as u128 {
                        return Err(OxiNumError::Overflow(format!(
                            "BigUint {value} does not fit in {}",
                            stringify!($t)
                        ).into()));
                    }
                    Ok(v as Self)
                }
            }

            impl TryFrom<BigUint> for $t {
                type Error = OxiNumError;
                fn try_from(value: BigUint) -> Result<Self, Self::Error> {
                    Self::try_from(&value)
                }
            }
        )*
    };
}

// i128 special: cannot exceed i128::MAX (= 2^127 - 1).
impl TryFrom<&BigUint> for i128 {
    type Error = OxiNumError;
    fn try_from(value: &BigUint) -> Result<Self, Self::Error> {
        let v = biguint_to_u128(value)?;
        if v > i128::MAX as u128 {
            return Err(OxiNumError::Overflow(
                format!("BigUint {value} does not fit in i128").into(),
            ));
        }
        Ok(v as i128)
    }
}

impl TryFrom<BigUint> for i128 {
    type Error = OxiNumError;
    fn try_from(value: BigUint) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl_try_from_biguint_signed!(i8, i16, i32, i64, isize);

// ---------------------------------------------------------------------------
// TryFrom<&BigInt> for primitives
// ---------------------------------------------------------------------------

macro_rules! impl_try_from_bigint_unsigned {
    ($($t:ident),*) => {
        $(
            impl TryFrom<&BigInt> for $t {
                type Error = OxiNumError;
                fn try_from(value: &BigInt) -> Result<Self, Self::Error> {
                    if value.is_negative() {
                        return Err(OxiNumError::Overflow(format!(
                            "negative BigInt {value} cannot fit in {}",
                            stringify!($t)
                        ).into()));
                    }
                    <$t>::try_from(value.magnitude())
                }
            }

            impl TryFrom<BigInt> for $t {
                type Error = OxiNumError;
                fn try_from(value: BigInt) -> Result<Self, Self::Error> {
                    Self::try_from(&value)
                }
            }
        )*
    };
}

impl_try_from_bigint_unsigned!(u8, u16, u32, u64, u128, usize);

// Signed targets require both:
//   - the magnitude doesn't exceed T::MAX (for positive values)
//   - or magnitude equals T::MAX as u128 + 1 (for negative MIN-boundary case)
macro_rules! impl_try_from_bigint_signed {
    ($($t:ident),*) => {
        $(
            impl TryFrom<&BigInt> for $t {
                type Error = OxiNumError;
                fn try_from(value: &BigInt) -> Result<Self, Self::Error> {
                    let mag_u128 = biguint_to_u128(value.magnitude())?;
                    let max_pos = <$t>::MAX as u128;
                    let min_abs = (<$t>::MAX as u128) + 1; // |T::MIN| = 2^(bits-1)
                    if value.is_negative() {
                        if mag_u128 > min_abs {
                            return Err(OxiNumError::Overflow(format!(
                                "BigInt {value} does not fit in {} (too negative)",
                                stringify!($t)
                            ).into()));
                        }
                        if mag_u128 == min_abs {
                            return Ok(<$t>::MIN);
                        }
                        // mag_u128 <= max_pos; negate safely.
                        let pos = mag_u128 as i128;
                        return Ok(-pos as $t);
                    }
                    if mag_u128 > max_pos {
                        return Err(OxiNumError::Overflow(format!(
                            "BigInt {value} does not fit in {} (too positive)",
                            stringify!($t)
                        ).into()));
                    }
                    Ok(mag_u128 as $t)
                }
            }

            impl TryFrom<BigInt> for $t {
                type Error = OxiNumError;
                fn try_from(value: BigInt) -> Result<Self, Self::Error> {
                    Self::try_from(&value)
                }
            }
        )*
    };
}

impl_try_from_bigint_signed!(i8, i16, i32, i64, isize);

// i128 special-case (handles 2^127 = -i128::MIN distinctly).
impl TryFrom<&BigInt> for i128 {
    type Error = OxiNumError;
    fn try_from(value: &BigInt) -> Result<Self, Self::Error> {
        let mag_u128 = biguint_to_u128(value.magnitude())?;
        let max_pos = i128::MAX as u128;
        let min_abs = (i128::MAX as u128) + 1; // 2^127
        if value.is_negative() {
            if mag_u128 > min_abs {
                return Err(OxiNumError::Overflow(
                    format!("BigInt {value} does not fit in i128 (too negative)").into(),
                ));
            }
            if mag_u128 == min_abs {
                return Ok(i128::MIN);
            }
            return Ok(-(mag_u128 as i128));
        }
        if mag_u128 > max_pos {
            return Err(OxiNumError::Overflow(
                format!("BigInt {value} does not fit in i128 (too positive)").into(),
            ));
        }
        Ok(mag_u128 as i128)
    }
}

impl TryFrom<BigInt> for i128 {
    type Error = OxiNumError;
    fn try_from(value: BigInt) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_i64_min_roundtrip() {
        let n = BigInt::from(i64::MIN);
        assert_eq!(format!("{n}"), format!("{}", i64::MIN));
        let back: i64 = i64::try_from(&n).expect("i64::MIN fits in i64");
        assert_eq!(back, i64::MIN);
    }

    #[test]
    fn from_i128_min_roundtrip() {
        let n = BigInt::from(i128::MIN);
        let back: i128 = i128::try_from(&n).expect("i128::MIN fits in i128");
        assert_eq!(back, i128::MIN);
    }

    #[test]
    fn u64_max_roundtrip() {
        let n = BigInt::from(u64::MAX);
        let back: u64 = u64::try_from(&n).expect("u64::MAX fits in u64");
        assert_eq!(back, u64::MAX);
    }

    #[test]
    fn try_from_overflow_signed() {
        let n = BigInt::from(u64::MAX);
        // i64 cannot hold u64::MAX.
        assert!(i64::try_from(&n).is_err());
    }

    #[test]
    fn try_from_negative_into_unsigned_errors() {
        let n = BigInt::from(-1i64);
        assert!(u32::try_from(&n).is_err());
    }
}
