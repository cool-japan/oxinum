//! `MpFloat` — a `rug::Float`-compatible adapter over `dashu-float`'s `DBig`.
//!
//! This module provides [`MpFloat`] and [`MpComplex`] as drop-in replacements
//! for the `rug::Float` and `rug::Complex` types used in scirs2-special's
//! `arbitrary_precision` module.  The API surface matches what that module
//! actually calls; it is **not** a complete re-implementation of the rug API.
//!
//! ## Design
//!
//! `MpFloat` wraps `dashu-float`'s `DBig` (decimal arbitrary-precision
//! big-float) and delegates all arithmetic to it.  Special functions
//! (gamma, erf, etc.) delegate to [`crate::special`].
//!
//! ## Precision
//!
//! `rug::Float` precision is measured in **binary bits**.  `DBig` precision
//! is in **decimal digits**.  The conversion used here is:
//!   `decimal_digits = ceil(bits * log10(2))` ≈ `bits * 0.30103`
//! plus a 5-digit guard margin, with a minimum of 10 decimal digits.

use crate::{
    special::{bessel_j0, catalan, digamma, erf, erfc, euler_gamma, gamma, ln_gamma},
    DBig,
};
use std::{
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

// ---------------------------------------------------------------------------
// Rounding mode (replaces rug::float::Round)
// ---------------------------------------------------------------------------

/// Rounding modes for arbitrary-precision operations.
///
/// This enum mirrors `rug::float::Round` for API compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Round {
    /// Round to nearest, ties away from zero (default).
    #[default]
    Nearest,
    /// Round toward positive infinity.
    Up,
    /// Round toward negative infinity.
    Down,
    /// Round toward zero.
    Zero,
}

// ---------------------------------------------------------------------------
// MpFloat
// ---------------------------------------------------------------------------

/// Arbitrary-precision floating-point number.
///
/// Wraps [`DBig`] with a precision (in **binary bits**) stored alongside so
/// that `prec()` returns the correct bit width.  All arithmetic is carried
/// out at the stored precision.
#[derive(Clone, Debug)]
pub struct MpFloat {
    value: DBig,
    bits: u32,
}

impl MpFloat {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a new `MpFloat` from an `f64` at the given **bit precision**.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::mp_float::MpFloat;
    /// let x = MpFloat::with_val(256, 3.14f64);
    /// assert!((x.to_f64() - 3.14).abs() < 1e-10);
    /// ```
    pub fn with_val(bits: u32, v: f64) -> Self {
        let prec = bits_to_decimal_prec(bits);
        let s = format!("{:.prec$e}", v, prec = prec + 5);
        let d = DBig::from_str(&s)
            .unwrap_or_else(|_| DBig::from_str(&format!("{v}")).expect("f64 to DBig: fallback"))
            .with_precision(prec)
            .value();
        Self { value: d, bits }
    }

    /// Create a new `MpFloat` from another `MpFloat` value at the given bit
    /// precision (analogous to `rug::Float::with_val(bits, &other)`).
    pub fn with_val_from(bits: u32, other: &MpFloat) -> Self {
        let prec = bits_to_decimal_prec(bits);
        Self {
            value: other.value.clone().with_precision(prec).value(),
            bits,
        }
    }

