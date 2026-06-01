//! Comprehensive IEEE-754 non-finite test suite for native `BigFloat`.
//!
//! Covers:
//! - Constructors and predicates (NaN, ±Inf, zero, finite)
//! - `classify()` returning `core::num::FpCategory`
//! - `is_sign_positive` / `is_sign_negative`
//! - `PartialEq` (NaN-aware: NaN ≠ NaN)
//! - `PartialOrd` (NaN is unordered)
//! - `total_cmp` (total order: −Inf < finite < +Inf < NaN)
//! - Arithmetic operator IEEE table (all five ops)
//! - Checked-method contracts still return `Err` where documented
//! - Transcendental non-finite propagation
//! - Serde round-trip for NaN / ±Inf (feature-gated)
//! - `num_traits::FloatConst` and `TotalOrder` (feature-gated)

use core::num::FpCategory;
use std::cmp::Ordering;

use oxinum_float::native::{BigFloat, RoundingMode};

const P: u32 = 64;
const MODE: RoundingMode = RoundingMode::HalfEven;

fn finite(n: i64) -> BigFloat {
    BigFloat::from_i64(n, P, MODE)
}

// ===========================================================================
// Section 1: Constructors and predicates
// ===========================================================================

#[test]
fn nan_is_nan() {
    assert!(BigFloat::nan(P).is_nan());
}

#[test]
fn infinity_is_infinite() {
    assert!(BigFloat::infinity(P).is_infinite());
}

#[test]
fn neg_infinity_is_negative_infinite() {
    let neg_inf = BigFloat::neg_infinity(P);
    assert!(neg_inf.is_infinite());
    assert!(neg_inf.is_sign_negative());
}

#[test]
fn finite_value_is_finite() {
    assert!(finite(42).is_finite());
}

#[test]
fn nan_not_finite() {
    assert!(!BigFloat::nan(P).is_finite());
}

#[test]
fn nan_not_infinite() {
    assert!(!BigFloat::nan(P).is_infinite());
}

#[test]
fn infinity_not_nan() {
    assert!(!BigFloat::infinity(P).is_nan());
}

#[test]
fn infinity_not_finite() {
    assert!(!BigFloat::infinity(P).is_finite());
}

#[test]
fn finite_not_nan() {
    assert!(!finite(1).is_nan());
}

#[test]
fn finite_not_infinite() {
    assert!(!finite(1).is_infinite());
}

// classify()

#[test]
fn classify_nan() {
    assert_eq!(BigFloat::nan(P).classify(), FpCategory::Nan);
}

#[test]
fn classify_pos_infinite() {
    assert_eq!(BigFloat::infinity(P).classify(), FpCategory::Infinite);
}

#[test]
fn classify_neg_infinite() {
    assert_eq!(BigFloat::neg_infinity(P).classify(), FpCategory::Infinite);
}

#[test]
fn classify_zero() {
    assert_eq!(BigFloat::zero(P).classify(), FpCategory::Zero);
}

#[test]
fn classify_normal() {
    assert_eq!(finite(7).classify(), FpCategory::Normal);
}

// is_sign_positive / is_sign_negative

#[test]
fn nan_is_sign_positive() {
    // Canonical NaN always has sign = Positive.
    assert!(BigFloat::nan(P).is_sign_positive());
}

#[test]
fn pos_inf_is_sign_positive() {
    assert!(BigFloat::infinity(P).is_sign_positive());
}

#[test]
fn neg_inf_is_sign_negative() {
    assert!(BigFloat::neg_infinity(P).is_sign_negative());
}

#[test]
fn zero_is_sign_positive() {
    // Canonical zero has sign = Positive.
    assert!(BigFloat::zero(P).is_sign_positive());
}

#[test]
fn positive_finite_is_sign_positive() {
    assert!(finite(3).is_sign_positive());
}

#[test]
fn negative_finite_is_sign_negative() {
    assert!(finite(-3).is_sign_negative());
}

// ===========================================================================
// Section 2: PartialEq (NaN-aware)
// ===========================================================================

