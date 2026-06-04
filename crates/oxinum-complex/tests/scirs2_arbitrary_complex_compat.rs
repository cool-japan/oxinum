//! Contract verification for `oxinum_complex::CBig` as used by
//! `scirs2-core::ArbitraryComplex`.
//!
//! Source under test:
//!   `../scirs/scirs2-core/src/numeric/arbitrary_precision.rs`
//!
//! `ArbitraryComplex` is a concrete wrapper around `CBig`. This file proves
//! two things without importing SciRS2 (which would create a dependency cycle):
//!
//! 1. **Compile-time contract** — `_assert_contract()` (never called) enumerates
//!    every method called by the consumer with exact argument and return types.
//!    Future signature drift in `oxinum-complex` will break *this* crate's CI
//!    long before it silently breaks SciRS2.
//!
//! 2. **Behavioural contract** — `#[test]` functions mirror the consumer's
//!    call patterns and verify correctness at the precision level used by
//!    `ArbitraryComplex::prec_digits()` (which calls `bits_to_decimal_digits`
//!    with 256-bit default precision, but the tests use 128 bits as the minimum
//!    representative case).
//!
//! Date: 2026-06-03.

use oxinum_complex::{CBig, DBig, OxiNumResult};

// ---------------------------------------------------------------------------
// Precision helper — replicates the consumer's `bits_to_decimal_digits`
// ---------------------------------------------------------------------------

/// Convert bit precision to decimal-digit count.
///
/// Mirrors `bits_to_decimal_digits` in `arbitrary_precision.rs`:
/// ```text
/// ((bits as f64) / LOG2_10).ceil() as usize
/// ```
fn bits_to_decimal_digits(bits: u32) -> usize {
    let bits_per_decimal_digit: f64 = std::f64::consts::LOG2_10;
    ((bits as f64) / bits_per_decimal_digit).ceil() as usize
}

// ---------------------------------------------------------------------------
// Compile-time contract
//
// This function is intentionally never called. Its sole purpose is to assert
// at *compile time* that every method the consumer invokes exists with the
// exact signatures it depends on.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn _assert_contract() {
    // CBig::zero() → CBig
    let _: CBig = CBig::zero();

    // CBig::from_f64(re: f64, im: f64) → OxiNumResult<CBig>
    let _: OxiNumResult<CBig> = CBig::from_f64(0.0_f64, 0.0_f64);

    // .to_f64_parts() → (f64, f64)
    let z = CBig::zero();
    let _: (f64, f64) = z.to_f64_parts();

    // .conj() → CBig
    let _: CBig = z.conj();

    // .abs(precision: usize) → OxiNumResult<DBig>
    let _: OxiNumResult<DBig> = z.abs(20_usize);

    // .arg(precision: usize) → OxiNumResult<DBig>
    let _: OxiNumResult<DBig> = z.arg(20_usize);

    // .ln(precision: usize) → OxiNumResult<CBig>
    let _: OxiNumResult<CBig> = z.ln(20_usize);

    // .exp(precision: usize) → OxiNumResult<CBig>
    let _: OxiNumResult<CBig> = z.exp(20_usize);

    // .sqrt(precision: usize) → OxiNumResult<CBig>
    let _: OxiNumResult<CBig> = z.sqrt(20_usize);

    // .pow(&CBig, precision: usize) → OxiNumResult<CBig>
    let _: OxiNumResult<CBig> = z.pow(&CBig::one(), 20_usize);

    // Owned Add / Sub / Mul / Div / Neg
    let a = CBig::zero();
    let b = CBig::one();
    let _: CBig = a.clone() + b.clone();
    let _: CBig = a.clone() - b.clone();
    let _: CBig = a.clone() * b.clone();
    let _: CBig = a.clone() / b.clone();
    let _: CBig = -a.clone();

    // PartialEq
    let _: bool = a == b;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Assert two `f64` values are within `epsilon` of each other.
fn assert_approx(actual: f64, expected: f64, epsilon: f64, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff < epsilon,
        "{label}: expected ≈{expected:.15}, got {actual:.15} (diff {diff:.3e})"
    );
}

