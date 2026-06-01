//! Conversions between [`RBig`] and primitive floating-point / string forms.
//!
//! This module wraps `dashu`'s already-exact bit-level decoding of `f64` /
//! `f32` (via [`TryFrom<f64>`] / [`TryFrom<f32>`]) plus the `to_f64()` and
//! `TryFrom<RBig> for f64` paths and exposes them through the
//! [`OxiNumError`] result type expected by the rest of the OxiNum
//! ecosystem.  It also adds a `parse_mixed` free function and a
//! `MixedNumber` newtype that implements `FromStr` for the human-friendly
//! mixed-number format (e.g. `"1 3/4"`, `"-2 1/3"`).
//!
//! The float decoding is exact (no rounding) because IEEE 754 `f32` /
//! `f64` are dyadic rationals; representing them as `RBig` is therefore
//! lossless.  See `from_f64` / `from_f32` below.
//!
//! All routines are `#![forbid(unsafe_code)]`; the bit-level work is done
//! through safe primitives such as `f64::to_bits()` and `dashu_base`'s
//! `decode()`.

use core::str::FromStr;

use dashu_base::ConversionError;

use crate::{IBig, RBig, UBig};
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// Float -> Rational (exact)
// ---------------------------------------------------------------------------

/// Convert an `f64` to an exact rational.
///
/// IEEE-754 `f64` values are dyadic rationals (`mantissa * 2^exp` for some
/// signed `exp`), so finite floats round-trip exactly.
///
/// # Errors
///
/// Returns [`OxiNumError::Parse`] if `x` is NaN or infinite.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{from_f64, RBig, IBig, UBig};
/// // 0.5 exactly = 1/2
/// let r = from_f64(0.5).unwrap();
/// assert_eq!(r, RBig::from_parts(IBig::from(1), UBig::from(2u32)));
/// // 0.25 exactly = 1/4
/// assert_eq!(from_f64(0.25).unwrap(),
///            RBig::from_parts(IBig::from(1), UBig::from(4u32)));
/// // NaN errors
/// assert!(from_f64(f64::NAN).is_err());
/// assert!(from_f64(f64::INFINITY).is_err());
/// ```
pub fn from_f64(x: f64) -> OxiNumResult<RBig> {
    RBig::try_from(x).map_err(|_| OxiNumError::Parse(format!("non-finite f64: {x}").into()))
}

/// Convert an `f32` to an exact rational.
///
/// IEEE-754 `f32` values are dyadic rationals, so finite floats round-trip
/// exactly.
///
/// # Errors
///
/// Returns [`OxiNumError::Parse`] if `x` is NaN or infinite.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{from_f32, RBig, IBig, UBig};
/// let r = from_f32(0.5).unwrap();
/// assert_eq!(r, RBig::from_parts(IBig::from(1), UBig::from(2u32)));
/// assert!(from_f32(f32::NAN).is_err());
/// ```
pub fn from_f32(x: f32) -> OxiNumResult<RBig> {
    RBig::try_from(x).map_err(|_| OxiNumError::Parse(format!("non-finite f32: {x}").into()))
}

// ---------------------------------------------------------------------------
// Rational -> Float
// ---------------------------------------------------------------------------

/// Convert a rational to the nearest `f64` (round-to-nearest, ties-to-even).
///
/// Returns `f64::INFINITY` / `f64::NEG_INFINITY` if the magnitude is
/// outside the representable range, and `0.0` for underflow.  Use
/// [`to_f64_exact`] if you need to detect precision loss.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{to_f64, RBig, IBig, UBig};
/// let half = RBig::from_parts(IBig::from(1), UBig::from(2u32));
/// assert_eq!(to_f64(&half), 0.5);
/// let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
/// assert!((to_f64(&third) - 1.0_f64 / 3.0).abs() < 1e-15);
/// ```
pub fn to_f64(x: &RBig) -> f64 {
    x.to_f64().value()
}

