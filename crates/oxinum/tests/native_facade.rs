//! Smoke tests for the `oxinum::native` re-export namespace.
//!
//! These tests verify only that every native type and helper resolves
//! through the top-level facade; behavioural correctness is exercised in
//! the underlying `oxinum-int` / `oxinum-float` / `oxinum-rational`
//! crates.

#![cfg(feature = "pure")]

use oxinum::native;
use oxinum::native::{BigComplex, BigFloat, BigInt, BigRational, BigUint, RoundingMode};

#[test]
fn native_biguint_resolves() {
    let n: BigUint = BigUint::from(7_u64);
    assert_eq!(format!("{n}"), "7");
}

#[test]
fn native_bigint_resolves() {
    let n: BigInt = BigInt::from(42_i64);
    assert_eq!(format!("{n}"), "42");
}

#[test]
fn native_bigrational_resolves() {
    // BigRational::from_integer takes BigInt by value.
    let r = BigRational::from_integer(BigInt::from(5_i64));
    assert!(r.is_integer());
}

#[test]
fn native_bigfloat_resolves() {
    // BigFloat::from_i64(n, precision, mode).
    let f = BigFloat::from_i64(3_i64, 50, RoundingMode::HalfEven);
    assert!(!f.is_zero());
}

#[test]
fn native_bigcomplex_resolves() {
    // BigComplex::new(re, im) over native BigFloat.
    let re = BigFloat::from_i64(3_i64, 64, RoundingMode::HalfEven);
    let im = BigFloat::from_i64(4_i64, 64, RoundingMode::HalfEven);
    let z = BigComplex::new(re, im);
    // |3 + 4i|^2 = 25.
    assert_eq!(z.norm_sqr().to_f64(), 25.0);
}

#[test]
fn native_aliases_resolve() {
    let _: native::Int = native::Int::from(1_i64);
    let _: native::Natural = native::Natural::from(2_u64);
    let _: native::Float = native::Float::from_i64(3_i64, 30, RoundingMode::HalfEven);
    let _: native::Rational = native::Rational::from_integer(BigInt::from(4_i64));
    let _: native::Complex = native::Complex::new(
        native::Float::from_i64(5_i64, 30, RoundingMode::HalfEven),
        native::Float::from_i64(6_i64, 30, RoundingMode::HalfEven),
    );
}

#[test]
fn native_int_helpers_resolve() {
    // gcd / gcd_binary / gcd_int / divrem / divrem_int / KARATSUBA_THRESHOLD
    // must all be reachable through the facade.
    let g = native::gcd(BigUint::from(12_u64), BigUint::from(18_u64));
    assert_eq!(format!("{g}"), "6");

    let g_bin = native::gcd_binary(BigUint::from(12_u64), BigUint::from(18_u64));
    assert_eq!(g_bin, g);

    let i = BigInt::from(-12_i64);
    let j = BigInt::from(18_i64);
    let g_int = native::gcd_int(&i, &j);
    assert_eq!(format!("{g_int}"), "6");

    let (q, r) = native::divrem(&BigUint::from(17_u64), &BigUint::from(5_u64));
    assert_eq!(format!("{q}"), "3");
    assert_eq!(format!("{r}"), "2");

    let (qi, ri) = native::divrem_int(&BigInt::from(17_i64), &BigInt::from(5_i64));
    assert_eq!(format!("{qi}"), "3");
    assert_eq!(format!("{ri}"), "2");

    // Just confirm the constant is reachable and sensible. Evaluated at
    // compile time so the static assertion does not trip
    // `clippy::assertions_on_constants`.
    const _KARATSUBA_NONZERO: () = assert!(native::KARATSUBA_THRESHOLD > 0);
}
