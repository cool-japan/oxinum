//! Serde JSON round-trip integration tests for both complex types.
//!
//! The whole file body is gated on the `serde` feature; without it the file
//! compiles to nothing (so `cargo test` without `--features serde` simply has
//! no cases here). Both [`oxinum_complex::CBig`] and
//! [`oxinum_complex::native::BigComplex`] serialize to a flat `{re, im}` object
//! and must reconstruct identically through `serde_json`.
#![cfg(feature = "serde")]

use oxinum_complex::native::BigComplex;
use oxinum_complex::CBig;

#[test]
fn cbig_json_round_trip() {
    let z = CBig::from_f64(3.0, -4.0).expect("finite parts");
    let json = serde_json::to_string(&z).expect("serialize CBig");
    let back: CBig = serde_json::from_str(&json).expect("deserialize CBig");
    // Component-wise PartialEq is the authoritative equality for CBig.
    assert!(back == z, "round-trip mismatch: {json}");
    assert_eq!(back.re().to_string(), "3");
    assert_eq!(back.im().to_string(), "-4");
}

#[test]
fn cbig_fractional_json_round_trip() {
    let z = CBig::from_f64(1.5, -2.25).expect("finite parts");
    let json = serde_json::to_string(&z).expect("serialize CBig");
    let back: CBig = serde_json::from_str(&json).expect("deserialize CBig");
    assert!(back == z, "round-trip mismatch: {json}");
}

#[test]
fn big_complex_json_round_trip() {
    let z = BigComplex::from_f64(1.0, 2.0, 64).expect("finite parts");
    let json = serde_json::to_string(&z).expect("serialize BigComplex");
    let back: BigComplex = serde_json::from_str(&json).expect("deserialize BigComplex");
    // BigComplex is PartialEq component-wise; compare both via that and f64.
    assert!(back == z, "round-trip mismatch: {json}");
    assert_eq!(back.to_f64_parts(), z.to_f64_parts());
}

#[test]
fn big_complex_negative_imag_round_trip() {
    let z = BigComplex::from_f64(-2.5, -7.0, 80).expect("finite parts");
    let json = serde_json::to_string(&z).expect("serialize BigComplex");
    let back: BigComplex = serde_json::from_str(&json).expect("deserialize BigComplex");
    assert!(back == z, "round-trip mismatch: {json}");
}
