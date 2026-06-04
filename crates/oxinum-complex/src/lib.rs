#![forbid(unsafe_code)]
//! Arbitrary-precision complex arithmetic for the OxiNum ecosystem.
//!
//! Provides [`CBig`], a complex number whose real and imaginary parts are
//! each a [`DBig`] (decimal arbitrary-precision float re-exported from
//! [`oxinum_float`]). Everything is Pure Rust — no GMP, no MPFR, no C/C++
//! — so the type works unchanged on every platform the rest of the
//! COOLJAPAN stack targets.
//!
//! `CBig` is the high-level, decimal-backed front end. For a ground-up
//! binary-base complex built directly on [`oxinum_float::native::BigFloat`]
//! with explicit rounding-mode control, see [`native::BigComplex`].
//!
//! # Layout
//!
//! A `CBig` is the ordered pair `(re, im)` representing `re + im·i`. The two
//! components are independent `DBig` values and may carry different
//! precisions; arithmetic and transcendental routines (added by sibling
//! modules) reconcile precision as needed.
//!
//! # Why `Hash`, `Eq`, and `Ord` are not implemented
//!
//! * **No `Ord` / `PartialOrd`.** The complex field is *not* ordered: there
//!   is no order relation compatible with the ring structure, so any total
//!   order would be arbitrary and mathematically misleading. Callers who
//!   need a sort key should compare on a derived scalar (e.g. magnitude or
//!   the lexicographic `(re, im)` pair) explicitly.
//! * **No `Hash` / `Eq`.** `CBig` is built from `DBig`, which deliberately
//!   does **not** implement [`core::hash::Hash`] (distinct representations
//!   can compare equal across precisions, mirroring the IEEE-754 reasons
//!   `f32`/`f64` skip `Hash` in the standard library). `PartialEq` is
//!   provided (component-wise) where useful, but a lawful `Eq`/`Hash` pair
//!   would require a canonicalisation policy that varies by use case.
//!
//! # Examples
//!
//! ```
//! use oxinum_complex::CBig;
//!
//! let z = CBig::from_f64(3.0, 4.0).expect("finite parts");
//! // |3 + 4i|^2 = 9 + 16 = 25
//! assert_eq!(z.norm_sqr().to_string(), "25");
//! ```

use core::str::FromStr;

pub use oxinum_core::{OxiNumError, OxiNumResult};
pub use oxinum_float::DBig;

/// Arbitrary-precision complex number `re + im·i`, with each component a
/// decimal [`DBig`].
///
/// See the crate-level documentation for the rationale behind the absence of
/// `Hash`, `Eq`, and `Ord`.
#[derive(Clone)]
pub struct CBig {
    pub(crate) re: DBig,
    pub(crate) im: DBig,
}

impl CBig {
    /// Construct a complex number from its real and imaginary parts.
    ///
    /// Alias of [`CBig::from_parts`].
    pub fn new(re: DBig, im: DBig) -> Self {
        Self::from_parts(re, im)
    }

    /// Construct a complex number from its real and imaginary parts.
    pub fn from_parts(re: DBig, im: DBig) -> Self {
        Self { re, im }
    }

    /// Construct a purely real complex number (`im = 0`).
    pub fn from_real(re: DBig) -> Self {
        Self {
            re,
            im: DBig::from(0u32),
        }
    }

    /// Construct a purely imaginary complex number (`re = 0`).
    pub fn from_imag(im: DBig) -> Self {
        Self {
            re: DBig::from(0u32),
            im,
        }
    }

    /// The additive identity `0 + 0·i`.
    pub fn zero() -> Self {
        Self {
            re: DBig::from(0u32),
            im: DBig::from(0u32),
        }
    }

    /// The multiplicative identity `1 + 0·i`.
    pub fn one() -> Self {
        Self {
            re: DBig::from(1u32),
            im: DBig::from(0u32),
        }
    }

    /// The imaginary unit `0 + 1·i`.
    pub fn i() -> Self {
        Self {
            re: DBig::from(0u32),
            im: DBig::from(1u32),
        }
    }

    /// Construct a complex number from a pair of `f64` values.
    ///
    /// Each component is converted to `DBig` by formatting it with 17
    /// significant digits (enough to round-trip any finite `f64`) and parsing
    /// the resulting string.
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::Parse`] if either input is `NaN` or infinite
    /// (`DBig` models neither) or if the formatted decimal string fails to
    /// parse.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::CBig;
    /// let z = CBig::from_f64(1.5, -2.25).expect("finite parts");
    /// assert_eq!(z.re().to_string(), "1.5");
    /// assert_eq!(z.im().to_string(), "-2.25");
    /// ```
    pub fn from_f64(re: f64, im: f64) -> OxiNumResult<Self> {
        Ok(Self {
            re: f64_to_dbig(re)?,
            im: f64_to_dbig(im)?,
        })
    }

    /// A shared reference to the real part.
    pub fn re(&self) -> &DBig {
        &self.re
    }

    /// A shared reference to the imaginary part.
    pub fn im(&self) -> &DBig {
        &self.im
    }

    /// A clone of the real part.
    pub fn real(&self) -> DBig {
        self.re.clone()
    }