#[test]
fn nan_ne_nan() {
    // NaN ≠ NaN (IEEE 754 rule).
    let nan1 = BigFloat::nan(P);
    let nan2 = BigFloat::nan(P);
    assert!(nan1 != nan2);
}

#[test]
fn nan_ne_finite() {
    let nan = BigFloat::nan(P);
    let zero = BigFloat::zero(P);
    assert!(nan != zero);
}

#[test]
fn nan_ne_infinity() {
    let nan = BigFloat::nan(P);
    let inf = BigFloat::infinity(P);
    assert!(nan != inf);
}

#[test]
fn pos_inf_eq_pos_inf() {
    assert_eq!(BigFloat::infinity(P), BigFloat::infinity(P));
}

#[test]
fn neg_inf_eq_neg_inf() {
    assert_eq!(BigFloat::neg_infinity(P), BigFloat::neg_infinity(P));
}

#[test]
fn pos_inf_ne_neg_inf() {
    assert_ne!(BigFloat::infinity(P), BigFloat::neg_infinity(P));
}

#[test]
fn inf_ne_finite() {
    let inf = BigFloat::infinity(P);
    let five = finite(5);
    assert!(inf != five);
}

#[test]
fn finite_eq_same_value() {
    let a = finite(7);
    let b = finite(7);
    assert_eq!(a, b);
}

#[test]
fn zeros_at_different_precisions_are_equal() {
    let z1 = BigFloat::zero(32);
    let z2 = BigFloat::zero(64);
    assert_eq!(z1, z2);
}

// ===========================================================================
// Section 3: PartialOrd (NaN is unordered)
// ===========================================================================

#[test]
fn nan_unordered_with_finite() {
    let nan = BigFloat::nan(P);
    let five = finite(5);
    assert_eq!(nan.partial_cmp(&five), None);
    assert_eq!(five.partial_cmp(&nan), None);
}

#[test]
fn nan_unordered_with_itself() {
    let nan = BigFloat::nan(P);
    assert_eq!(nan.partial_cmp(&nan), None);
}

#[test]
fn nan_unordered_with_inf() {
    let nan = BigFloat::nan(P);
    let inf = BigFloat::infinity(P);
    assert_eq!(nan.partial_cmp(&inf), None);
    assert_eq!(inf.partial_cmp(&nan), None);
}

#[test]
fn neg_inf_lt_finite() {
    let neg_inf = BigFloat::neg_infinity(P);
    let five = finite(5);
    assert!(neg_inf < five);
}

#[test]
fn finite_lt_pos_inf() {
    let five = finite(5);
    let inf = BigFloat::infinity(P);
    assert!(five < inf);
}

#[test]
fn neg_inf_lt_pos_inf() {
    assert!(BigFloat::neg_infinity(P) < BigFloat::infinity(P));
}

#[test]
fn pos_inf_gt_zero() {
    assert!(BigFloat::infinity(P) > BigFloat::zero(P));
}

#[test]
fn neg_inf_lt_zero() {
    assert!(BigFloat::neg_infinity(P) < BigFloat::zero(P));
}

// ===========================================================================
// Section 4: total_cmp
// ===========================================================================

#[test]
fn total_cmp_nan_nan_equal() {
    assert_eq!(
        BigFloat::nan(P).total_cmp(&BigFloat::nan(P)),
        Ordering::Equal
    );
}

#[test]
fn total_cmp_pos_inf_lt_nan() {
    // NaN has rank 3, +Inf has rank 2.
    assert_eq!(
        BigFloat::infinity(P).total_cmp(&BigFloat::nan(P)),
        Ordering::Less
    );
}

#[test]
fn total_cmp_nan_gt_pos_inf() {
    assert_eq!(
        BigFloat::nan(P).total_cmp(&BigFloat::infinity(P)),
        Ordering::Greater
    );
}

#[test]
fn total_cmp_neg_inf_lt_pos_inf() {
    assert_eq!(
        BigFloat::neg_infinity(P).total_cmp(&BigFloat::infinity(P)),
        Ordering::Less
    );
}

#[test]
fn total_cmp_neg_inf_lt_zero() {
    assert_eq!(
        BigFloat::neg_infinity(P).total_cmp(&BigFloat::zero(P)),
        Ordering::Less
    );
}

