//! SciRS2 floating-point API compatibility verification.
//!
//! Proves that `oxinum-float` satisfies the exact contract that
//! `scirs2-core/src/numeric/arbitrary_precision.rs` depends on, both at
//! compile time (type signatures) and at runtime (behavioral correctness).
//!
//! The SciRS2 consumer imports:
//!   ```ignore
//!   use oxinum_float::{
//!       compute_e, compute_ln2, compute_pi,
//!       cos, cosh, exp, ln,
//!       precision::with_precision,
//!       sin, sinh, sqrt, tan, tanh, DBig,
//!   };
//!   use oxinum_float::pow as oxinum_pow;
//!   use oxinum_float::atan as oxinum_atan;
//!   use oxinum_float::atan2 as oxinum_atan2;
//!   ```
//! It also calls `.to_f64()` on `DBig` (extracts `.value()`) and
//! `DBig::from_str` (standard `FromStr`).

use std::str::FromStr;

use oxinum_float::{
    atan, atan2, compute_e, compute_ln2, compute_pi, cos, cosh, exp, ln, pow,
    precision::with_precision, sin, sinh, sqrt, tan, tanh, DBig,
};

// -----------------------------------------------------------------------
// Compile-time contract: all symbols SciRS2 uses must resolve.
// -----------------------------------------------------------------------

#[allow(dead_code)]
fn _assert_float_contract() {
    // DBig construction.
    let zero = DBig::from(0_u32);
    let one = DBig::from(1_u32);

    // DBig::from_str.
    let _x: DBig = DBig::from_str("3.14").expect("parse");

    // with_precision.
    let _y = with_precision(&zero, 50);

    // compute_* constants.
    let _pi = compute_pi(30);
    let _e = compute_e(30);
    let _ln2 = compute_ln2(30);

    // Elementary functions (all take (&DBig, usize) → OxiNumResult<DBig>).
    let _ = sqrt(&one, 30);
    let _ = exp(&zero, 30);
    let _ = ln(&one, 30);
    let _ = pow(&one, &zero, 30);

    // Trigonometric functions.
    let _ = sin(&zero, 30);
    let _ = cos(&zero, 30);
    let _ = tan(&zero, 30);
    let _ = atan(&zero, 30);
    let _ = atan2(&zero, &one, 30);
    let _ = sinh(&zero, 30);
    let _ = cosh(&zero, 30);
    let _ = tanh(&zero, 30);

    // to_f64 / .value() pattern used by SciRS2.
    let v: f64 = one.to_f64().value();
    let _ = v;
}

// -----------------------------------------------------------------------
// Behavioural tests
// -----------------------------------------------------------------------

fn prec_40() -> usize {
    40
}

#[test]
fn dbig_from_str_and_to_f64() {
    let x = DBig::from_str("2.5").expect("parse 2.5");
    let f: f64 = x.to_f64().value();
    assert!((f - 2.5_f64).abs() < 1e-15);
}

#[test]
fn with_precision_changes_precision() {
    let x = DBig::from(1_u32);
    let x100 = with_precision(&x, 100);
    assert_eq!(x100.precision(), 100);
    let f: f64 = x100.to_f64().value();
    assert!((f - 1.0_f64).abs() < 1e-15);
}

#[test]
fn compute_pi_starts_with_3_14() {
    let pi = compute_pi(40);
    let s = pi.to_string();
    assert!(s.starts_with("3.14159265358979"), "pi = {s}");
}

#[test]
fn compute_e_starts_with_2_71() {
    let e = compute_e(40);
    let s = e.to_string();
    assert!(s.starts_with("2.71828182845904"), "e = {s}");
}

#[test]
fn compute_ln2_starts_with_0_69() {
    let ln2 = compute_ln2(40);
    let s = ln2.to_string();
    assert!(s.starts_with("0.693147180559945"), "ln2 = {s}");
}

#[test]
fn sqrt_of_four_is_two() {
    let four = DBig::from(4_u32);
    let r = sqrt(&four, 30).expect("sqrt ok");
    let f: f64 = r.to_f64().value();
    assert!((f - 2.0).abs() < 1e-14, "sqrt(4) = {f}");
}

#[test]
fn sqrt_negative_errors() {
    let neg_one = -DBig::from(1_u32);
    assert!(sqrt(&neg_one, 30).is_err());
}

#[test]
fn exp_of_zero_is_one() {
    let zero = DBig::from(0_u32);
    let r = exp(&zero, 30).expect("exp ok");
    let f: f64 = r.to_f64().value();
    assert!((f - 1.0).abs() < 1e-14);
}

#[test]
fn exp_of_one_is_e() {
    let one = DBig::from(1_u32);
    let r = exp(&one, 40).expect("exp ok");
    let f: f64 = r.to_f64().value();
    let e_f64 = std::f64::consts::E;
    assert!((f - e_f64).abs() < 1e-14, "exp(1) = {f}");
}

#[test]
fn ln_of_one_is_zero() {
    let one = DBig::from(1_u32);
    let r = ln(&one, 30).expect("ln ok");
    let f: f64 = r.to_f64().value();
    assert!(f.abs() < 1e-14);
}

#[test]
fn ln_non_positive_errors() {
    let zero = DBig::from(0_u32);
    assert!(ln(&zero, 30).is_err(), "ln(0) must error");
    let neg_one = -DBig::from(1_u32);
    assert!(ln(&neg_one, 30).is_err(), "ln(-1) must error");
}

