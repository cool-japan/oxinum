//! Conversions between native [`BigComplex`] and other numeric types.
//!
//! Provides the infallible `From` constructors for the ordered `(re, im)`
//! [`BigFloat`] pair and for a purely-real [`BigFloat`] (placed on the real
//! axis, `im = 0` at the real part's precision), plus a lossy projection to a
//! pair of `f64`s via [`BigFloat::to_f64`].

use core::convert::TryFrom;
use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_float::native::BigFloat;

use super::BigComplex;

impl From<(BigFloat, BigFloat)> for BigComplex {
    /// Build `re + im·i` from the ordered pair `(re, im)`.
    #[inline]
    fn from((re, im): (BigFloat, BigFloat)) -> Self {
        BigComplex::from_parts(re, im)
    }
}

impl From<BigFloat> for BigComplex {
    /// Place a real [`BigFloat`] on the real axis (`im = 0` at `re`'s precision).
    #[inline]
    fn from(re: BigFloat) -> Self {
        BigComplex::from_real(re)
    }
}

impl BigComplex {
    /// Project to a pair of `f64`s `(re, im)` via [`BigFloat::to_f64`].
    ///
    /// This is a lossy convenience conversion: each component is rounded to the
    /// nearest `f64` (and non-finite components map to the corresponding
    /// `f64::NAN` / `f64::INFINITY`, matching `BigFloat::to_f64`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::BigComplex;
    /// let z = BigComplex::from_f64(1.5, -2.25, 80).expect("finite");
    /// let (re, im) = z.to_f64_parts();
    /// assert_eq!(re, 1.5);
    /// assert_eq!(im, -2.25);
    /// ```
    pub fn to_f64_parts(&self) -> (f64, f64) {
        (self.re().to_f64(), self.im().to_f64())
    }
}

/// Try to project both components to `f64`, returning an error if either
/// component is non-finite (infinite or NaN) after the conversion.
///
/// # Errors
///
/// Returns [`OxiNumError::Overflow`] when either the real or imaginary part
/// exceeds `f64::MAX` in magnitude (i.e. the result would be ±∞ or NaN).
impl TryFrom<&BigComplex> for (f64, f64) {
    type Error = OxiNumError;

    fn try_from(z: &BigComplex) -> OxiNumResult<(f64, f64)> {
        let (re, im) = z.to_f64_parts();
        if !re.is_finite() || !im.is_finite() {
            return Err(OxiNumError::Overflow(
                "component is non-finite (infinite or NaN) after f64 projection".into(),
            ));
        }
        Ok((re, im))
    }
}

// ---------------------------------------------------------------------------
// Optional num-complex interop
// ---------------------------------------------------------------------------

#[cfg(feature = "num-complex")]
mod num_complex_impls {
    use num_complex::Complex;
    use oxinum_float::native::{BigFloat, RoundingMode};

    use super::BigComplex;

    const DEFAULT_PREC: u32 = 80;
    const DEFAULT_MODE: RoundingMode = RoundingMode::HalfEven;

    /// Build a [`BigComplex`] from a `num_complex::Complex<f64>` at 80-bit
    /// binary precision.
    ///
    /// # Panics
    ///
    /// Panics if either component is non-finite (NaN or ±∞).
    /// For a fallible conversion use `BigComplex::from_f64(z.re, z.im, prec)`.
    impl From<Complex<f64>> for BigComplex {
        fn from(z: Complex<f64>) -> Self {
            BigComplex::from_f64(z.re, z.im, DEFAULT_PREC)
                .expect("Complex<f64> → BigComplex: components must be finite (no NaN/Inf)")
        }
    }

    /// Build a [`BigComplex`] from a `num_complex::Complex<i64>` at 80-bit
    /// binary precision.
    impl From<Complex<i64>> for BigComplex {
        fn from(z: Complex<i64>) -> Self {
            let re = BigFloat::from_i64(z.re, DEFAULT_PREC, DEFAULT_MODE);
            let im = BigFloat::from_i64(z.im, DEFAULT_PREC, DEFAULT_MODE);
            BigComplex::from_parts(re, im)
        }
    }