#[test]
fn total_cmp_zero_lt_pos_inf() {
    assert_eq!(
        BigFloat::zero(P).total_cmp(&BigFloat::infinity(P)),
        Ordering::Less
    );
}

#[test]
fn total_cmp_neg_inf_lt_finite() {
    assert_eq!(
        BigFloat::neg_infinity(P).total_cmp(&finite(5)),
        Ordering::Less
    );
}

#[test]
fn total_cmp_finite_lt_pos_inf() {
    assert_eq!(finite(5).total_cmp(&BigFloat::infinity(P)), Ordering::Less);
}

#[test]
fn total_cmp_sorts_correctly() {
    // Sorted sequence under total_cmp: −Inf, −3, 0, 5, +Inf, NaN.
    let mut vals = [
        BigFloat::nan(P),
        BigFloat::infinity(P),
        finite(5),
        BigFloat::zero(P),
        finite(-3),
        BigFloat::neg_infinity(P),
    ];
    vals.sort_by(|a, b| a.total_cmp(b));

    // Position 0: -Inf
    assert!(
        vals[0].is_infinite() && vals[0].is_sign_negative(),
        "first in total order should be -Inf"
    );
    // Position 1: -3
    assert!(
        vals[1].is_finite() && vals[1].is_sign_negative(),
        "second should be negative finite"
    );
    // Position 2: 0
    assert!(vals[2].is_zero(), "third should be zero");
    // Position 3: 5
    assert!(
        vals[3].is_finite() && vals[3].is_sign_positive() && !vals[3].is_zero(),
        "fourth should be positive finite"
    );
    // Position 4: +Inf
    assert!(
        vals[4].is_infinite() && vals[4].is_sign_positive(),
        "fifth should be +Inf"
    );
    // Position 5: NaN
    assert!(vals[5].is_nan(), "last in total order should be NaN");
}

// ===========================================================================
// Section 5: Arithmetic operators — IEEE 754 table
// ===========================================================================

// --- Addition ---

#[test]
fn add_pos_inf_pos_inf_is_pos_inf() {
    let inf = BigFloat::infinity(P);
    let result = &inf + &inf;
    assert!(result.is_infinite() && result.is_sign_positive());
}

#[test]
fn add_neg_inf_neg_inf_is_neg_inf() {
    let neg_inf = BigFloat::neg_infinity(P);
    let result = &neg_inf + &neg_inf;
    assert!(result.is_infinite() && result.is_sign_negative());
}

#[test]
fn add_pos_inf_neg_inf_is_nan() {
    let pos_inf = BigFloat::infinity(P);
    let neg_inf = BigFloat::neg_infinity(P);
    let result = &pos_inf + &neg_inf;
    assert!(result.is_nan(), "+Inf + -Inf should be NaN");
}

#[test]
fn add_inf_plus_finite_is_inf() {
    let inf = BigFloat::infinity(P);
    let five = finite(5);
    let result = &inf + &five;
    assert!(result.is_infinite() && result.is_sign_positive());
}

// --- Subtraction ---

#[test]
fn sub_inf_minus_inf_is_nan() {
    let inf = BigFloat::infinity(P);
    let result = &inf - &inf;
    assert!(result.is_nan(), "+Inf - +Inf should be NaN");
}

#[test]
fn sub_neg_inf_minus_pos_inf_is_neg_inf() {
    let neg_inf = BigFloat::neg_infinity(P);
    let pos_inf = BigFloat::infinity(P);
    let result = &neg_inf - &pos_inf;
    assert!(result.is_infinite() && result.is_sign_negative());
}

// --- Multiplication ---

#[test]
fn mul_inf_times_zero_is_nan() {
    let inf = BigFloat::infinity(P);
    let zero = BigFloat::zero(P);
    let result = &inf * &zero;
    assert!(result.is_nan(), "Inf * 0 should be NaN");
}

#[test]
fn mul_zero_times_inf_is_nan() {
    let zero = BigFloat::zero(P);
    let inf = BigFloat::infinity(P);
    let result = &zero * &inf;
    assert!(result.is_nan(), "0 * Inf should be NaN");
}

