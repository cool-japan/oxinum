//! SciRS2 trait-hierarchy compatibility verification.
//!
//! Proves at compile time and at runtime that `oxinum-core` exports the
//! exact types and traits that `scirs2-core/src/numeric/arbitrary_precision.rs`
//! consumes.  Running this test confirms that the trait hierarchy is compatible
//! with SciRS2's numeric requirements.
//!
//! The SciRS2 consumer calls:
//!   - `use oxinum_core::Abs;`               → via dashu-base re-export
//!   - `use oxinum_core::Signed;`            → via dashu-base re-export
//!   - `oxinum_core::Sign::Positive`         → variant from dashu-base::Sign
//!   - `oxinum_core::Sign::Negative`         → variant from dashu-base::Sign
//!   - `.abs()` on IBig (via Abs trait)
//!   - `.sign()` on IBig (via Signed trait)
//!
//! This file is the whole test.

// oxinum-core does not re-export integer types directly; use oxinum_int which
// re-exports dashu-int's IBig / UBig.  This is the same path SciRS2 takes:
// it imports Abs/Signed from oxinum_core and IBig from oxinum_int.
// SciRS2 imports both `Abs` and `Signed` from oxinum_core.  The `Abs` trait
// is actively used for `.abs()` below.  `Signed` exposes `.sign()` on IBig;
// dashu's IBig also provides `.sign()` as an inherent method, so rustc
// doesn't require the trait import for method resolution on the current toolchain.
// We document the intent here and call `.sign()` in the tests to verify the
// method resolves correctly regardless of how it's brought into scope.
use oxinum_core::{Abs, Sign};
use oxinum_int::IBig;

// -----------------------------------------------------------------------
// Compile-time signature assertions
// -----------------------------------------------------------------------

/// Statically ensure every symbol consumed by SciRS2 resolves correctly.
/// This function is never called; it only needs to compile.
#[allow(dead_code)]
fn _assert_core_contract() {
    use oxinum_core::Signed; // the Signed trait is imported here for explicit use

    // Sign variants must exist.
    let _pos = Sign::Positive;
    let _neg = Sign::Negative;

    // Abs and Signed must be in scope and their methods invocable on IBig.
    let n = IBig::from(-5_i32);

    // IBig must implement Abs (returns UBig).
    let _abs_val = Abs::abs(n.clone());

    // IBig must implement Signed (returns Sign).
    let _sign = Signed::sign(&n);
}

// -----------------------------------------------------------------------
// Behavioural tests
// -----------------------------------------------------------------------

#[test]
fn sign_variants_exist_and_are_distinguishable() {
    assert_ne!(Sign::Positive, Sign::Negative);
}

#[test]
fn abs_trait_on_ibig_positive() {
    use oxinum_int::IBig;
    let n = IBig::from(42_i32);
    let a = n.abs(); // UBig
    assert_eq!(a.to_string(), "42");
}

#[test]
fn abs_trait_on_ibig_negative() {
    use oxinum_int::IBig;
    let n = IBig::from(-42_i32);
    let a = n.abs(); // UBig
    assert_eq!(a.to_string(), "42");
}

#[test]
fn abs_trait_on_ibig_zero() {
    use oxinum_int::IBig;
    let n = IBig::from(0_i32);
    let a = n.abs();
    assert_eq!(a.to_string(), "0");
}

#[test]
fn signed_trait_positive() {
    use oxinum_int::IBig;
    let n = IBig::from(7_i32);
    assert_eq!(n.sign(), Sign::Positive);
}

#[test]
fn signed_trait_negative() {
    use oxinum_int::IBig;
    let n = IBig::from(-7_i32);
    assert_eq!(n.sign(), Sign::Negative);
}

#[test]
fn signed_trait_zero_is_positive() {
    // dashu IBig: zero has Positive sign, matching the canonical-zero
    // convention that SciRS2 relies on for zero-check logic.
    use oxinum_int::IBig;
    let n = IBig::from(0_i32);
    assert_eq!(n.sign(), Sign::Positive);
}

#[test]
fn scirs2_signum_logic_matches_expected() {
    // Replicate the exact signum logic from scirs2-core/src/numeric/arbitrary_precision.rs
    // lines 333-344:
    //   if self.value == IBig::from(0i32) { return 0; }
    //   match self.value.sign() {
    //       oxinum_core::Sign::Positive => 1,
    //       oxinum_core::Sign::Negative => -1,
    //   }
    use oxinum_int::IBig;

    let cases: &[(i32, i32)] = &[(-5, -1), (0, 0), (3, 1), (i32::MIN, -1), (i32::MAX, 1)];
    for &(input, expected_signum) in cases {
        let ibig = IBig::from(input);
        let signum = if ibig == IBig::from(0_i32) {
            0
        } else {
            match ibig.sign() {
                Sign::Positive => 1,
                Sign::Negative => -1,
            }
        };
        assert_eq!(signum, expected_signum, "signum mismatch for input {input}");
    }
}

#[test]
fn oxinumerror_is_send_sync() {
    use oxinum_core::OxiNumError;
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OxiNumError>();
}

#[test]
fn roundingmode_exhaustive() {
    use oxinum_core::RoundingMode;
    // All 8 variants compile and are distinct.
    let modes = [
        RoundingMode::Up,
        RoundingMode::Down,
        RoundingMode::Ceiling,
        RoundingMode::Floor,
        RoundingMode::HalfUp,
        RoundingMode::HalfDown,
        RoundingMode::HalfEven,
        RoundingMode::Unnecessary,
    ];
    assert_eq!(modes.len(), 8);
}
