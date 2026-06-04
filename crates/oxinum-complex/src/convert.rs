//! Conversions between [`CBig`] and ordinary Rust / `oxinum-float` scalars.
//!
//! These `From` impls let callers build a complex number from a single real
//! component (placed on the real axis) or from an explicit `(re, im)` pair,
//! using either [`DBig`] or plain integers. The lossy [`CBig::to_f64_parts`]
//! escape hatch projects both components down to `f64`.
//!
//! # Integer conversions are exact
//!
//! `dashu-float`'s default `DBig::from(n: i64)` carries only the *one*
//! significant decimal digit needed to print `n`, and `DBig` arithmetic
//! rounds each result back to its operands' precision. Building both parts
//! that way would make any later multiplication collapse precision — e.g.
//! `CBig::from((3, 4)).norm_sqr()` would round `9 + 16` to a single digit
//! and yield `30` rather than the exact `25`.
//!
//! To avoid that footgun, the integer `From` impls below rebind each part to
//! `dashu-float`'s **unlimited** precision (precision `0`) via
//! [`oxinum_float::precision::with_precision`]. At unlimited precision every
//! `finite × finite` and `finite ± finite` operation is *exact*, so an
//! integer-constructed `CBig` keeps full precision through subsequent
//! `norm_sqr`, multiplication, and `pow`. The [`DBig`]-based conversions
//! ([`From<(DBig, DBig)>`], [`From<DBig>`], [`From<&DBig>`]) pass their inputs
//! through unchanged and so already carry whatever precision the caller chose.

use crate::CBig;
use core::convert::TryFrom;
use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_float::precision::with_precision;
use oxinum_float::DBig;

/// Build an *exact* [`DBig`] from a signed integer.
///
/// `DBig::from(n)` retains only the single significant digit it needs to
/// render `n`, which causes later `DBig` arithmetic to round back to that
/// precision. Rebinding to precision `0` (`dashu-float`'s "unlimited") makes
/// the value carry no precision cap, so products and sums involving it stay
/// exact across the whole `i64` range (and beyond).
#[inline]
fn exact_dbig(n: i64) -> DBig {
    with_precision(&DBig::from(n), 0)
}

/// Build a complex number from an explicit `(re, im)` pair of [`DBig`] values.
impl From<(DBig, DBig)> for CBig {
    fn from((re, im): (DBig, DBig)) -> Self {
        CBig::from_parts(re, im)
    }
}

/// Embed a real [`DBig`] on the real axis (`im = 0`).
impl From<DBig> for CBig {
    fn from(re: DBig) -> Self {
        CBig::from_real(re)
    }
}

/// Embed a borrowed real [`DBig`] on the real axis (`im = 0`).
impl From<&DBig> for CBig {
    fn from(re: &DBig) -> Self {
        CBig::from_real(re.clone())
    }
}

/// Build a complex number from an integer `(re, im)` pair (convenience).
///
/// Both parts are represented **exactly** (at unlimited `DBig` precision), so
/// the result keeps full precision through later arithmetic — e.g.
/// `CBig::from((3, 4)).norm_sqr()` is the exact `25`. See the module-level
/// "Integer conversions are exact" note for the rationale.
impl From<(i64, i64)> for CBig {
    fn from((re, im): (i64, i64)) -> Self {
        CBig::from_parts(exact_dbig(re), exact_dbig(im))
    }
}

/// Embed an integer on the real axis (`im = 0`).
///
/// The real part is represented **exactly** (at unlimited `DBig` precision),
/// so the value keeps full precision through later arithmetic. See the
/// module-level "Integer conversions are exact" note for the rationale.
impl From<i64> for CBig {
    fn from(re: i64) -> Self {
        CBig::from_real(exact_dbig(re))
    }
}

impl CBig {
    /// Project both components down to `f64`, returning `(re, im)`.
    ///
    /// # Precision
    ///
    /// This conversion is **lossy**: each arbitrary-precision [`DBig`]
    /// component is rounded to the nearest `f64`. Values whose magnitude
    /// exceeds [`f64::MAX`] saturate to `±∞`, and digits beyond the 53-bit
    /// mantissa are discarded. Use it only when an ordinary floating-point
    /// approximation is acceptable.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::CBig;
    /// let z = CBig::from_f64(3.5, -1.25).expect("finite parts");
    /// assert_eq!(z.to_f64_parts(), (3.5, -1.25));
    /// ```
    pub fn to_f64_parts(&self) -> (f64, f64) {
        (self.re.to_f64().value(), self.im.to_f64().value())
    }
}