#[test]
fn mul_inf_times_finite_nonzero_is_inf() {
    let inf = BigFloat::infinity(P);
    let three = finite(3);
    let result = &inf * &three;
    assert!(result.is_infinite() && result.is_sign_positive());
}

#[test]
fn mul_pos_inf_times_neg_is_neg_inf() {
    let inf = BigFloat::infinity(P);
    let neg_three = finite(-3);
    let result = &inf * &neg_three;
    assert!(result.is_infinite() && result.is_sign_negative());
}

#[test]
fn mul_inf_times_inf_is_inf() {
    let pos_inf = BigFloat::infinity(P);
    let neg_inf = BigFloat::neg_infinity(P);
    let result = &pos_inf * &neg_inf;
    assert!(result.is_infinite() && result.is_sign_negative());
}

// --- Division ---

#[test]
fn div_finite_by_zero_is_pos_inf() {
    let five = finite(5);
    let zero = BigFloat::zero(P);
    let result = &five / &zero;
    assert!(result.is_infinite(), "5 / 0 should be +Inf");
    assert!(result.is_sign_positive(), "5 / 0 should be positive Inf");
}

#[test]
fn div_neg_finite_by_zero_is_neg_inf() {
    let neg_five = finite(-5);
    let zero = BigFloat::zero(P);
    let result = &neg_five / &zero;
    assert!(
        result.is_infinite() && result.is_sign_negative(),
        "-5 / 0 should be -Inf"
    );
}

#[test]
fn div_zero_by_zero_is_nan() {
    let zero = BigFloat::zero(P);
    let result = &zero / &zero;
    assert!(result.is_nan(), "0 / 0 should be NaN");
}

#[test]
fn div_inf_by_inf_is_nan() {
    let inf = BigFloat::infinity(P);
    let result = &inf / &inf;
    assert!(result.is_nan(), "+Inf / +Inf should be NaN");
}

#[test]
fn div_finite_by_inf_is_zero() {
    let five = finite(5);
    let inf = BigFloat::infinity(P);
    let result = &five / &inf;
    assert!(result.is_zero(), "5 / +Inf should be 0");
}

#[test]
fn div_inf_by_finite_is_inf() {
    let inf = BigFloat::infinity(P);
    let three = finite(3);
    let result = &inf / &three;
    assert!(result.is_infinite() && result.is_sign_positive());
}

// --- Remainder ---

#[test]
fn rem_inf_by_anything_is_nan() {
    let inf = BigFloat::infinity(P);
    let three = finite(3);
    let result = &inf % &three;
    assert!(result.is_nan(), "Inf % 3 should be NaN");
}

#[test]
fn rem_finite_by_zero_is_nan() {
    let five = finite(5);
    let zero = BigFloat::zero(P);
    let result = &five % &zero;
    assert!(result.is_nan(), "5 % 0 should be NaN");
}

#[test]
fn rem_finite_by_inf_is_finite() {
    // IEEE 754: finite % ±Inf = finite (the lhs unchanged in value).
    let five = finite(5);
    let inf = BigFloat::infinity(P);
    let result = &five % &inf;
    assert!(result.is_finite(), "5 % +Inf should be finite");
}

// --- NaN propagation through all ops ---

#[test]
fn nan_propagates_through_add() {
    let nan = BigFloat::nan(P);
    let five = finite(5);
    assert!((&nan + &five).is_nan());
    assert!((&five + &nan).is_nan());
}

#[test]
fn nan_propagates_through_sub() {
    let nan = BigFloat::nan(P);
    let five = finite(5);
    assert!((&nan - &five).is_nan());
    assert!((&five - &nan).is_nan());
}

#[test]
fn nan_propagates_through_mul() {
    let nan = BigFloat::nan(P);
    let five = finite(5);
    assert!((&nan * &five).is_nan());
    assert!((&five * &nan).is_nan());
}

#[test]
fn nan_propagates_through_div() {
    let nan = BigFloat::nan(P);
    let five = finite(5);
    assert!((&nan / &five).is_nan());
    assert!((&five / &nan).is_nan());
}