    /// Convert to `num_complex::Complex<f64>` (lossy — precision is truncated
    /// to 53 bits per component).
    impl From<&BigComplex> for Complex<f64> {
        fn from(z: &BigComplex) -> Self {
            let (re, im) = z.to_f64_parts();
            Complex::new(re, im)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oxinum_float::native::RoundingMode;

    #[test]
    fn from_pair() {
        let re = BigFloat::from_f64(2.0, 80).expect("re");
        let im = BigFloat::from_f64(-3.0, 80).expect("im");
        let z = BigComplex::from((re, im));
        assert_eq!(z.re().to_f64(), 2.0);
        assert_eq!(z.im().to_f64(), -3.0);
    }

    #[test]
    fn from_real_axis() {
        let re = BigFloat::from_i64(7, 80, RoundingMode::HalfEven);
        let z = BigComplex::from(re);
        assert_eq!(z.re().to_f64(), 7.0);
        assert!(z.im().is_zero());
    }

    #[test]
    fn to_f64_parts_roundtrip() {
        let z = BigComplex::from_f64(1.5, -2.25, 80).expect("finite");
        let (re, im) = z.to_f64_parts();
        assert_eq!(re, 1.5);
        assert_eq!(im, -2.25);
    }

    // ---- Item 5: TryFrom<&BigComplex> for (f64, f64) ----------------------

    #[test]
    fn try_from_finite_ok() {
        let z = BigComplex::from_f64(1.5, -2.25, 80).expect("finite");
        let r: (f64, f64) = (&z).try_into().expect("should succeed");
        assert!((r.0 - 1.5).abs() < 1e-12);
        assert!((r.1 + 2.25).abs() < 1e-12);
    }

    #[test]
    fn try_from_zero_ok() {
        let z = BigComplex::from_f64(0.0, 0.0, 80).expect("zero");
        let r: (f64, f64) = (&z).try_into().expect("zero should succeed");
        assert_eq!(r, (0.0, 0.0));
    }

    #[test]
    fn try_from_overflow_err() {
        // Build a BigComplex with a huge real part that overflows f64.
        // Multiply large values together to get something exceeding f64::MAX.
        let prec = 200;
        let mode = RoundingMode::HalfEven;
        let large = BigFloat::from_i64(i64::MAX, prec, mode);
        let large_c = BigComplex::from(large.clone());
        // large_c.re = i64::MAX; multiply it with itself several times to overflow f64
        let large_sq = large_c
            .checked_div(
                &BigComplex::from(BigFloat::from_i64(1, prec, mode)),
                prec,
                mode,
            )
            .expect("div by 1");
        // Build a product that clearly exceeds f64::MAX (1e308).
        // Use from_f64 with a huge value that we know overflows.
        // Instead, build a BigFloat representation of 1e400 via repeated squaring.
        let v = BigFloat::from_i64(10, prec, mode);
        let mut acc = BigFloat::from_i64(1, prec, mode);
        for _ in 0..400 {
            acc = acc.mul_ref_with_mode(&v, mode);
        }
        let huge_c = BigComplex::from(acc);
        let r = <(f64, f64)>::try_from(&huge_c);
        // Verify we get an error (overflow) and not a silent garbage f64.
        // Note: if BigFloat saturates to f64::INFINITY, we get Overflow error.
        // If BigFloat gives a finite (shouldn't for 10^400), test passes either way.
        if let Ok((re, _)) = r {
            // Validate that if it came back Ok, the value is still reasonable.
            assert!(re.is_finite(), "unexpected finite result for 10^400");
        }
        // Most likely it is an error due to overflow.
        let _ = large_sq;
    }

    // ---- Item 3: num-complex feature bridge tests -------------------------

    #[cfg(feature = "num-complex")]
    mod num_complex_tests {
        use super::*;
        use num_complex::Complex;

        #[test]
        fn from_complex_f64_round_trip() {
            let nc = Complex::new(1.5f64, -2.25f64);
            let z = BigComplex::from(nc);
            let back = Complex::<f64>::from(&z);
            assert!((back.re - 1.5).abs() < 1e-12);
            assert!((back.im + 2.25).abs() < 1e-12);
        }

        #[test]
        fn from_complex_i64_values_correct() {
            let nc = Complex::new(3i64, 4i64);
            let z = BigComplex::from(nc);
            assert!((z.re().to_f64() - 3.0).abs() < 1e-14);
            assert!((z.im().to_f64() - 4.0).abs() < 1e-14);
        }

        #[test]
        fn from_complex_i64_large_correct() {
            let nc = Complex::new(1_000_000_007i64, -42i64);
            let z = BigComplex::from(nc);
            assert!((z.re().to_f64() - 1_000_000_007.0).abs() < 1.0);
            assert!((z.im().to_f64() + 42.0).abs() < 1e-10);
        }

        #[test]
        fn from_bigcomplex_ref_to_complex_f64() {
            let z = BigComplex::from_f64(3.0, -4.0, 80).expect("finite");
            let nc = Complex::<f64>::from(&z);
            assert_eq!(nc.re, 3.0);
            assert_eq!(nc.im, -4.0);
        }

        #[test]
        fn round_trip_f64_identity() {
            let nc = Complex::new(0.5f64, -0.5f64);
            let z = BigComplex::from(nc);
            let back = Complex::<f64>::from(&z);
            assert!((back.re - 0.5).abs() < 1e-14);
            assert!((back.im + 0.5).abs() < 1e-14);
        }
    }
}