/// Try to project both components to `f64`, returning an error if either
/// component is non-finite (infinite or NaN) after the conversion.
///
/// # Errors
///
/// Returns [`OxiNumError::Overflow`] when either the real or imaginary part
/// exceeds [`f64::MAX`] in magnitude (i.e. the result would be ±∞ or NaN).
impl TryFrom<&CBig> for (f64, f64) {
    type Error = OxiNumError;

    fn try_from(z: &CBig) -> OxiNumResult<(f64, f64)> {
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
    use super::exact_dbig;
    use crate::CBig;
    use num_complex::Complex;

    /// Build a [`CBig`] from a `num_complex::Complex<f64>`.
    ///
    /// # Panics
    ///
    /// Panics if either component is non-finite (NaN or ±∞).
    /// For a fallible conversion, use `CBig::from_f64(z.re, z.im)` directly.
    impl From<Complex<f64>> for CBig {
        fn from(z: Complex<f64>) -> Self {
            CBig::from_f64(z.re, z.im)
                .expect("Complex<f64> → CBig: components must be finite (no NaN/Inf)")
        }
    }

    /// Build a [`CBig`] from a `num_complex::Complex<i64>` — both parts are exact.
    ///
    /// Each integer component is stored at unlimited `DBig` precision, so all
    /// subsequent arithmetic (e.g. `norm_sqr`, `pow`) keeps full precision.
    impl From<Complex<i64>> for CBig {
        fn from(z: Complex<i64>) -> Self {
            CBig::from_parts(exact_dbig(z.re), exact_dbig(z.im))
        }
    }

    /// Convert to `num_complex::Complex<f64>` (lossy — precision is truncated
    /// to 53 bits per component).
    impl From<&CBig> for Complex<f64> {
        fn from(z: &CBig) -> Self {
            let (re, im) = z.to_f64_parts();
            Complex::new(re, im)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_dbig_pair() {
        let re = DBig::from(7);
        let im = DBig::from(-4);
        let z: CBig = (re, im).into();
        assert_eq!(z.re().to_string(), "7");
        assert_eq!(z.im().to_string(), "-4");
    }

    #[test]
    fn from_dbig_lands_on_real_axis() {
        let d = DBig::from(5);
        let z: CBig = d.into();
        assert_eq!(z.re().to_string(), "5");
        assert_eq!(z.im().to_string(), "0");
        assert!(z.is_real());
    }

    #[test]
    fn from_dbig_ref_lands_on_real_axis() {
        let d = DBig::from(9);
        let z: CBig = (&d).into();
        assert_eq!(z.re().to_string(), "9");
        assert_eq!(z.im().to_string(), "0");
        // Source `DBig` is untouched (borrowed, not moved).
        assert_eq!(d.to_string(), "9");
    }

    #[test]
    fn from_integer_pair() {
        let z: CBig = (1i64, 2i64).into();
        assert_eq!(z.re().to_string(), "1");
        assert_eq!(z.im().to_string(), "2");
    }

    #[test]
    fn from_integer_lands_on_real_axis() {
        let z: CBig = 42i64.into();
        assert_eq!(z.re().to_string(), "42");
        assert_eq!(z.im().to_string(), "0");
        assert!(z.is_real());
    }

    #[test]
    fn to_f64_parts_round_trips() {
        let z = CBig::from_f64(3.5, -1.25).expect("finite parts");
        assert_eq!(z.to_f64_parts(), (3.5, -1.25));
    }

    // ---- Regression: integer conversions must be EXACT --------------------
    //
    // Before the fix, `DBig::from(n)` kept only one significant digit and
    // `DBig` arithmetic rounded back to that precision, so integer-built
    // `CBig` values silently collapsed precision under multiplication
    // (`from((3, 4)).norm_sqr()` returned ~30 instead of 25).

    #[test]
    fn integer_parts_carry_unlimited_precision() {
        // Precision 0 is `dashu-float`'s "unlimited" — the marker that makes
        // subsequent products/sums exact.
        let z: CBig = (3i64, 4i64).into();
        assert_eq!(
            z.re().precision(),
            0,
            "real part must be unlimited-precision"
        );
        assert_eq!(
            z.im().precision(),
            0,
            "imag part must be unlimited-precision"
        );

        let r: CBig = 7i64.into();
        assert_eq!(
            r.re().precision(),
            0,
            "real-axis part must be unlimited-precision"
        );
        assert_eq!(r.im().to_string(), "0");
    }

    #[test]
    fn integer_norm_sqr_is_exact() {
        // |3 + 4i|² = 9 + 16 = 25, exactly (the headline footgun).
        let z: CBig = (3i64, 4i64).into();
        assert_eq!(z.norm_sqr().to_string(), "25");
    }

    #[test]
    fn integer_product_is_exact() {
        // (1 + 2i)(3 + 4i) = (3 − 8) + (4 + 6)i = -5 + 10i, exactly.
        let prod = CBig::from((1i64, 2i64)) * CBig::from((3i64, 4i64));
        assert_eq!(prod.re().to_string(), "-5");
        assert_eq!(prod.im().to_string(), "10");
    }

    #[test]
    fn integer_large_magnitude_norm_sqr_is_exact() {
        // 1_000_000_007² = 1_000_000_014_000_000_049 — far more than a single
        // significant digit, so this fails loudly if precision collapses.
        let z: CBig = (1_000_000_007i64, 0i64).into();
        assert_eq!(z.norm_sqr().to_string(), "1000000014000000049");
    }

    #[test]
    fn integer_i64_max_norm_sqr_is_exact() {
        // i64::MAX = 9_223_372_036_854_775_807; its square is 39 digits and
        // must be represented exactly under unlimited precision.
        let z: CBig = (i64::MAX, 0i64).into();
        assert_eq!(
            z.norm_sqr().to_string(),
            "85070591730234615847396907784232501249"
        );
    }

    // ---- Item 5: TryFrom<&CBig> for (f64, f64) ----------------------------

    #[test]
    fn try_from_finite_ok() {
        let z = CBig::from_f64(1.5, -2.25).expect("finite");
        let r: (f64, f64) = (&z).try_into().expect("should succeed");
        assert!((r.0 - 1.5).abs() < 1e-12);
        assert!((r.1 + 2.25).abs() < 1e-12);
    }

    #[test]
    fn try_from_zero_ok() {
        let z = CBig::zero();
        let r: (f64, f64) = (&z).try_into().expect("zero should succeed");
        assert_eq!(r, (0.0, 0.0));
    }

    #[test]
    fn try_from_overflow_err() {
        // Build a CBig with a huge real part that overflows f64 using a
        // bounded-precision DBig so that to_f64() does not panic.
        // f64::MAX ≈ 1.8e308; 1e400 is safely beyond that.
        const PREC: usize = 64; // enough decimal digits to represent 1e400
        let base = with_precision(&DBig::from(10i64), PREC);
        let mut acc = with_precision(&DBig::from(1i64), PREC);
        for _ in 0..400 {
            acc = with_precision(&(&acc * &base), PREC);
        }
        let huge = CBig::from_parts(acc, DBig::from(0i64));
        let r = <(f64, f64)>::try_from(&huge);
        assert!(r.is_err(), "should fail on overflow: {r:?}");
    }

    // ---- Item 3: num-complex feature bridge tests -------------------------

    #[cfg(feature = "num-complex")]
    mod num_complex_tests {
        use super::*;
        use num_complex::Complex;

        #[test]
        fn from_complex_f64_round_trip() {
            let nc = Complex::new(1.5f64, -2.25f64);
            let z = CBig::from(nc);
            let back = Complex::<f64>::from(&z);
            assert!((back.re - 1.5).abs() < 1e-12);
            assert!((back.im + 2.25).abs() < 1e-12);
        }

        #[test]
        fn from_complex_i64_is_exact() {
            let nc = Complex::new(3i64, 4i64);
            let z = CBig::from(nc);
            assert_eq!(z.norm_sqr().to_string(), "25");
        }

        #[test]
        fn from_complex_i64_large_is_exact() {
            // (i64::MAX + 0i) — should preserve full precision.
            let nc = Complex::new(i64::MAX, 1i64);
            let z = CBig::from(nc);
            assert_eq!(z.re().to_string(), i64::MAX.to_string());
            assert_eq!(z.im().to_string(), "1");
        }

        #[test]
        fn from_cbig_ref_to_complex_f64() {
            let z = CBig::from_f64(3.0, -4.0).expect("finite");
            let nc = Complex::<f64>::from(&z);
            assert_eq!(nc.re, 3.0);
            assert_eq!(nc.im, -4.0);
        }

        #[test]
        fn from_complex_i64_norm_sqr_exact() {
            // |5 + 12i|² = 25 + 144 = 169 = 13²
            let nc = Complex::new(5i64, 12i64);
            let z = CBig::from(nc);
            assert_eq!(z.norm_sqr().to_string(), "169");
        }
    }
}