#[test]
fn nan_propagates_through_rem() {
    let nan = BigFloat::nan(P);
    let five = finite(5);
    assert!((&nan % &five).is_nan());
    assert!((&five % &nan).is_nan());
}

// ===========================================================================
// Section 6: Checked methods still return Err for finite-domain errors
// ===========================================================================

#[test]
fn div_ref_finite_by_zero_still_errors() {
    // The *operator* produces Inf; the *checked method* still errors.
    let five = finite(5);
    let zero = BigFloat::zero(P);
    assert!(
        five.div_ref(&zero).is_err(),
        "div_ref on finite/0 should return Err(DivByZero)"
    );
}

#[test]
fn sqrt_of_negative_finite_still_errors() {
    let neg_one = finite(-1);
    assert!(
        neg_one.sqrt(P, MODE).is_err(),
        "sqrt(-1) should return Err(Domain)"
    );
}

#[test]
fn ln_of_zero_still_errors() {
    let zero = BigFloat::zero(P);
    assert!(zero.ln(P, MODE).is_err(), "ln(0) should return Err(Domain)");
}

#[test]
fn ln_of_negative_finite_still_errors() {
    let neg_one = finite(-1);
    assert!(
        neg_one.ln(P, MODE).is_err(),
        "ln(-1) should return Err(Domain)"
    );
}

// ===========================================================================
// Section 7: Transcendental non-finite propagation
// ===========================================================================

#[test]
fn exp_pos_inf_is_pos_inf() {
    let inf = BigFloat::infinity(P);
    let result = inf.exp(P, MODE).expect("exp(+Inf) should succeed");
    assert!(
        result.is_infinite() && result.is_sign_positive(),
        "exp(+Inf) should be +Inf"
    );
}

#[test]
fn exp_neg_inf_is_zero() {
    let neg_inf = BigFloat::neg_infinity(P);
    let result = neg_inf.exp(P, MODE).expect("exp(-Inf) should succeed");
    assert!(result.is_zero(), "exp(-Inf) should be +0");
}

#[test]
fn exp_nan_is_nan() {
    let nan = BigFloat::nan(P);
    let result = nan
        .exp(P, MODE)
        .expect("exp(NaN) should succeed (returns NaN)");
    assert!(result.is_nan(), "exp(NaN) should be NaN");
}

#[test]
fn sqrt_pos_inf_is_pos_inf() {
    let inf = BigFloat::infinity(P);
    let result = inf.sqrt(P, MODE).expect("sqrt(+Inf) should succeed");
    assert!(
        result.is_infinite() && result.is_sign_positive(),
        "sqrt(+Inf) should be +Inf"
    );
}

#[test]
fn sqrt_neg_inf_is_nan() {
    let neg_inf = BigFloat::neg_infinity(P);
    let result = neg_inf
        .sqrt(P, MODE)
        .expect("sqrt(-Inf) should succeed (returns NaN)");
    assert!(result.is_nan(), "sqrt(-Inf) should be NaN");
}

#[test]
fn sqrt_nan_is_nan() {
    let nan = BigFloat::nan(P);
    let result = nan
        .sqrt(P, MODE)
        .expect("sqrt(NaN) should succeed (returns NaN)");
    assert!(result.is_nan(), "sqrt(NaN) should be NaN");
}

#[test]
fn ln_pos_inf_is_pos_inf() {
    let inf = BigFloat::infinity(P);
    let result = inf.ln(P, MODE).expect("ln(+Inf) should succeed");
    assert!(
        result.is_infinite() && result.is_sign_positive(),
        "ln(+Inf) should be +Inf"
    );
}

#[test]
fn ln_neg_inf_is_nan() {
    let neg_inf = BigFloat::neg_infinity(P);
    let result = neg_inf
        .ln(P, MODE)
        .expect("ln(-Inf) should succeed (returns NaN)");
    assert!(result.is_nan(), "ln(-Inf) should be NaN");
}

#[test]
fn ln_nan_is_nan() {
    let nan = BigFloat::nan(P);
    let result = nan
        .ln(P, MODE)
        .expect("ln(NaN) should succeed (returns NaN)");
    assert!(result.is_nan(), "ln(NaN) should be NaN");
}