/// Convert a rational to an `f64`, requiring exact representability.
///
/// # Errors
///
/// - [`OxiNumError::Overflow`] if the magnitude is outside the
///   representable `f64` range.
/// - [`OxiNumError::Precision`] if the value cannot be represented
///   without rounding (e.g. `1/3`).
///
/// # Examples
///
/// ```
/// use oxinum_rational::{to_f64_exact, RBig, IBig, UBig};
/// // 1/2 is exact
/// let half = RBig::from_parts(IBig::from(1), UBig::from(2u32));
/// assert_eq!(to_f64_exact(&half).unwrap(), 0.5);
/// // 1/3 is not representable
/// let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
/// assert!(to_f64_exact(&third).is_err());
/// ```
pub fn to_f64_exact(x: &RBig) -> OxiNumResult<f64> {
    match f64::try_from(x.clone()) {
        Ok(v) => Ok(v),
        Err(ConversionError::OutOfBounds) => Err(OxiNumError::Overflow(
            format!("rational {x} exceeds f64 range").into(),
        )),
        Err(ConversionError::LossOfPrecision) => Err(OxiNumError::Precision(
            format!("rational {x} not exactly representable as f64").into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Mixed-number parsing
// ---------------------------------------------------------------------------

/// Parse a mixed-number string.
///
/// Accepts the following forms:
///
/// * `"3"` — integer (delegates to `RBig::from_str`)
/// * `"3/4"` — proper or improper fraction (delegates to `RBig::from_str`)
/// * `"-3/4"`, `"3/-4"` — signed fractions (delegates to `RBig::from_str`)
/// * `"1 3/4"` — mixed number, equal to `7/4`
/// * `"-2 1/3"` — negative mixed number, equal to `-7/3`
///   (the sign binds to the whole value, not just the integer part)
///
/// Whitespace between the integer and fractional parts is any sequence of
/// ASCII whitespace.
///
/// # Errors
///
/// Returns [`OxiNumError::Parse`] if either part fails to parse, if the
/// fractional component is not positive, or if the string is malformed.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{parse_mixed, RBig, IBig, UBig};
/// let r = parse_mixed("1 3/4").unwrap();
/// assert_eq!(r, RBig::from_parts(IBig::from(7), UBig::from(4u32)));
/// let neg = parse_mixed("-2 1/3").unwrap();
/// assert_eq!(neg, RBig::from_parts(IBig::from(-7), UBig::from(3u32)));
/// let plain = parse_mixed("3/4").unwrap();
/// assert_eq!(plain, RBig::from_parts(IBig::from(3), UBig::from(4u32)));
/// ```
pub fn parse_mixed(s: &str) -> OxiNumResult<RBig> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(OxiNumError::Parse("empty mixed-number string".into()));
    }

    // Locate the first whitespace gap.  If none, fall through to the plain
    // RBig parser (which accepts "3", "3/4", "-3/4", etc.).
    let split_at = trimmed.find(char::is_whitespace);
    let Some(idx) = split_at else {
        return RBig::from_str(trimmed)
            .map_err(|e| OxiNumError::Parse(format!("invalid rational: {e}").into()));
    };

    let int_part_str = &trimmed[..idx];
    let frac_part_str = trimmed[idx..].trim_start();
    if frac_part_str.is_empty() {
        return Err(OxiNumError::Parse(
            "missing fractional part in mixed number".into(),
        ));
    }
    if !frac_part_str.contains('/') {
        return Err(OxiNumError::Parse(
            format!("expected fraction after whole part, got {frac_part_str:?}").into(),
        ));
    }

    let int_part = IBig::from_str(int_part_str).map_err(|e| {
        OxiNumError::Parse(format!("invalid integer part {int_part_str:?}: {e}").into())
    })?;
    let frac_part = RBig::from_str(frac_part_str).map_err(|e| {
        OxiNumError::Parse(format!("invalid fractional part {frac_part_str:?}: {e}").into())
    })?;

    // Well-formed mixed numbers have a non-negative fractional part.
    if frac_part.numerator() < &IBig::ZERO {
        return Err(OxiNumError::Parse(
            "fractional part of a mixed number must be non-negative".into(),
        ));
    }

    // Combine: the sign of the integer part rules the whole value.  We
    // build `|int| + frac` then negate if necessary.  This correctly
    // handles `"-0 1/3"` (rare but valid) as `+1/3`.
    let neg = int_part < IBig::ZERO;
    let abs_int = if neg { -&int_part } else { int_part };
    let combined = RBig::from(abs_int) + frac_part;
    let result = if neg { -combined } else { combined };
    Ok(result)
}

/// Newtype wrapper providing [`FromStr`] for the mixed-number format.
///
/// `RBig` already implements `FromStr` for the standard `"a/b"` syntax;
/// this newtype extends parsing to also accept `"1 3/4"` while keeping
/// the orphan rule satisfied.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{MixedNumber, RBig, IBig, UBig};
/// use std::str::FromStr;
/// let m: MixedNumber = "1 3/4".parse().unwrap();
/// assert_eq!(m.0, RBig::from_parts(IBig::from(7), UBig::from(4u32)));
/// let n = MixedNumber::from_str("-2 1/3").unwrap();
/// assert_eq!(n.0, RBig::from_parts(IBig::from(-7), UBig::from(3u32)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MixedNumber(pub RBig);

impl FromStr for MixedNumber {
    type Err = OxiNumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_mixed(s).map(MixedNumber)
    }
}

impl core::fmt::Display for MixedNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Render as a mixed number when the absolute value is improper
        // (|num| > den) and the denominator is not 1.  Falls back to the
        // standard `RBig` display otherwise.
        let num = self.0.numerator();
        let den_u: &UBig = self.0.denominator();
        let den_i: IBig = den_u.clone().into();
        if den_i == IBig::ONE {
            return write!(f, "{}", num);
        }
        let abs_num = if num < &IBig::ZERO {
            -num.clone()
        } else {
            num.clone()
        };
        if abs_num < den_i {
            // proper fraction
            return write!(f, "{}", self.0);
        }
        let whole = &abs_num / &den_i;
        let rem = abs_num - &whole * &den_i;
        let sign = if num < &IBig::ZERO { "-" } else { "" };
        if rem == IBig::ZERO {
            write!(f, "{}{}", sign, whole)
        } else {
            write!(f, "{}{} {}/{}", sign, whole, rem, den_i)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_f64_half() {
        let r = from_f64(0.5).expect("finite");
        assert_eq!(r, RBig::from_parts(IBig::from(1), UBig::from(2u32)));
    }

    #[test]
    fn from_f64_negative() {
        let r = from_f64(-0.125).expect("finite");
        assert_eq!(r, RBig::from_parts(IBig::from(-1), UBig::from(8u32)));
    }

    #[test]
    fn from_f64_zero() {
        let r = from_f64(0.0).expect("finite");
        assert_eq!(r, RBig::ZERO);
    }

    #[test]
    fn from_f64_one_tenth_is_dyadic() {
        // 0.1 in binary is the dyadic fraction
        //   3602879701896397 / 2^55
        // so the exact RBig must reduce to numerator 3602879701896397,
        // denominator 2^55 / gcd(...) and the value must equal the original f64.
        let original = 0.1_f64;
        let r = from_f64(original).expect("finite");
        // Round-trip back through to_f64_exact must succeed (it's dyadic).
        let back = to_f64_exact(&r).expect("exact");
        assert_eq!(back, original);
        // The denominator must be a power of two.
        let den = r.denominator().clone();
        let den_i: IBig = den.into();
        // Repeatedly halve via division: a power of two has bit_count == 1.
        let two = IBig::from(2);
        let mut count_bits = 0u32;
        let mut tmp = den_i;
        while tmp > IBig::ZERO {
            if (&tmp % &two) == IBig::ONE {
                count_bits += 1;
            }
            tmp /= &two;
        }
        assert_eq!(count_bits, 1, "0.1 denominator must be a power of two");
    }

    #[test]
    fn from_f64_nan_errors() {
        assert!(from_f64(f64::NAN).is_err());
    }

    #[test]
    fn from_f64_inf_errors() {
        assert!(from_f64(f64::INFINITY).is_err());
        assert!(from_f64(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn from_f32_basic() {
        let r = from_f32(0.5_f32).expect("finite");
        assert_eq!(r, RBig::from_parts(IBig::from(1), UBig::from(2u32)));
        assert!(from_f32(f32::NAN).is_err());
        assert!(from_f32(f32::INFINITY).is_err());
    }

    #[test]
    fn to_f64_third() {
        let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        let v = to_f64(&third);
        assert!((v - 1.0_f64 / 3.0).abs() < 1e-15);
    }

    #[test]
    fn to_f64_exact_dyadic_ok() {
        let r = RBig::from_parts(IBig::from(7), UBig::from(8u32));
        assert_eq!(to_f64_exact(&r).expect("exact"), 0.875);
    }

    #[test]
    fn to_f64_exact_third_errors() {
        let r = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        assert!(matches!(to_f64_exact(&r), Err(OxiNumError::Precision(_))));
    }

    #[test]
    fn f64_roundtrip_dyadic() {
        // Every dyadic rational that fits in f64 must round-trip exactly.
        for &orig in &[
            0.5_f64,
            -0.5,
            0.25,
            -0.125,
            0.875,
            1.0,
            -1.0,
            2.0_f64.powi(20),
            2.0_f64.powi(-20),
            std::f64::consts::PI, // dyadic representation of pi-as-f64
        ] {
            let r = from_f64(orig).expect("finite");
            let back = to_f64_exact(&r).expect("exact roundtrip");
            assert_eq!(back, orig, "roundtrip failed for {orig}");
        }
    }

    #[test]
    fn parse_mixed_basic() {
        let r = parse_mixed("1 3/4").expect("ok");
        assert_eq!(r, RBig::from_parts(IBig::from(7), UBig::from(4u32)));
    }

    #[test]
    fn parse_mixed_negative() {
        // "-2 1/3" must be -7/3, NOT -5/3
        let r = parse_mixed("-2 1/3").expect("ok");
        assert_eq!(r, RBig::from_parts(IBig::from(-7), UBig::from(3u32)));
    }

    #[test]
    fn parse_mixed_plain_fraction() {
        let r = parse_mixed("3/4").expect("ok");
        assert_eq!(r, RBig::from_parts(IBig::from(3), UBig::from(4u32)));
    }

    #[test]
    fn parse_mixed_plain_integer() {
        let r = parse_mixed("5").expect("ok");
        assert_eq!(r, RBig::from(5u32));
    }

    #[test]
    fn parse_mixed_with_whitespace() {
        let r = parse_mixed("  1   3/4  ").expect("ok");
        assert_eq!(r, RBig::from_parts(IBig::from(7), UBig::from(4u32)));
    }

    #[test]
    fn parse_mixed_empty_errors() {
        assert!(parse_mixed("").is_err());
        assert!(parse_mixed("   ").is_err());
    }

    #[test]
    fn parse_mixed_bad_integer_errors() {
        assert!(parse_mixed("abc 1/2").is_err());
    }

    #[test]
    fn parse_mixed_bad_fraction_errors() {
        assert!(parse_mixed("1 xyz").is_err());
        assert!(parse_mixed("1 2").is_err()); // missing slash
    }

    #[test]
    fn parse_mixed_negative_fractional_rejected() {
        assert!(parse_mixed("1 -1/2").is_err());
    }

    #[test]
    fn mixed_number_from_str() {
        let m: MixedNumber = "1 3/4".parse().expect("ok");
        assert_eq!(m.0, RBig::from_parts(IBig::from(7), UBig::from(4u32)));
    }

    #[test]
    fn mixed_number_display_proper() {
        let m = MixedNumber(RBig::from_parts(IBig::from(3), UBig::from(4u32)));
        assert_eq!(m.to_string(), "3/4");
    }

    #[test]
    fn mixed_number_display_improper() {
        let m = MixedNumber(RBig::from_parts(IBig::from(7), UBig::from(4u32)));
        assert_eq!(m.to_string(), "1 3/4");
    }

    #[test]
    fn mixed_number_display_negative_improper() {
        let m = MixedNumber(RBig::from_parts(IBig::from(-7), UBig::from(3u32)));
        assert_eq!(m.to_string(), "-2 1/3");
    }

    #[test]
    fn mixed_number_display_integer() {
        let m = MixedNumber(RBig::from(5u32));
        assert_eq!(m.to_string(), "5");
    }

    #[test]
    fn mixed_number_roundtrip_display_parse() {
        // "1 3/4" -> 7/4 -> Display -> "1 3/4" -> parse -> 7/4
        let m1: MixedNumber = "1 3/4".parse().expect("ok");
        let rendered = m1.to_string();
        assert_eq!(rendered, "1 3/4");
        let m2: MixedNumber = rendered.parse().expect("ok");
        assert_eq!(m1, m2);
    }

    // ----- serde JSON round-trip (feature-gated) ---------------------------

    #[cfg(feature = "serde")]
    #[test]
    fn rbig_serde_json_roundtrip() {
        let r = RBig::from_parts(IBig::from(355), UBig::from(113u32));
        let json = serde_json::to_string(&r).expect("serialize");
        let back: RBig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn rbig_serde_json_roundtrip_negative() {
        let r = RBig::from_parts(IBig::from(-7), UBig::from(3u32));
        let json = serde_json::to_string(&r).expect("serialize");
        let back: RBig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn mixed_number_serde_json_roundtrip() {
        let m = MixedNumber(RBig::from_parts(IBig::from(7), UBig::from(4u32)));
        let json = serde_json::to_string(&m).expect("serialize");
        let back: MixedNumber = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, back);
    }
}