// ---------------------------------------------------------------------------
// Behavioural tests
// ---------------------------------------------------------------------------

/// Consumer pattern: `CBig::zero()` is the additive identity.
#[test]
fn test_zero() {
    let z = CBig::zero();
    let (re, im) = z.to_f64_parts();
    assert_eq!(re, 0.0, "zero real part");
    assert_eq!(im, 0.0, "zero imag part");
}

/// Consumer pattern: `CBig::from_f64(re, im)` and `.to_f64_parts()` round-trip.
#[test]
fn test_from_f64_roundtrip() {
    let z = CBig::from_f64(3.0, 4.0).expect("finite inputs must succeed");
    let (re, im) = z.to_f64_parts();
    assert_approx(re, 3.0, 1e-15, "re");
    assert_approx(im, 4.0, 1e-15, "im");
}

/// Consumer pattern: `CBig::from_f64` with NaN → `Err`; the consumer uses
/// `.unwrap_or_else(|_| CBig::zero())` so it must return an error for NaN.
#[test]
fn test_from_f64_nonfinite_returns_err() {
    assert!(
        CBig::from_f64(f64::NAN, 0.0).is_err(),
        "NaN real part must be rejected"
    );
    assert!(
        CBig::from_f64(0.0, f64::NAN).is_err(),
        "NaN imag part must be rejected"
    );
    assert!(
        CBig::from_f64(f64::INFINITY, 0.0).is_err(),
        "+Inf real part must be rejected"
    );
    assert!(
        CBig::from_f64(0.0, f64::NEG_INFINITY).is_err(),
        "-Inf imag part must be rejected"
    );
}

/// Consumer pattern: `.conj()` negates the imaginary part.
#[test]
fn test_conj() {
    let z = CBig::from_f64(3.0, 4.0).expect("finite inputs");
    let c = z.conj();
    let (re, im) = c.to_f64_parts();
    assert_approx(re, 3.0, 1e-15, "conj re");
    assert_approx(im, -4.0, 1e-15, "conj im");
}

/// Consumer pattern: owned `+`, `-`, `*`, `/`, unary `-`.
#[test]
fn test_arithmetic_ops() {
    let a = CBig::from_f64(1.0, 2.0).expect("finite");
    let b = CBig::from_f64(3.0, 4.0).expect("finite");
    let eps = 1e-10_f64;

    // (1+2i) + (3+4i) = 4+6i
    let sum = a.clone() + b.clone();
    let (re, im) = sum.to_f64_parts();
    assert_approx(re, 4.0, eps, "add re");
    assert_approx(im, 6.0, eps, "add im");

    // (1+2i) - (3+4i) = -2-2i
    let diff = a.clone() - b.clone();
    let (re, im) = diff.to_f64_parts();
    assert_approx(re, -2.0, eps, "sub re");
    assert_approx(im, -2.0, eps, "sub im");

    // (1+2i) * (3+4i) = (3-8) + (4+6)i = -5+10i
    let prod = a.clone() * b.clone();
    let (re, im) = prod.to_f64_parts();
    assert_approx(re, -5.0, eps, "mul re");
    assert_approx(im, 10.0, eps, "mul im");

    // (3+4i) / (3+4i) = 1
    let quot = b.clone() / b.clone();
    let (re, im) = quot.to_f64_parts();
    assert_approx(re, 1.0, eps, "div re");
    assert_approx(im, 0.0, eps, "div im");

    // -(1+2i) = -1-2i
    let neg = -a.clone();
    let (re, im) = neg.to_f64_parts();
    assert_approx(re, -1.0, eps, "neg re");
    assert_approx(im, -2.0, eps, "neg im");
}

/// Consumer pattern: `PartialEq` comparison.
#[test]
fn test_partial_eq() {
    let z1 = CBig::from_f64(1.5, -2.0).expect("finite");
    let z2 = CBig::from_f64(1.5, -2.0).expect("finite");
    let z3 = CBig::from_f64(1.5, 2.0).expect("finite");

    assert!(z1 == z2, "equal values must compare equal");
    assert!(z1 != z3, "different values must compare unequal");
    assert!(CBig::zero() == CBig::zero(), "zero == zero");
}