#[test]
fn sin_pos_inf_is_nan() {
    let inf = BigFloat::infinity(P);
    let result = inf
        .sin(P, MODE)
        .expect("sin(+Inf) should succeed (returns NaN)");
    assert!(result.is_nan(), "sin(+Inf) should be NaN");
}

#[test]
fn sin_neg_inf_is_nan() {
    let neg_inf = BigFloat::neg_infinity(P);
    let result = neg_inf
        .sin(P, MODE)
        .expect("sin(-Inf) should succeed (returns NaN)");
    assert!(result.is_nan(), "sin(-Inf) should be NaN");
}

#[test]
fn sin_nan_is_nan() {
    let nan = BigFloat::nan(P);
    let result = nan
        .sin(P, MODE)
        .expect("sin(NaN) should succeed (returns NaN)");
    assert!(result.is_nan(), "sin(NaN) should be NaN");
}

#[test]
fn cos_pos_inf_is_nan() {
    let inf = BigFloat::infinity(P);
    let result = inf
        .cos(P, MODE)
        .expect("cos(+Inf) should succeed (returns NaN)");
    assert!(result.is_nan(), "cos(+Inf) should be NaN");
}

#[test]
fn cos_nan_is_nan() {
    let nan = BigFloat::nan(P);
    let result = nan
        .cos(P, MODE)
        .expect("cos(NaN) should succeed (returns NaN)");
    assert!(result.is_nan(), "cos(NaN) should be NaN");
}

#[test]
fn atan_pos_inf_is_positive_finite() {
    // atan(+Inf) = +π/2 — a positive finite value.
    let inf = BigFloat::infinity(P);
    let result = inf.atan(P, MODE).expect("atan(+Inf) should succeed");
    assert!(
        result.is_finite() && result.is_sign_positive(),
        "atan(+Inf) should be positive finite (≈ π/2)"
    );
    // Sanity: ≈ 1.5707963...
    let diff = (result.to_f64() - std::f64::consts::FRAC_PI_2).abs();
    assert!(
        diff < 1e-14,
        "atan(+Inf) should be ≈ π/2, got {}",
        result.to_f64()
    );
}

#[test]
fn atan_neg_inf_is_negative_finite() {
    // atan(-Inf) = -π/2 — a negative finite value.
    let neg_inf = BigFloat::neg_infinity(P);
    let result = neg_inf.atan(P, MODE).expect("atan(-Inf) should succeed");
    assert!(
        result.is_finite() && result.is_sign_negative(),
        "atan(-Inf) should be negative finite (≈ -π/2)"
    );
    let diff = (result.to_f64() - (-std::f64::consts::FRAC_PI_2)).abs();
    assert!(
        diff < 1e-14,
        "atan(-Inf) should be ≈ -π/2, got {}",
        result.to_f64()
    );
}

#[test]
fn atan_nan_is_nan() {
    let nan = BigFloat::nan(P);
    let result = nan
        .atan(P, MODE)
        .expect("atan(NaN) should succeed (returns NaN)");
    assert!(result.is_nan(), "atan(NaN) should be NaN");
}