    /// A clone of the imaginary part.
    pub fn imag(&self) -> DBig {
        self.im.clone()
    }

    /// Decompose into the owned `(re, im)` pair.
    pub fn into_parts(self) -> (DBig, DBig) {
        (self.re, self.im)
    }

    /// The complex conjugate `re − im·i`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::CBig;
    /// let z = CBig::from_f64(2.0, 3.0).expect("finite parts");
    /// let c = z.conj();
    /// assert_eq!(c.re().to_string(), "2");
    /// assert_eq!(c.im().to_string(), "-3");
    /// ```
    pub fn conj(&self) -> Self {
        Self {
            re: self.re.clone(),
            im: -self.im.clone(),
        }
    }

    /// The squared magnitude `re² + im²` (always real, non-negative).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::CBig;
    /// let z = CBig::from_f64(3.0, 4.0).expect("finite parts");
    /// assert_eq!(z.norm_sqr().to_string(), "25");
    /// ```
    pub fn norm_sqr(&self) -> DBig {
        (&self.re * &self.re) + (&self.im * &self.im)
    }

    /// Returns `true` if both components are zero.
    pub fn is_zero(&self) -> bool {
        let zero = DBig::from(0u32);
        self.re == zero && self.im == zero
    }

    /// Returns `true` if the imaginary part is zero (the value is real).
    pub fn is_real(&self) -> bool {
        self.im == DBig::from(0u32)
    }

    /// Returns `true` if the real part is zero (the value is purely imaginary).
    pub fn is_imaginary(&self) -> bool {
        self.re == DBig::from(0u32)
    }
}

/// Convert an `f64` to `DBig` via a 17-significant-digit decimal string.
///
/// `dashu-float` does not implement `From<f64>` for `DBig`; the reliable
/// route is to format the value (17 digits uniquely identifies any finite
/// `f64`) and parse it back. Non-finite inputs are rejected because `DBig`
/// has no `NaN` / `Inf` representation.
fn f64_to_dbig(v: f64) -> OxiNumResult<DBig> {
    if v.is_nan() {
        return Err(OxiNumError::Parse("cannot encode NaN as DBig".into()));
    }
    if v.is_infinite() {
        return Err(OxiNumError::Parse("cannot encode infinity as DBig".into()));
    }
    let s = format!("{v:.17e}");
    DBig::from_str(&s).map_err(|e| OxiNumError::Parse(format!("invalid f64→DBig: {e:?}").into()))
}

// ---------------------------------------------------------------------------
// Sub-modules. The CBig type itself lives entirely in this file; the modules
// below add operator impls, transcendental / trig functions, conversions,
// formatting, and optional feature integrations.
// ---------------------------------------------------------------------------

mod convert;
mod fmt;
mod inverse_trig;
mod ops;
mod transcendental;
mod trig;

#[cfg(feature = "num-traits")]
mod num_traits_impl;
#[cfg(feature = "serde")]
mod serde_impl;

/// Native binary-base arbitrary-precision complex implementation built on
/// [`oxinum_float::native::BigFloat`].
///
/// This module is additive: the crate-root [`Complex`] alias remains the
/// decimal-backed [`CBig`]. Reach for [`native::BigComplex`] when you want
/// ground-up Pure Rust binary complex arithmetic with explicit rounding-mode
/// control.
pub mod native;

/// Convenience alias for [`CBig`], mirroring the `BigFloat = DBig` convention
/// used throughout the OxiNum crates.
pub type Complex = CBig;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_one_i_constructors() {
        let z = CBig::zero();
        assert!(z.is_zero());
        assert_eq!(z.re().to_string(), "0");
        assert_eq!(z.im().to_string(), "0");

        let one = CBig::one();
        assert!(one.is_real());
        assert_eq!(one.re().to_string(), "1");
        assert_eq!(one.im().to_string(), "0");

        let imag = CBig::i();
        assert!(imag.is_imaginary());
        assert_eq!(imag.re().to_string(), "0");
        assert_eq!(imag.im().to_string(), "1");
    }

    #[test]
    fn from_parts_round_trip() {
        let z = CBig::from_f64(2.5, -7.0).expect("finite parts");
        let (re, im) = z.clone().into_parts();
        assert_eq!(re.to_string(), "2.5");
        assert_eq!(im.to_string(), "-7");
        assert_eq!(z.real().to_string(), "2.5");
        assert_eq!(z.imag().to_string(), "-7");
    }

    #[test]
    fn conj_negates_imag_and_norm_sqr() {
        let z = CBig::from_f64(3.0, 4.0).expect("finite parts");
        let c = z.conj();
        assert_eq!(c.re().to_string(), "3");
        assert_eq!(c.im().to_string(), "-4");
        // |3 + 4i|^2 = 25.
        assert_eq!(z.norm_sqr().to_string(), "25");
    }

    #[test]
    fn from_f64_rejects_non_finite() {
        assert!(CBig::from_f64(f64::NAN, 0.0).is_err());
        assert!(CBig::from_f64(0.0, f64::INFINITY).is_err());
        assert!(CBig::from_f64(f64::NEG_INFINITY, 1.0).is_err());
    }
}
