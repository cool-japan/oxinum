//! Property-based tests for [`oxinum_complex::native::BigComplex`] using
//! `proptest`.
//!
//! Mirrors the two branch-safe / exact `CBig` properties on the native
//! binary-base type at `prec = 80` bits, over the same bounded input range
//! `a, b ∈ [−5, 5]`, skipping arguments near the origin:
//!
//! 1. `exp(ln z) ≈ z` — branch-safe `exp ∘ ln` direction (no 2π wrap). The
//!    tolerance scales with `|z|`.
//! 2. `conj(conj(z)) == z` — exact.

use oxinum_complex::native::{BigComplex, RoundingMode};
use proptest::prelude::*;

/// Working precision in bits.
const PREC: u32 = 80;

/// Rounding mode for every native operation here.
const MODE: RoundingMode = RoundingMode::HalfEven;

/// Skip arguments whose magnitude is below this (keeps clear of the `ln` branch
/// point at the origin).
const NEAR_ZERO: f64 = 1e-6;

proptest! {
    /// Property 1: `exp(ln z) ≈ z` (branch-safe exp∘ln direction).
    #[test]
    fn exp_ln_round_trip(a in -5.0f64..5.0, b in -5.0f64..5.0) {
        let mag = a.hypot(b);
        prop_assume!(mag > NEAR_ZERO);

        let z = BigComplex::from_f64(a, b, PREC).expect("finite parts");
        let back = z
            .ln(PREC, MODE)
            .expect("ln")
            .exp(PREC, MODE)
            .expect("exp");

        let (re, im) = back.to_f64_parts();
        let tol = 1e-12 * mag.max(1.0);
        prop_assert!((re - a).abs() < tol, "re: {re} vs {a} (tol {tol})");
        prop_assert!((im - b).abs() < tol, "im: {im} vs {b} (tol {tol})");
    }

    /// Property 2: double conjugation is the identity (exact).
    #[test]
    fn conj_conj_is_identity(a in -5.0f64..5.0, b in -5.0f64..5.0) {
        let z = BigComplex::from_f64(a, b, PREC).expect("finite parts");
        prop_assert!(z.conj().conj() == z);
    }
}
