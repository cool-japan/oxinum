//! Integration tests for the F1 milestone wrapper-polish items:
//!
//! * `precision::with_precision` rebinds working precision.
//! * `precision::epsilon` returns the canonical decimal epsilon at a
//!   chosen precision.
//! * `precision::ulp` returns the unit in the last place at a value's
//!   carried precision.
//! * `serde` feature gates JSON round-trip for `DBig`.
//!
//! See `crates/oxinum-float/TODO.md` -> F1.

#![forbid(unsafe_code)]

use std::str::FromStr;

use oxinum_float::{
    precision::{epsilon, ulp, with_precision},
    DBig, OxiNumError,
};

// ---------------------------------------------------------------------------
// with_precision
// ---------------------------------------------------------------------------

#[test]
fn with_precision_rebinds_to_requested_precision() {
    let a = DBig::from_str("1.234").expect("parse 1.234");
    assert_eq!(a.precision(), 4);

    // Up-rebind: no loss of information.
    let up = with_precision(&a, 100);
    assert_eq!(up.precision(), 100);
    assert_eq!(up, a, "up-rebinding must preserve numeric value");

    // Down-rebind: precision shrinks; numeric value may round.
    let down = with_precision(&a, 2);
    assert_eq!(down.precision(), 2);
}

#[test]
fn with_precision_zero_means_unlimited() {
    // `precision == 0` is dashu's "unlimited" sentinel.  We pass it
    // through unchanged.  The carried precision is then 0.
    let a = DBig::from_str("1.234").expect("parse 1.234");
    let unlimited = with_precision(&a, 0);
    assert_eq!(unlimited.precision(), 0);
}

// ---------------------------------------------------------------------------
// epsilon
// ---------------------------------------------------------------------------

#[test]
fn epsilon_10_matches_one_e_minus_nine_at_precision_10() {
    let eps = epsilon(10).expect("epsilon(10)");
    assert_eq!(eps.precision(), 10);

    let expected = DBig::from_str("1e-9")
        .expect("parse 1e-9")
        .with_precision(10)
        .value();
    assert_eq!(eps, expected, "epsilon(10) should equal 1e-9 @ p10");
}

#[test]
fn epsilon_1_matches_one() {
    // precision == 1 -> 10^(1-1) = 10^0 = 1 (at precision 1).
    let eps = epsilon(1).expect("epsilon(1)");
    assert_eq!(eps.precision(), 1);
    let expected = DBig::from_str("1")
        .expect("parse 1")
        .with_precision(1)
        .value();
    assert_eq!(eps, expected);
}

#[test]
fn epsilon_50_matches_one_e_minus_49_at_precision_50() {
    let eps = epsilon(50).expect("epsilon(50)");
    assert_eq!(eps.precision(), 50);
    let expected = DBig::from_str("1e-49")
        .expect("parse 1e-49")
        .with_precision(50)
        .value();
    assert_eq!(eps, expected);
}

#[test]
fn epsilon_zero_is_an_error() {
    match epsilon(0) {
        Err(OxiNumError::Precision(_)) => {}
        other => panic!("expected Precision error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// ulp
// ---------------------------------------------------------------------------

#[test]
fn ulp_of_one_point_two_three_is_one_hundredth() {
    // Mirrors the dashu_float::FBig::ulp doctest.
    let x = DBig::from_str("1.23").expect("parse 1.23");
    let u = ulp(&x).expect("ulp at finite precision");
    assert_eq!(u, DBig::from_str("0.01").expect("parse 0.01"));
}

#[test]
fn ulp_preserves_carried_precision() {
    // `ulp(x)` is expressed at the same precision as `x` — this is the
    // invariant we rely on for downstream error analysis.
    for s in ["1.23", "0.001", "1234567890", "9.87654321e-12"] {
        let x = DBig::from_str(s).unwrap_or_else(|e| panic!("parse {s}: {e}"));
        let u = ulp(&x).expect("ulp");
        assert_eq!(
            u.precision(),
            x.precision(),
            "ulp({s}) must keep precision {} (got {})",
            x.precision(),
            u.precision(),
        );
    }
}

#[test]
fn ulp_at_precision_10_matches_epsilon_10_at_unit_value() {
    // For a value whose magnitude is 1.0 carried at decimal precision p,
    // ulp(x) coincides with epsilon(p): both name "10^(1 - p)".
    let one_at_p10 = DBig::from_str("1")
        .expect("parse 1")
        .with_precision(10)
        .value();
    assert_eq!(one_at_p10.precision(), 10);

    let u = ulp(&one_at_p10).expect("ulp(1 @ p10)");
    let eps = epsilon(10).expect("epsilon(10)");
    assert_eq!(u, eps, "ulp(1 @ p10) should match epsilon(10)");
}

#[test]
fn ulp_of_unlimited_precision_is_an_error() {
    // `DBig::from_parts_const(...)` with no min_precision can produce a
    // value with precision > 0, so we deliberately rebind to 0
    // ("unlimited") to exercise the error path.
    let unlimited = DBig::from_str("1.23")
        .expect("parse 1.23")
        .with_precision(0)
        .value();
    assert_eq!(unlimited.precision(), 0);
    match ulp(&unlimited) {
        Err(OxiNumError::Precision(_)) => {}
        other => panic!("expected Precision error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// serde JSON round-trip (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn dbig_json_roundtrip_via_dashu_serde() {
    // The serde impl lives on `dashu_float::FBig` (gated by
    // `dashu-float/serde`); we exercise it through the `DBig` re-export.
    for s in ["0", "1.23", "-4.5e-10", "3.14159265358979"] {
        let x = DBig::from_str(s).unwrap_or_else(|e| panic!("parse {s}: {e}"));
        let json = serde_json::to_string(&x).expect("serialize DBig");
        let back: DBig = serde_json::from_str(&json).expect("deserialize DBig");
        assert_eq!(back, x, "round-trip mismatch for {s} via JSON: {json}");
    }
}