    /// Create from a `DBig` reference, using the existing precision.
    pub fn from_dbig(d: &DBig, bits: u32) -> Self {
        let prec = bits_to_decimal_prec(bits);
        Self {
            value: d.clone().with_precision(prec).value(),
            bits,
        }
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Return the precision in **binary bits**.
    pub fn prec(&self) -> u32 {
        self.bits
    }

    /// Return the decimal precision used internally.
    pub fn decimal_prec(&self) -> usize {
        bits_to_decimal_prec(self.bits)
    }

    /// Convert to `f64` (may lose precision).
    pub fn to_f64(&self) -> f64 {
        self.value.to_string().parse::<f64>().unwrap_or(0.0)
    }

    /// Borrow the inner `DBig`.
    pub fn as_dbig(&self) -> &DBig {
        &self.value
    }

    // -----------------------------------------------------------------------
    // Numeric predicates
    // -----------------------------------------------------------------------

    /// True if the value is exactly zero.
    pub fn is_zero(&self) -> bool {
        is_zero_dbig(&self.value)
    }

    /// True if the value is finite (always true for `DBig`).
    pub fn is_finite(&self) -> bool {
        // dashu-float DBig is always finite (no Inf/NaN)
        true
    }

    /// True if the value is an integer (fractional part is zero).
    pub fn is_integer(&self) -> bool {
        is_integer_dbig(&self.value)
    }

    /// True if the value is strictly positive.
    pub fn is_sign_positive(&self) -> bool {
        !self.value.to_string().starts_with('-') && !self.is_zero()
    }

    /// True if the value is strictly negative.
    pub fn is_sign_negative(&self) -> bool {
        self.value.to_string().starts_with('-') && !self.is_zero()
    }

    // -----------------------------------------------------------------------
    // Math operations
    // -----------------------------------------------------------------------

    /// Absolute value.
    pub fn abs(self) -> Self {
        let s = self.value.to_string();
        let positive = s.trim_start_matches('-');
        let d = DBig::from_str(positive)
            .unwrap_or_else(|_| self.value.clone())
            .with_precision(self.decimal_prec())
            .value();
        Self {
            value: d,
            bits: self.bits,
        }
    }

    /// Square root.  Returns 0 for negative inputs (mirrors MPFR NaN→0 fallback).
    pub fn sqrt(self) -> Self {
        if self.is_sign_negative() {
            return Self::with_val(self.bits, 0.0);
        }
        let prec = self.decimal_prec();
        match crate::elementary::sqrt(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Exponential e^x.
    pub fn exp(self) -> Self {
        let prec = self.decimal_prec();
        match crate::elementary::exp(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Natural logarithm.  Returns 0 for x ≤ 0 (fallback).
    pub fn ln(self) -> Self {
        let prec = self.decimal_prec();
        match crate::elementary::ln(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Sine.
    pub fn sin(self) -> Self {
        let prec = self.decimal_prec();
        match crate::trig::sin(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Cosine.
    pub fn cos(self) -> Self {
        let prec = self.decimal_prec();
        match crate::trig::cos(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Gamma function Γ(self).
    pub fn gamma(self) -> Self {
        let prec = self.decimal_prec();
        match gamma(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, f64::NAN),
        }
    }

    /// Apply gamma function in-place (mirrors `rug::Float::gamma_mut()`).
    pub fn gamma_mut(&mut self) {
        let prec = self.decimal_prec();
        if let Ok(v) = gamma(&self.value, prec) {
            self.value = v;
        }
    }

    /// Log-gamma function ln(Γ(self)).
    pub fn ln_gamma(self) -> Self {
        let prec = self.decimal_prec();
        match ln_gamma(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, f64::NAN),
        }
    }

    /// Apply log-gamma in-place (mirrors `rug::Float::ln_gamma_mut()`).
    pub fn ln_gamma_mut(&mut self) {
        let prec = self.decimal_prec();
        if let Ok(v) = ln_gamma(&self.value, prec) {
            self.value = v;
        }
    }

    /// Digamma function ψ(self).
    pub fn digamma(self) -> Self {
        let prec = self.decimal_prec();
        match digamma(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, f64::NAN),
        }
    }

    /// Apply digamma in-place (mirrors `rug::Float::digamma_mut()`).
    pub fn digamma_mut(&mut self) {
        let prec = self.decimal_prec();
        if let Ok(v) = digamma(&self.value, prec) {
            self.value = v;
        }
    }

    /// Error function erf(self).
    pub fn erf(self) -> Self {
        let prec = self.decimal_prec();
        match erf(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Apply erf in-place (mirrors `rug::Float::erf_mut()`).
    pub fn erf_mut(&mut self) {
        let prec = self.decimal_prec();
        if let Ok(v) = erf(&self.value, prec) {
            self.value = v;
        }
    }

    /// Complementary error function erfc(self).
    pub fn erfc(self) -> Self {
        let prec = self.decimal_prec();
        match erfc(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Apply erfc in-place (mirrors `rug::Float::erfc_mut()`).
    pub fn erfc_mut(&mut self) {
        let prec = self.decimal_prec();
        if let Ok(v) = erfc(&self.value, prec) {
            self.value = v;
        }
    }

    /// Bessel function J₀(self).
    pub fn j0(self) -> Self {
        let prec = self.decimal_prec();
        match bessel_j0(&self.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Apply j0 in-place (mirrors `rug::Float::j0_mut()`).
    pub fn j0_mut(&mut self) {
        let prec = self.decimal_prec();
        if let Ok(v) = bessel_j0(&self.value, prec) {
            self.value = v;
        }
    }

    /// Raise to the power of another `MpFloat`.
    pub fn pow_float(self, exp: &MpFloat) -> Self {
        let prec = self.decimal_prec();
        match crate::elementary::pow(&self.value, &exp.value, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Raise to an integer power.
    pub fn pow_i32(self, exp: i32) -> Self {
        let prec = self.decimal_prec();
        let exp_d = dbig_f64(exp as f64, prec);
        match crate::elementary::pow(&self.value, &exp_d, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }

    /// Raise to an f64 power (convenience).
    pub fn pow_f64(self, exp: f64) -> Self {
        let prec = self.decimal_prec();
        let exp_d = dbig_f64(exp, prec);
        match crate::elementary::pow(&self.value, &exp_d, prec) {
            Ok(v) => Self {
                value: v,
                bits: self.bits,
            },
            Err(_) => Self::with_val(self.bits, 0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Display and scientific notation formatting
// ---------------------------------------------------------------------------

impl fmt::Display for MpFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // If a precision was requested, use f64 for the format (sufficient for
        // typical display purposes).  Without requested precision, show full DBig.
        if let Some(prec) = f.precision() {
            write!(f, "{:.prec$}", self.to_f64(), prec = prec)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl fmt::LowerExp for MpFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(prec) = f.precision() {
            write!(f, "{:.prec$e}", self.to_f64(), prec = prec)
        } else {
            write!(f, "{:e}", self.to_f64())
        }
    }
}

impl fmt::UpperExp for MpFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(prec) = f.precision() {
            write!(f, "{:.prec$E}", self.to_f64(), prec = prec)
        } else {
            write!(f, "{:E}", self.to_f64())
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

impl PartialEq for MpFloat {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for MpFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl PartialEq<f64> for MpFloat {
    fn eq(&self, other: &f64) -> bool {
        (self.to_f64() - other).abs() < 1e-300
    }
}

impl PartialOrd<f64> for MpFloat {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.to_f64().partial_cmp(other)
    }
}

// ---------------------------------------------------------------------------
// Arithmetic ops (owned × owned)
// ---------------------------------------------------------------------------

impl Add for MpFloat {
    type Output = MpFloat;
    fn add(self, rhs: MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value + &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Sub for MpFloat {
    type Output = MpFloat;
    fn sub(self, rhs: MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value - &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Mul for MpFloat {
    type Output = MpFloat;
    fn mul(self, rhs: MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value * &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Div for MpFloat {
    type Output = MpFloat;
    fn div(self, rhs: MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value / &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

// ---------------------------------------------------------------------------
// Arithmetic ops (owned × &ref)
// ---------------------------------------------------------------------------

impl Add<&MpFloat> for MpFloat {
    type Output = MpFloat;
    fn add(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value + &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Sub<&MpFloat> for MpFloat {
    type Output = MpFloat;
    fn sub(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value - &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Mul<&MpFloat> for MpFloat {
    type Output = MpFloat;
    fn mul(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value * &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Div<&MpFloat> for MpFloat {
    type Output = MpFloat;
    fn div(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value / &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

// ---------------------------------------------------------------------------
// Arithmetic ops (&ref × &ref)
// ---------------------------------------------------------------------------

impl Add<&MpFloat> for &MpFloat {
    type Output = MpFloat;
    fn add(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value + &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Sub<&MpFloat> for &MpFloat {
    type Output = MpFloat;
    fn sub(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value - &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Mul<&MpFloat> for &MpFloat {
    type Output = MpFloat;
    fn mul(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value * &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

impl Div<&MpFloat> for &MpFloat {
    type Output = MpFloat;
    fn div(self, rhs: &MpFloat) -> MpFloat {
        let prec = self.bits.max(rhs.bits);
        let d = (&self.value / &rhs.value)
            .with_precision(bits_to_decimal_prec(prec))
            .value();
        MpFloat {
            value: d,
            bits: prec,
        }
    }
}

// ---------------------------------------------------------------------------
// f64 arithmetic
// ---------------------------------------------------------------------------

impl Add<f64> for MpFloat {
    type Output = MpFloat;
    fn add(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self + rhs_mp
    }
}

impl Sub<f64> for MpFloat {
    type Output = MpFloat;
    fn sub(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self - rhs_mp
    }
}

impl Mul<f64> for MpFloat {
    type Output = MpFloat;
    fn mul(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self * rhs_mp
    }
}

impl Div<f64> for MpFloat {
    type Output = MpFloat;
    fn div(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self / rhs_mp
    }
}

impl Add<f64> for &MpFloat {
    type Output = MpFloat;
    fn add(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self + &rhs_mp
    }
}

impl Sub<f64> for &MpFloat {
    type Output = MpFloat;
    fn sub(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self - &rhs_mp
    }
}

impl Mul<f64> for &MpFloat {
    type Output = MpFloat;
    fn mul(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self * &rhs_mp
    }
}

impl Div<f64> for &MpFloat {
    type Output = MpFloat;
    fn div(self, rhs: f64) -> MpFloat {
        let rhs_mp = MpFloat::with_val(self.bits, rhs);
        self / &rhs_mp
    }
}

// ---------------------------------------------------------------------------
// Negation
// ---------------------------------------------------------------------------

impl Neg for MpFloat {
    type Output = MpFloat;
    fn neg(self) -> MpFloat {
        let bits = self.bits;
        let s = self.value.to_string();
        let neg_s = if let Some(stripped) = s.strip_prefix('-') {
            stripped.to_string()
        } else if s == "0" || s == "0.0" {
            s
        } else {
            format!("-{s}")
        };
        let d = DBig::from_str(&neg_s)
            .unwrap_or_else(|_| DBig::from_str("0.0").expect("zero"))
            .with_precision(bits_to_decimal_prec(bits))
            .value();
        MpFloat { value: d, bits }
    }
}

impl Neg for &MpFloat {
    type Output = MpFloat;
    fn neg(self) -> MpFloat {
        self.clone().neg()
    }
}

// ---------------------------------------------------------------------------
// Assign ops
// ---------------------------------------------------------------------------

impl AddAssign<&MpFloat> for MpFloat {
    fn add_assign(&mut self, rhs: &MpFloat) {
        let new = &*self + rhs;
        *self = new;
    }
}

impl AddAssign<MpFloat> for MpFloat {
    fn add_assign(&mut self, rhs: MpFloat) {
        let new = &*self + &rhs;
        *self = new;
    }
}

impl AddAssign<f64> for MpFloat {
    fn add_assign(&mut self, rhs: f64) {
        let new = self.clone() + rhs;
        *self = new;
    }
}

impl SubAssign<&MpFloat> for MpFloat {
    fn sub_assign(&mut self, rhs: &MpFloat) {
        let new = &*self - rhs;
        *self = new;
    }
}

impl SubAssign<MpFloat> for MpFloat {
    fn sub_assign(&mut self, rhs: MpFloat) {
        let new = &*self - &rhs;
        *self = new;
    }
}

impl SubAssign<f64> for MpFloat {
    fn sub_assign(&mut self, rhs: f64) {
        let new = self.clone() - rhs;
        *self = new;
    }
}

impl MulAssign<&MpFloat> for MpFloat {
    fn mul_assign(&mut self, rhs: &MpFloat) {
        let new = &*self * rhs;
        *self = new;
    }
}

impl MulAssign<MpFloat> for MpFloat {
    fn mul_assign(&mut self, rhs: MpFloat) {
        let new = &*self * &rhs;
        *self = new;
    }
}

impl MulAssign<f64> for MpFloat {
    fn mul_assign(&mut self, rhs: f64) {
        let new = self.clone() * rhs;
        *self = new;
    }
}

impl DivAssign<f64> for MpFloat {
    fn div_assign(&mut self, rhs: f64) {
        let new = self.clone() / rhs;
        *self = new;
    }
}

impl DivAssign<MpFloat> for MpFloat {
    fn div_assign(&mut self, rhs: MpFloat) {
        let new = self.clone() / rhs;
        *self = new;
    }
}

impl DivAssign<&MpFloat> for MpFloat {
    fn div_assign(&mut self, rhs: &MpFloat) {
        let new = self.clone() / rhs;
        *self = new;
    }
}

// ---------------------------------------------------------------------------
// Pow trait (mirrors rug::ops::Pow)
// ---------------------------------------------------------------------------

/// Pow implementation for `MpFloat` — `base.pow(exp)`.
pub trait MpPow<T> {
    fn pow(self, exp: T) -> MpFloat;
}

impl MpPow<&MpFloat> for MpFloat {
    fn pow(self, exp: &MpFloat) -> MpFloat {
        self.pow_float(exp)
    }
}

impl MpPow<MpFloat> for MpFloat {
    fn pow(self, exp: MpFloat) -> MpFloat {
        self.pow_float(&exp)
    }
}

impl MpPow<f64> for MpFloat {
    fn pow(self, exp: f64) -> MpFloat {
        self.pow_f64(exp)
    }
}

impl MpPow<i32> for MpFloat {
    fn pow(self, exp: i32) -> MpFloat {
        self.pow_i32(exp)
    }
}

// ---------------------------------------------------------------------------
// MpComplex
// ---------------------------------------------------------------------------

/// Arbitrary-precision complex number.
///
/// Stores real and imaginary parts as [`MpFloat`] at a shared bit precision.
/// This mirrors enough of `rug::Complex` to replace its usage in
/// `scirs2-special::arbitrary_precision`.
#[derive(Clone, Debug)]
pub struct MpComplex {
    re: MpFloat,
    im: MpFloat,
    bits: u32,
}

impl MpComplex {
    /// Create a complex number from `(real, imag)` f64 pair.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_float::mp_float::MpComplex;
    /// let z = MpComplex::with_val(256, (1.0f64, 2.0f64));
    /// assert!((z.real().to_f64() - 1.0).abs() < 1e-10);
    /// assert!((z.imag().to_f64() - 2.0).abs() < 1e-10);
    /// ```
    pub fn with_val(bits: u32, (re, im): (f64, f64)) -> Self {
        Self {
            re: MpFloat::with_val(bits, re),
            im: MpFloat::with_val(bits, im),
            bits,
        }
    }

    /// Borrow the real part.
    pub fn real(&self) -> &MpFloat {
        &self.re
    }

    /// Borrow the imaginary part.
    pub fn imag(&self) -> &MpFloat {
        &self.im
    }

    /// Decompose into `(real, imag)` `MpFloat` pair (mirrors
    /// `rug::Complex::into_real_imag()`).
    pub fn into_real_imag(self) -> (MpFloat, MpFloat) {
        (self.re, self.im)
    }

    /// Precision in bits.
    pub fn prec(&self) -> u32 {
        self.bits
    }
}

// ---------------------------------------------------------------------------
// PrecisionContext helper: convert bits → decimal prec
// ---------------------------------------------------------------------------

/// Convert a **bit** precision to an approximate **decimal digit** count.
///
/// Formula: `ceil(bits * log10(2))` + 5 guard digits, minimum 10.
pub fn bits_to_decimal_prec(bits: u32) -> usize {
    let digits = (bits as f64 * std::f64::consts::LOG2_10.recip()).ceil() as usize;
    digits.max(10) + 5
}

/// Build a `DBig` constant for the Euler-Mascheroni constant at the given bit
/// precision. Convenience wrapper so PrecisionContext doesn't need to import
/// special module.
pub fn euler_gamma_at_bits(bits: u32) -> MpFloat {
    let prec = bits_to_decimal_prec(bits);
    let d = euler_gamma(prec);
    MpFloat { value: d, bits }
}

/// Build Catalan's constant at the given bit precision.
pub fn catalan_at_bits(bits: u32) -> MpFloat {
    let prec = bits_to_decimal_prec(bits);
    let d = catalan(prec);
    MpFloat { value: d, bits }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn is_zero_dbig(v: &DBig) -> bool {
    let s = v.to_string().trim_start_matches('-').to_string();
    s == "0" || s == "0.0" || {
        if let Some(dot) = s.find('.') {
            let int = &s[..dot];
            let frac = &s[dot + 1..];
            int == "0" && frac.chars().all(|c| c == '0')
        } else {
            s.chars().all(|c| c == '0')
        }
    }
}

fn is_integer_dbig(v: &DBig) -> bool {
    let s = v.to_string().trim_start_matches('-').to_string();
    if let Some(dot) = s.find('.') {
        let frac = &s[dot + 1..];
        frac.chars().all(|c| c == '0')
    } else {
        true
    }
}

fn dbig_f64(v: f64, precision: usize) -> DBig {
    let s = format!("{:.prec$e}", v, prec = precision + 5);
    DBig::from_str(&s)
        .unwrap_or_else(|_| DBig::from_str(&format!("{v}")).expect("f64 to DBig fallback"))
        .with_precision(precision)
        .value()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_val_pi() {
        let x = MpFloat::with_val(256, std::f64::consts::PI);
        let v = x.to_f64();
        assert!((v - std::f64::consts::PI).abs() < 1e-10, "pi = {v}");
    }

    #[test]
    fn arithmetic_ops() {
        let a = MpFloat::with_val(128, 3.0);
        let b = MpFloat::with_val(128, 2.0);
        assert!(((&a + &b).to_f64() - 5.0).abs() < 1e-10);
        assert!(((&a - &b).to_f64() - 1.0).abs() < 1e-10);
        assert!(((&a * &b).to_f64() - 6.0).abs() < 1e-10);
        assert!(((&a / &b).to_f64() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn sqrt_exp_ln() {
        let x = MpFloat::with_val(128, 4.0);
        let s = x.sqrt();
        assert!((s.to_f64() - 2.0).abs() < 1e-8, "sqrt(4) = {}", s.to_f64());

        let e = MpFloat::with_val(128, 1.0).exp();
        assert!((e.to_f64() - std::f64::consts::E).abs() < 1e-8);

        let l = MpFloat::with_val(128, std::f64::consts::E).ln();
        assert!((l.to_f64() - 1.0).abs() < 1e-8);
    }

    #[test]
    fn is_zero_predicate() {
        let z = MpFloat::with_val(128, 0.0);
        let nz = MpFloat::with_val(128, 1.0);
        assert!(z.is_zero());
        assert!(!nz.is_zero());
    }

    #[test]
    fn is_integer_predicate() {
        let i = MpFloat::with_val(128, 5.0);
        let f = MpFloat::with_val(128, 5.5);
        assert!(i.is_integer());
        assert!(!f.is_integer());
    }

    #[test]
    fn gamma_mut_test() {
        let mut x = MpFloat::with_val(128, 5.0);
        x.gamma_mut();
        // Γ(5) = 4! = 24
        assert!(
            (x.to_f64() - 24.0).abs() < 1e-5,
            "gamma(5) = {}",
            x.to_f64()
        );
    }

    #[test]
    fn erf_mut_test() {
        let mut x = MpFloat::with_val(128, 0.0);
        x.erf_mut();
        assert!(x.is_zero());
    }

    #[test]
    fn neg_operator() {
        let x = MpFloat::with_val(128, 3.0);
        let neg_x = -x;
        assert!(neg_x.is_sign_negative());
        assert!((neg_x.to_f64() + 3.0).abs() < 1e-10);
    }

    #[test]
    fn complex_into_parts() {
        let z = MpComplex::with_val(128, (1.5_f64, 2.5_f64));
        let (re, im) = z.into_real_imag();
        assert!((re.to_f64() - 1.5).abs() < 1e-10);
        assert!((im.to_f64() - 2.5).abs() < 1e-10);
    }

    #[test]
    fn euler_gamma_const() {
        let g = euler_gamma_at_bits(256);
        let v = g.to_f64();
        assert!((v - 0.5772156649_f64).abs() < 1e-8, "euler_gamma = {v}");
    }

    #[test]
    fn catalan_const() {
        let g = catalan_at_bits(256);
        let v = g.to_f64();
        assert!((v - 0.9159655942_f64).abs() < 1e-8, "catalan = {v}");
    }
}
