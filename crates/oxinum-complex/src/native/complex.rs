//! Native `BigComplex` — a binary-base arbitrary-precision complex number
//! built as the ordered pair `(re, im)` of [`BigFloat`]s.
//!
//! The value of a `BigComplex` is `re + im·i`, where both components are
//! native binary [`BigFloat`]s. Each component carries its own precision
//! and (via [`BigFloat`]) full IEEE-754-style non-finite support; arithmetic
//! and transcendental routines added by sibling modules reconcile precision
//! and rounding as required.
//!
//! # Why `Hash`, `Eq`, and `Ord` are not implemented
//!
//! The complex field has no order compatible with its ring structure, so no
//! `Ord` / `PartialOrd` is provided. `BigFloat` itself implements neither
//! `Eq` (NaN breaks reflexivity) nor `Hash`, so `BigComplex` inherits the
//! same constraints; component-wise `PartialEq` is available where useful.

use oxinum_core::OxiNumResult;
use oxinum_float::native::{BigFloat, RoundingMode};

/// Native arbitrary-precision binary complex number `re + im·i`.
///
/// Both components are [`BigFloat`] values. See the module-level
/// documentation for the rationale behind the absence of `Hash`, `Eq`, and
/// `Ord`.
///
/// # Examples
///
/// ```
/// use oxinum_complex::native::{BigComplex, RoundingMode};
///
/// let z = BigComplex::from_f64(3.0, 4.0, 53).expect("finite parts");
/// // |3 + 4i|^2 = 25.
/// assert_eq!(z.norm_sqr().to_f64(), 25.0);
/// ```
#[derive(Clone)]
pub struct BigComplex {
    pub(crate) re: BigFloat,
    pub(crate) im: BigFloat,
}

impl BigComplex {
    /// Construct a complex number from its real and imaginary parts.
    pub fn from_parts(re: BigFloat, im: BigFloat) -> Self {
        Self { re, im }
    }

    /// Construct a complex number from its real and imaginary parts.
    ///
    /// Alias of [`BigComplex::from_parts`].
    pub fn new(re: BigFloat, im: BigFloat) -> Self {
        Self::from_parts(re, im)
    }

    /// Construct a purely real complex number (`im = 0` at `re`'s precision).
    pub fn from_real(re: BigFloat) -> Self {
        let prec = re.precision();
        Self {
            re,
            im: BigFloat::zero(prec),
        }
    }

    /// Construct a purely imaginary complex number (`re = 0` at `im`'s precision).
    pub fn from_imag(im: BigFloat) -> Self {
        let prec = im.precision();
        Self {
            re: BigFloat::zero(prec),
            im,
        }
    }

    /// The additive identity `0 + 0·i` at `prec` bits of precision.
    pub fn zero(prec: u32) -> Self {
        Self {
            re: BigFloat::zero(prec),
            im: BigFloat::zero(prec),
        }
    }

    /// The multiplicative identity `1 + 0·i` at `prec` bits of precision.
    pub fn one(prec: u32, mode: RoundingMode) -> Self {
        Self {
            re: BigFloat::from_i64(1, prec, mode),
            im: BigFloat::zero(prec),
        }
    }

    /// The imaginary unit `0 + 1·i` at `prec` bits of precision.
    pub fn i(prec: u32, mode: RoundingMode) -> Self {
        Self {
            re: BigFloat::zero(prec),
            im: BigFloat::from_i64(1, prec, mode),
        }
    }

    /// Construct a complex number from a pair of `f64` values at `prec` bits.
    ///
    /// Delegates to [`BigFloat::from_f64`] for each component.
    ///
    /// # Errors
    ///
    /// Propagates the [`OxiNumError`](oxinum_core::OxiNumError) returned by
    /// [`BigFloat::from_f64`] when a component is `NaN` or infinite — the
    /// native `BigFloat` rejects those inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::BigComplex;
    /// let z = BigComplex::from_f64(1.5, -2.25, 53).expect("finite parts");
    /// assert_eq!(z.re().to_f64(), 1.5);
    /// assert_eq!(z.im().to_f64(), -2.25);
    /// ```
    pub fn from_f64(re: f64, im: f64, prec: u32) -> OxiNumResult<Self> {
        Ok(Self {
            re: BigFloat::from_f64(re, prec)?,
            im: BigFloat::from_f64(im, prec)?,
        })
    }

    /// A shared reference to the real part.
    pub fn re(&self) -> &BigFloat {
        &self.re
    }

    /// A shared reference to the imaginary part.
    pub fn im(&self) -> &BigFloat {
        &self.im
    }

    /// A clone of the real part.
    pub fn real(&self) -> BigFloat {
        self.re.clone()
    }

    /// A clone of the imaginary part.
    pub fn imag(&self) -> BigFloat {
        self.im.clone()
    }

    /// Decompose into the owned `(re, im)` pair.
    pub fn into_parts(self) -> (BigFloat, BigFloat) {
        (self.re, self.im)
    }

    /// The complex conjugate `re − im·i`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::BigComplex;
    /// let z = BigComplex::from_f64(2.0, 3.0, 53).expect("finite parts");
    /// let c = z.conj();
    /// assert_eq!(c.re().to_f64(), 2.0);
    /// assert_eq!(c.im().to_f64(), -3.0);
    /// ```
    pub fn conj(&self) -> Self {
        Self {
            re: self.re.clone(),
            im: -&self.im,
        }
    }

    /// The squared magnitude `re² + im²` (always real, non-negative).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::BigComplex;
    /// let z = BigComplex::from_f64(3.0, 4.0, 53).expect("finite parts");
    /// assert_eq!(z.norm_sqr().to_f64(), 25.0);
    /// ```
    pub fn norm_sqr(&self) -> BigFloat {
        &(&self.re * &self.re) + &(&self.im * &self.im)
    }

    /// Returns `true` if both components are the canonical zero.
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_one_i_constructors() {
        let z = BigComplex::zero(53);
        assert!(z.is_zero());

        let one = BigComplex::one(53, RoundingMode::HalfEven);
        assert_eq!(one.re().to_f64(), 1.0);
        assert_eq!(one.im().to_f64(), 0.0);

        let imag = BigComplex::i(53, RoundingMode::HalfEven);
        assert_eq!(imag.re().to_f64(), 0.0);
        assert_eq!(imag.im().to_f64(), 1.0);
    }

    #[test]
    fn from_parts_round_trip() {
        let z = BigComplex::from_f64(2.5, -7.0, 53).expect("finite parts");
        let (re, im) = z.clone().into_parts();
        assert_eq!(re.to_f64(), 2.5);
        assert_eq!(im.to_f64(), -7.0);
        assert_eq!(z.real().to_f64(), 2.5);
        assert_eq!(z.imag().to_f64(), -7.0);
    }

    #[test]
    fn conj_and_norm_sqr() {
        let z = BigComplex::from_f64(3.0, 4.0, 53).expect("finite parts");
        let c = z.conj();
        assert_eq!(c.re().to_f64(), 3.0);
        assert_eq!(c.im().to_f64(), -4.0);
        assert_eq!(z.norm_sqr().to_f64(), 25.0);
    }
}
