//! Integration tests for the `oxinum` facade crate.
//!
//! These tests exercise real dashu arithmetic through the `oxinum` public API
//! and serve as M1 acceptance tests: all operations must produce correct
//! results, not just compile.

use oxinum::{IBig, RBig, UBig};
use std::str::FromStr;

// ── Integer tests ──────────────────────────────────────────────────────────

#[test]
fn ubig_add_commutative() {
    let a = UBig::from(123_456_789u64);
    let b = UBig::from(987_654_321u64);
    assert_eq!(&a + &b, &b + &a);
}

#[test]
fn ubig_mul_commutative() {
    let a = UBig::from(123_456_789u64);
    let b = UBig::from(987_654_321u64);
    assert_eq!(&a * &b, &b * &a);
}

#[test]
fn ibig_add_commutative() {
    let a = IBig::from(-123_456_789i64);
    let b = IBig::from(987_654_321i64);
    assert_eq!(&a + &b, &b + &a);
}

#[test]
fn ibig_from_str_radix_hex() {
    let v = IBig::from_str_radix("deadbeef", 16).expect("valid hex");
    assert_eq!(v, IBig::from(0xdead_beef_u64));
}

#[test]
fn ibig_in_radix_roundtrip() {
    for radix in [2u32, 8, 10, 16] {
        let original = IBig::from(123_456_789i64);
        let s = format!("{}", original.in_radix(radix));
        let parsed = IBig::from_str_radix(&s, radix).expect("roundtrip parse failed");
        assert_eq!(original, parsed, "radix {radix} roundtrip failed");
    }
}

#[test]
fn ibig_negative_str_radix() {
    let v = IBig::from_str_radix("-deadbeef", 16).expect("valid negative hex");
    assert_eq!(v, IBig::from(-(0xdead_beef_i64)));
}

#[test]
fn ubig_pow() {
    let two = UBig::from(2u32);
    assert_eq!(two.pow(10usize), UBig::from(1024u32));
    assert_eq!(UBig::from(2u32).pow(20usize), UBig::from(1_048_576u32));
}

#[test]
fn ibig_large_pow() {
    // 2^100 — known exact decimal representation (31 digits)
    let big = UBig::from(2u32).pow(100usize);
    let ibig = IBig::from(big);
    let s = ibig.to_string();
    assert!(s.len() > 20, "2^100 should have many digits: {s}");
    assert_eq!(s, "1267650600228229401496703205376");
}

// ── Rational tests ─────────────────────────────────────────────────────────

#[test]
fn rbig_from_parts() {
    let r = RBig::from_parts(IBig::from(355), UBig::from(113u32));
    assert_eq!(r.numerator(), &IBig::from(355));
    assert_eq!(r.denominator(), &UBig::from(113u32));
}

#[test]
fn rbig_arithmetic_half_plus_third() {
    // 1/2 + 1/3 = 5/6
    let half = RBig::from_parts(IBig::from(1), UBig::from(2u32));
    let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
    let sum = half + third;
    assert_eq!(sum.numerator(), &IBig::from(5));
    assert_eq!(sum.denominator(), &UBig::from(6u32));
}

#[test]
fn rbig_sub_fractions() {
    // 3/4 - 1/4 = 1/2
    let three_quarters = RBig::from_parts(IBig::from(3), UBig::from(4u32));
    let one_quarter = RBig::from_parts(IBig::from(1), UBig::from(4u32));
    let diff = three_quarters - one_quarter;
    assert_eq!(diff.numerator(), &IBig::from(1));
    assert_eq!(diff.denominator(), &UBig::from(2u32));
}

#[test]
fn relaxed_canonicalize() {
    use oxinum::Relaxed;
    // Relaxed stores unreduced fractions; canonicalize reduces them.
    // gcd(15, 6) = 3 → -15/6 → -5/2
    let r = Relaxed::from_parts(IBig::from(-15), UBig::from(6u32));
    assert_eq!(r.numerator(), &IBig::from(-15));
    assert_eq!(r.denominator(), &UBig::from(6u32));
    let canonical = r.canonicalize();
    assert_eq!(canonical.numerator(), &IBig::from(-5));
    assert_eq!(canonical.denominator(), &UBig::from(2u32));
}

// ── Float tests ────────────────────────────────────────────────────────────

#[test]
fn dbig_from_str() {
    use oxinum::DBig;
    let d = DBig::from_str("3.14").expect("valid decimal");
    let s = d.to_string();
    assert!(s.starts_with("3.14"), "got: {s}");
}

#[test]
fn dbig_arithmetic_non_integer_result() {
    use oxinum::DBig;
    // 1.25 + 2.50 = 3.75  (avoids integer result that elides the decimal point)
    let a = DBig::from_str("1.25").expect("ok");
    let b = DBig::from_str("2.50").expect("ok");
    let sum = a + b;
    assert_eq!(sum.to_string(), "3.75");
}

#[test]
fn dbig_mul_pi_times_two() {
    use oxinum::DBig;
    let pi = DBig::from_str("3.14159265358979323846").expect("ok");
    let two = DBig::from(2u32);
    let two_pi = &pi * &two;
    assert_eq!(two_pi.to_string(), "6.28318530717958647692");
}