#[test]
fn pow_two_ten_is_1024() {
    let two = DBig::from(2_u32);
    let ten = DBig::from(10_u32);
    let r = pow(&two, &ten, 30).expect("pow ok");
    let f: f64 = r.to_f64().value();
    assert!((f - 1024.0).abs() < 1e-10, "2^10 = {f}");
}

#[test]
fn sin_of_zero_is_zero() {
    let zero = DBig::from(0_u32);
    let r = sin(&zero, 30).expect("sin ok");
    let f: f64 = r.to_f64().value();
    assert!(f.abs() < 1e-14);
}

#[test]
fn cos_of_zero_is_one() {
    let zero = DBig::from(0_u32);
    let r = cos(&zero, 30).expect("cos ok");
    let f: f64 = r.to_f64().value();
    assert!((f - 1.0).abs() < 1e-14);
}

#[test]
fn pythagorean_identity_at_0_5() {
    let x = DBig::from_str("0.5").expect("parse");
    let s = sin(&x, 40).expect("sin ok");
    let c = cos(&x, 40).expect("cos ok");
    let s2 = s.clone() * s;
    let c2 = c.clone() * c;
    let sum = s2 + c2;
    let f: f64 = sum.to_f64().value();
    assert!((f - 1.0).abs() < 1e-12, "sin²+cos² = {f}");
}

#[test]
fn tan_of_zero_is_zero() {
    let zero = DBig::from(0_u32);
    let r = tan(&zero, 30).expect("tan ok");
    let f: f64 = r.to_f64().value();
    assert!(f.abs() < 1e-14);
}

#[test]
fn atan_of_one_is_pi_over_4() {
    let one = DBig::from(1_u32);
    let r = atan(&one, 40).expect("atan ok");
    let f: f64 = r.to_f64().value();
    let expected = std::f64::consts::FRAC_PI_4;
    assert!((f - expected).abs() < 1e-13, "atan(1) = {f}");
}

#[test]
fn atan2_quadrant_table() {
    // atan2(y=0, x=1) = 0; atan2(y=1, x=0) = π/2; atan2(y=0, x=-1) = π.
    let zero = DBig::from(0_u32);
    let one = DBig::from(1_u32);
    let neg_one = -DBig::from(1_u32);

    let r0 = atan2(&zero, &one, 30).expect("atan2 ok");
    assert!(r0.to_f64().value().abs() < 1e-14);

    let r_pi_2 = atan2(&one, &zero, 30).expect("atan2 ok");
    let f_pi_2: f64 = r_pi_2.to_f64().value();
    assert!((f_pi_2 - std::f64::consts::FRAC_PI_2).abs() < 1e-13);

    let r_pi = atan2(&zero, &neg_one, 40).expect("atan2 ok");
    let f_pi: f64 = r_pi.to_f64().value();
    assert!((f_pi - std::f64::consts::PI).abs() < 1e-13);
}

#[test]
fn sinh_zero_is_zero() {
    let zero = DBig::from(0_u32);
    let r = sinh(&zero, 30).expect("sinh ok");
    let f: f64 = r.to_f64().value();
    assert!(f.abs() < 1e-14);
}

#[test]
fn cosh_zero_is_one() {
    let zero = DBig::from(0_u32);
    let r = cosh(&zero, 30).expect("cosh ok");
    let f: f64 = r.to_f64().value();
    assert!((f - 1.0).abs() < 1e-14);
}

#[test]
fn tanh_zero_is_zero() {
    let zero = DBig::from(0_u32);
    let r = tanh(&zero, 30).expect("tanh ok");
    let f: f64 = r.to_f64().value();
    assert!(f.abs() < 1e-14);
}

#[test]
fn hyperbolic_identity_at_1() {
    // cosh²(x) - sinh²(x) = 1.
    let one = DBig::from(1_u32);
    let ch = cosh(&one, 40).expect("cosh ok");
    let sh = sinh(&one, 40).expect("sinh ok");
    let diff = ch.clone() * ch - sh.clone() * sh;
    let f: f64 = diff.to_f64().value();
    assert!((f - 1.0).abs() < 1e-12, "cosh²-sinh² = {f}");
}

#[test]
fn scirs2_asin_path_via_atan() {
    // The SciRS2 `asin` helper implements:
    //   asin(x) = atan(x / sqrt(1 - x²))
    // Verify that sqrt/atan compose correctly for x = 0.5.
    let prec = prec_40();
    let x = DBig::from_str("0.5").expect("parse");
    let x2 = x.clone() * x.clone();
    let one_minus_x2 = DBig::from(1_u32) - x2;
    let denom = sqrt(&one_minus_x2, prec + 4).expect("sqrt ok");
    let ratio = x.clone() / denom;
    let result = atan(&ratio, prec).expect("atan ok");
    let f: f64 = result.to_f64().value();
    let expected = (0.5_f64).asin();
    assert!(
        (f - expected).abs() < 1e-12,
        "asin(0.5) via atan = {f}, expected {expected}"
    );
}

#[test]
fn scirs2_f64_to_dbig_to_f64_roundtrip() {
    // Replicate the f64_to_dbig path used by SciRS2:
    //   let s = format!("{v:.17e}"); DBig::from_str(&s)
    for &v in &[
        0.0_f64,
        1.0,
        -1.0,
        0.5,
        std::f64::consts::PI,
        1e100,
        -1e-100,
    ] {
        let s = format!("{v:.17e}");
        let dbig = DBig::from_str(&s).unwrap_or_else(|_| DBig::from(0_u32));
        let back: f64 = dbig.to_f64().value();
        assert!(
            (back - v).abs() < v.abs() * 1e-14 + 1e-300,
            "round-trip failed for v={v}: got {back}"
        );
    }
}
