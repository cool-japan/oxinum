//! Integration tests for `BigRational`'s `OxiNum`, `OxiSigned`, and `Pow<u32>`
//! trait implementations.

use oxinum_core::{OxiNum, OxiSigned, Pow, Sign};
use oxinum_int::native::{BigInt, BigUint};
use oxinum_rational::native::BigRational;

fn make_rational(num: i64, den: u64) -> BigRational {
    BigRational::from_parts(BigInt::from(num), BigUint::from_u64(den)).expect("valid rational")
}

// ---------------------------------------------------------------------------
// OxiNum
// ---------------------------------------------------------------------------

#[test]
fn oxinum_rational_zero_one() {
    let zero = make_rational(0, 1);
    let one = make_rational(1, 1);
    let half = make_rational(1, 2);

    assert!(OxiNum::is_zero(&zero));
    assert!(!OxiNum::is_one(&zero));

    assert!(!OxiNum::is_zero(&one));
    assert!(OxiNum::is_one(&one));

    assert!(!OxiNum::is_zero(&half));
    assert!(!OxiNum::is_one(&half));
}

#[test]
fn oxinum_rational_negative_not_one() {
    let neg_one = make_rational(-1, 1);
    assert!(!OxiNum::is_zero(&neg_one));
    assert!(!OxiNum::is_one(&neg_one));
}

// ---------------------------------------------------------------------------
// OxiSigned
// ---------------------------------------------------------------------------

#[test]
fn oxisigned_positive() {
    let pos = make_rational(3, 4);
    assert_eq!(OxiSigned::signum(&pos), Sign::Positive);
}

#[test]
fn oxisigned_negative() {
    let neg = make_rational(-3, 4);
    assert_eq!(OxiSigned::signum(&neg), Sign::Negative);
}

#[test]
fn oxisigned_zero_is_positive_sign() {
    // Canonical-zero convention: signum of zero returns Sign::Positive.
    let zero = make_rational(0, 1);
    assert_eq!(OxiSigned::signum(&zero), Sign::Positive);
}

#[test]
fn oxisigned_abs_of_negative() {
    let neg = make_rational(-3, 4);
    let abs_neg = OxiSigned::abs(&neg);
    assert_eq!(OxiSigned::signum(&abs_neg), Sign::Positive);
    assert_eq!(abs_neg, make_rational(3, 4));
}

#[test]
fn oxisigned_abs_of_positive_unchanged() {
    let pos = make_rational(5, 7);
    let abs_pos = OxiSigned::abs(&pos);
    assert_eq!(abs_pos, pos);
}

#[test]
fn oxisigned_is_negative_is_positive() {
    let pos = make_rational(2, 3);
    let neg = make_rational(-2, 3);
    let zero = make_rational(0, 1);

    assert!(OxiSigned::is_positive(&pos));
    assert!(!OxiSigned::is_negative(&pos));

    assert!(!OxiSigned::is_positive(&neg));
    assert!(OxiSigned::is_negative(&neg));

    // Zero is neither positive nor negative.
    assert!(!OxiSigned::is_positive(&zero));
    assert!(!OxiSigned::is_negative(&zero));
}

// ---------------------------------------------------------------------------
// Pow<u32>
// ---------------------------------------------------------------------------

#[test]
fn pow_zero_exponent_is_one() {
    let half = make_rational(1, 2);
    let result = Pow::<u32>::pow(&half, 0);
    assert_eq!(result, make_rational(1, 1));
}

#[test]
fn pow_exponent_one_is_identity() {
    let r = make_rational(2, 5);
    assert_eq!(Pow::<u32>::pow(&r, 1), r);
}

#[test]
fn pow_half_to_fourth() {
    // (1/2)^4 = 1/16
    let half = make_rational(1, 2);
    let result = Pow::<u32>::pow(&half, 4);
    assert_eq!(result, make_rational(1, 16));
}

#[test]
fn pow_two_thirds_cubed() {
    // (2/3)^3 = 8/27
    let two_thirds = make_rational(2, 3);
    let result = Pow::<u32>::pow(&two_thirds, 3);
    assert_eq!(result, make_rational(8, 27));
}

#[test]
fn pow_one_to_large_exponent() {
    // 1^100 = 1
    let one = make_rational(1, 1);
    let result = Pow::<u32>::pow(&one, 100);
    assert_eq!(result, make_rational(1, 1));
}

#[test]
fn pow_negative_base_even_exponent_positive() {
    // (-1/2)^2 = 1/4
    let neg_half = make_rational(-1, 2);
    let result = Pow::<u32>::pow(&neg_half, 2);
    assert_eq!(result, make_rational(1, 4));
    assert_eq!(OxiSigned::signum(&result), Sign::Positive);
}

#[test]
fn pow_negative_base_odd_exponent_negative() {
    // (-1/2)^3 = -1/8
    let neg_half = make_rational(-1, 2);
    let result = Pow::<u32>::pow(&neg_half, 3);
    assert_eq!(result, make_rational(-1, 8));
    assert_eq!(OxiSigned::signum(&result), Sign::Negative);
}

#[test]
fn pow_auto_reduces() {
    // (1/2)^2 = 1/4 (auto-reduced, not 2/8)
    let r = make_rational(1, 2);
    let result = Pow::<u32>::pow(&r, 2);
    assert_eq!(result, make_rational(1, 4));
}

#[test]
fn pow_zero_base_positive_exponent() {
    // 0^3 = 0
    let zero = make_rational(0, 1);
    let result = Pow::<u32>::pow(&zero, 3);
    assert_eq!(result, make_rational(0, 1));
}