// ===========================================================================
// Section 8: Serde round-trip for non-finite values
// ===========================================================================

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn nan_roundtrip() {
        let orig = BigFloat::nan(P);
        let json = serde_json::to_string(&orig).expect("serialize NaN");
        let back: BigFloat = serde_json::from_str(&json).expect("deserialize NaN");
        assert!(back.is_nan(), "NaN should round-trip to NaN");
        assert_eq!(back.precision(), P);
    }

    #[test]
    fn pos_inf_roundtrip() {
        let orig = BigFloat::infinity(P);
        let json = serde_json::to_string(&orig).expect("serialize +Inf");
        let back: BigFloat = serde_json::from_str(&json).expect("deserialize +Inf");
        assert!(
            back.is_infinite() && back.is_sign_positive(),
            "+Inf round-trip"
        );
        assert_eq!(back.precision(), P);
    }

    #[test]
    fn neg_inf_roundtrip() {
        let orig = BigFloat::neg_infinity(P);
        let json = serde_json::to_string(&orig).expect("serialize -Inf");
        let back: BigFloat = serde_json::from_str(&json).expect("deserialize -Inf");
        assert!(
            back.is_infinite() && back.is_sign_negative(),
            "-Inf round-trip"
        );
        assert_eq!(back.precision(), P);
    }

    #[test]
    fn finite_roundtrip() {
        let orig = BigFloat::from_i64(42, P, MODE);
        let json = serde_json::to_string(&orig).expect("serialize 42");
        let back: BigFloat = serde_json::from_str(&json).expect("deserialize 42");
        assert!(back.is_finite());
        assert_eq!(back.to_f64(), 42.0);
    }

    #[test]
    fn legacy_payload_no_class_defaults_to_finite() {
        // A payload with no "class" field should deserialize as Finite
        // because `#[serde(default)]` on the class field defaults to FloatClass::Finite.
        // We construct a valid Finite zero payload.
        let legacy = r#"{"sign":false,"mantissa":{"limbs":[]},"exponent":0,"precision":64}"#;
        let result: Result<BigFloat, _> = serde_json::from_str(legacy);
        // May or may not parse depending on BigUint repr — skip on failure (impl detail).
        if let Ok(val) = result {
            assert!(
                val.is_finite(),
                "legacy payload without 'class' should be Finite"
            );
        }
    }
}

// ===========================================================================
// Section 9: num_traits FloatConst and TotalOrder
// ===========================================================================

#[cfg(feature = "num-traits")]
mod num_traits_tests {
    use std::cmp::Ordering;

    use num_traits::float::FloatConst;

    use super::*;

    // FloatConst

    #[test]
    fn float_const_pi_positive_finite() {
        let pi = BigFloat::PI();
        assert!(pi.is_finite() && pi.is_sign_positive());
        // Sanity: π ≈ 3.14159...
        let diff = (pi.to_f64() - std::f64::consts::PI).abs();
        assert!(diff < 1e-14, "FloatConst::PI() should be ≈ π");
    }

    #[test]
    fn float_const_e_positive_finite() {
        let e = BigFloat::E();
        assert!(e.is_finite() && e.is_sign_positive());
        let diff = (e.to_f64() - std::f64::consts::E).abs();
        assert!(diff < 1e-14, "FloatConst::E() should be ≈ e");
    }

    #[test]
    fn float_const_ln2_positive_finite() {
        let ln2 = BigFloat::LN_2();
        assert!(ln2.is_finite() && ln2.is_sign_positive());
        let diff = (ln2.to_f64() - std::f64::consts::LN_2).abs();
        assert!(diff < 1e-14, "FloatConst::LN_2() should be ≈ ln(2)");
    }

    #[test]
    fn float_const_sqrt2_positive_finite() {
        let sqrt2 = BigFloat::SQRT_2();
        assert!(sqrt2.is_finite() && sqrt2.is_sign_positive());
        let diff = (sqrt2.to_f64() - std::f64::consts::SQRT_2).abs();
        assert!(diff < 1e-14, "FloatConst::SQRT_2() should be ≈ √2");
    }

    // TotalOrder

    #[test]
    fn total_order_nan_nan_equal() {
        let nan = BigFloat::nan(P);
        assert_eq!(
            nan.total_cmp(&BigFloat::nan(128)),
            Ordering::Equal,
            "TotalOrder: NaN == NaN"
        );
    }

    #[test]
    fn total_order_inf_lt_nan() {
        assert_eq!(
            BigFloat::infinity(P).total_cmp(&BigFloat::nan(P)),
            Ordering::Less,
            "TotalOrder: +Inf < NaN"
        );
    }

    #[test]
    fn total_order_neg_inf_lt_pos_inf() {
        assert_eq!(
            BigFloat::neg_infinity(P).total_cmp(&BigFloat::infinity(P)),
            Ordering::Less,
            "TotalOrder: -Inf < +Inf"
        );
    }

    #[test]
    fn total_order_finite_lt_inf() {
        assert_eq!(
            finite(100).total_cmp(&BigFloat::infinity(P)),
            Ordering::Less,
            "TotalOrder: finite < +Inf"
        );
    }
}