/// Consumer pattern: `z.abs(digits)` on (3+4i) ≈ 5.
#[test]
fn test_abs_3_4_is_5() {
    let digits = bits_to_decimal_digits(128);
    let z = CBig::from_f64(3.0, 4.0).expect("finite");
    let mag = z.abs(digits).expect("abs must succeed for finite input");
    let mag_f64 = mag.to_f64().value();
    assert_approx(mag_f64, 5.0, 1e-10, "|3+4i|");
}

/// Consumer pattern: `z.arg(digits)` on (0+1i) ≈ π/2.
#[test]
fn test_arg_purely_imaginary() {
    let digits = bits_to_decimal_digits(128);
    let z = CBig::i();
    let arg = z.arg(digits).expect("arg must succeed for non-zero input");
    let arg_f64 = arg.to_f64().value();
    let half_pi = std::f64::consts::FRAC_PI_2;
    assert_approx(arg_f64, half_pi, 1e-10, "arg(i)");
}

/// Consumer pattern: `exp(ln(z)) ≈ z` for a non-zero z.
#[test]
fn test_exp_ln_roundtrip() {
    let digits = bits_to_decimal_digits(128);
    let z = CBig::from_f64(2.0, 3.0).expect("finite");
    let ln_z = z.ln(digits).expect("ln of non-zero complex must succeed");
    let exp_ln_z = ln_z.exp(digits).expect("exp must succeed");
    let (re, im) = exp_ln_z.to_f64_parts();
    assert_approx(re, 2.0, 1e-10, "exp(ln(z)) re");
    assert_approx(im, 3.0, 1e-10, "exp(ln(z)) im");
}

/// Consumer pattern: `sqrt(z)² ≈ z` for a positive-real z.
#[test]
fn test_sqrt_squared() {
    let digits = bits_to_decimal_digits(128);
    let z = CBig::from_f64(9.0, 0.0).expect("finite");
    let sq = z
        .sqrt(digits)
        .expect("sqrt of non-negative real must succeed");
    let sq2 = sq.clone() * sq.clone();
    let (re, im) = sq2.to_f64_parts();
    assert_approx(re, 9.0, 1e-10, "sqrt(9)^2 re");
    assert_approx(im, 0.0, 1e-10, "sqrt(9)^2 im");
}

/// Consumer pattern: Euler's identity `exp(iπ) + 1 ≈ 0`.
#[test]
fn test_eulers_identity() {
    let digits = bits_to_decimal_digits(128);

    // Build iπ = CBig::from_f64(0.0, π). π is available via std because DBig
    // itself uses f64 for seeding; we can also pass the consumer's own path:
    // the precision is set by `bits_to_decimal_digits`, not the seed value.
    let pi_f64 = std::f64::consts::PI;
    // Use `from_parts` via DBig to get more precision than raw f64 allows,
    // but for the purpose of this compatibility test, f64-seeded is sufficient.
    let i_pi = CBig::from_f64(0.0, pi_f64).expect("pi is finite");

    let result = i_pi.exp(digits).expect("exp(iπ) must succeed");
    let (re, im) = result.to_f64_parts();

    // exp(iπ) = -1 + 0i, so exp(iπ) + 1 = 0 + 0i.
    assert_approx(re + 1.0, 0.0, 1e-10, "exp(iπ).re + 1");
    assert_approx(im, 0.0, 1e-10, "exp(iπ).im");
}

/// Consumer pattern: `z.pow(&w, digits)` — verify i⁴ ≈ 1.
#[test]
fn test_pow_i_fourth_is_one() {
    let digits = bits_to_decimal_digits(128);
    let i = CBig::i();
    // Exponent: 4 as a purely-real CBig. Consumer uses CBig directly for the exponent.
    let four = CBig::from_f64(4.0, 0.0).expect("finite");
    let result = i.pow(&four, digits).expect("i^4 must succeed");
    let (re, im) = result.to_f64_parts();
    assert_approx(re, 1.0, 1e-10, "i^4 re");
    assert_approx(im, 0.0, 1e-10, "i^4 im");
}
