//! Property-based tests for [`oxinum_complex::CBig`] using `proptest`.
//!
//! Inputs are pairs `a, b ∈ [−5, 5]`; arguments whose magnitude is below
//! `1e-6` are skipped so the `exp(ln z)` round trip and the squared-`abs` law
//! never touch the branch point at the origin.
//!
//! The properties split into two `proptest!` blocks by cost:
//!
//! * **Exact algebraic laws** (conjugation, additive inverse, multiplicative
//!   identity) are cheap — they touch only `+`, `*`, and `==`, so they run at
//!   the workspace-root default case count (`proptest.toml`, `cases = 64`).
//! * **Transcendental laws** (`exp∘ln`, `abs²`) each evaluate decimal `DBig`
//!   `ln`/`exp`/`sqrt` at precision 40 with guard digits, which is heavy; they
//!   run with an explicit, reduced `cases = 16` so the suite stays fast while
//!   still sampling a spread of off-axis inputs. (The much cheaper native
//!   binary mirror of `exp∘ln` lives in `native_properties.rs`.)
//!
//! Properties asserted (precision 40, `f64`-projected tolerance noted inline):
//!
//! 1. `exp(ln z) ≈ z` — the **branch-safe** `exp ∘ ln` direction (not
//!    `ln ∘ exp`): `ln` lands in the principal strip and `exp` is
//!    single-valued, so there is no 2π wrap. With the recent `atan2`/`atan`
//!    fix in `oxinum-float`, this holds off-axis; the tolerance scales with
//!    `|z|` since the round trip's absolute error grows with magnitude.
//! 2. `conj(conj(z)) == z` — exact.
//! 3. `z + (−z) == CBig::zero()` — exact.
//! 4. `z · 1 == z` — exact.
//! 5. `abs(z)² ≈ norm_sqr(z)` — tolerance proportional to `norm_sqr` (both
//!    sides scale as `|z|²`).

use oxinum_complex::CBig;
use proptest::prelude::*;

/// Working precision (significant decimal digits).
const PREC: usize = 40;

/// Skip arguments whose magnitude is below this (keeps clear of the origin /
/// `ln` branch point).
const NEAR_ZERO: f64 = 1e-6;

/// Reduced case count for the expensive decimal-transcendental properties.
const HEAVY_CASES: u32 = 16;

// ---------------------------------------------------------------------------
// Exact algebraic laws — cheap; run at the workspace default case count.
// ---------------------------------------------------------------------------

proptest! {
    /// Property 2: double conjugation is the identity (exact).
    #[test]
    fn conj_conj_is_identity(a in -5.0f64..5.0, b in -5.0f64..5.0) {
        let z = CBig::from_f64(a, b).expect("finite parts");
        prop_assert!(z.conj().conj() == z);
    }

    /// Property 3: `z + (−z) == 0` (exact).
    #[test]
    fn add_negation_is_zero(a in -5.0f64..5.0, b in -5.0f64..5.0) {
        let z = CBig::from_f64(a, b).expect("finite parts");
        let sum = &z + &(-&z);
        prop_assert!(sum == CBig::zero());
        prop_assert!(sum.is_zero());
    }

    /// Property 4: `z · 1 == z` (exact multiplicative identity).
    #[test]
    fn mul_one_is_identity(a in -5.0f64..5.0, b in -5.0f64..5.0) {
        let z = CBig::from_f64(a, b).expect("finite parts");
        prop_assert!(&z * &CBig::one() == z);
    }
}

// ---------------------------------------------------------------------------
// Transcendental laws — heavy (decimal ln/exp/sqrt at precision 40); run with
// a reduced case count.
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: HEAVY_CASES, ..ProptestConfig::default() })]

    /// Property 1: `exp(ln z) ≈ z` (branch-safe exp∘ln direction).
    ///
    /// The absolute tolerance is scaled by `max(1, |z|)`: the round trip's
    /// error is relative, so a base of `1e-12` comfortably covers the range.
    #[test]
    fn exp_ln_round_trip(a in -5.0f64..5.0, b in -5.0f64..5.0) {
        let mag = a.hypot(b);
        prop_assume!(mag > NEAR_ZERO);

        let z = CBig::from_f64(a, b).expect("finite parts");
        let back = z.ln(PREC).expect("ln").exp(PREC).expect("exp");

        let (re, im) = back.to_f64_parts();
        let tol = 1e-12 * mag.max(1.0);
        prop_assert!((re - a).abs() < tol, "re: {re} vs {a} (tol {tol})");
        prop_assert!((im - b).abs() < tol, "im: {im} vs {b} (tol {tol})");
    }

    /// Property 5: `abs(z)² ≈ norm_sqr(z)` on the `f64` projection.
    #[test]
    fn abs_squared_matches_norm_sqr(a in -5.0f64..5.0, b in -5.0f64..5.0) {
        let mag = a.hypot(b);
        prop_assume!(mag > NEAR_ZERO);

        let z = CBig::from_f64(a, b).expect("finite parts");
        let abs = z.abs(PREC).expect("abs").to_f64().value();
        let ns = z.norm_sqr().to_f64().value();

        let lhs = abs * abs;
        // Both sides scale as |z|²; allow a tolerance proportional to ns.
        let tol = 1e-9 * ns.max(1.0);
        prop_assert!((lhs - ns).abs() < tol, "abs² = {lhs}, norm_sqr = {ns} (tol {tol})");
    }
}
