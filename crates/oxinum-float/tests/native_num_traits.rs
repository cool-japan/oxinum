//! Integration tests for `num_traits` implementations on native `BigFloat`.
//!
//! Run with: `cargo nextest run -p oxinum-float --features num-traits`

#![cfg(feature = "num-traits")]

use num_traits::{Num, One, Signed, Zero};
use oxinum_float::native::{BigFloat, RoundingMode};

// ---------------------------------------------------------------------------
// Zero / One
// ---------------------------------------------------------------------------

#[test]
fn bigfloat_zero_is_zero() {
    let z: BigFloat = Zero::zero();
    assert!(z.is_zero());
}

#[test]
fn bigfloat_one_is_one() {
    let o: BigFloat = One::one();
    assert!(!o.is_zero());
    assert_eq!(o.to_f64(), 1.0);
}

#[test]
fn bigfloat_zero_plus_one() {
    let z: BigFloat = Zero::zero();
    let o: BigFloat = One::one();
    let sum = z + o;
    assert_eq!(sum.to_f64(), 1.0);
}

// ---------------------------------------------------------------------------
// Num (from_str_radix)
// ---------------------------------------------------------------------------

#[test]
fn bigfloat_from_str_radix_decimal() {
    // Parse a simple decimal that is not an approximation of any well-known constant.
    let n: BigFloat = Num::from_str_radix("1.5", 10).expect("decimal float");
    assert_eq!(n.to_f64(), 1.5_f64);
}

#[test]
fn bigfloat_from_str_radix_integer() {
    let n: BigFloat = Num::from_str_radix("42", 10).expect("integer");
    assert_eq!(n.to_f64(), 42.0);
}

#[test]
fn bigfloat_from_str_radix_non_decimal_error() {
    let result: Result<BigFloat, _> = Num::from_str_radix("ff", 16);
    assert!(result.is_err(), "non-decimal radix should fail");
}

// ---------------------------------------------------------------------------
// Signed
// ---------------------------------------------------------------------------

#[test]
fn bigfloat_abs() {
    let neg = BigFloat::from_i64(-7, 53, RoundingMode::HalfEven);
    let result = Signed::abs(&neg);
    assert_eq!(result.to_f64(), 7.0);
    assert!(result.signum() > 0 || result.is_zero());
}

#[test]
fn bigfloat_signum_positive() {
    let pos = BigFloat::from_i64(5, 53, RoundingMode::HalfEven);
    let s: BigFloat = Signed::signum(&pos);
    assert_eq!(s.to_f64(), 1.0);
}

#[test]
fn bigfloat_signum_negative() {
    let neg = BigFloat::from_i64(-5, 53, RoundingMode::HalfEven);
    let s: BigFloat = Signed::signum(&neg);
    assert_eq!(s.to_f64(), -1.0);
}

#[test]
fn bigfloat_signum_zero() {
    let zero: BigFloat = Zero::zero();
    let s: BigFloat = Signed::signum(&zero);
    assert!(s.is_zero());
}

#[test]
fn bigfloat_is_positive_negative() {
    let pos = BigFloat::from_i64(3, 53, RoundingMode::HalfEven);
    let neg = BigFloat::from_i64(-3, 53, RoundingMode::HalfEven);
    let zero: BigFloat = Zero::zero();
    assert!(pos.is_positive());
    assert!(!pos.is_negative());
    assert!(neg.is_negative());
    assert!(!neg.is_positive());
    assert!(!zero.is_positive());
    assert!(!zero.is_negative());
}

#[test]
fn bigfloat_abs_sub() {
    let a = BigFloat::from_i64(10, 53, RoundingMode::HalfEven);
    let b = BigFloat::from_i64(3, 53, RoundingMode::HalfEven);
    // a > b → a - b = 7
    let r1 = a.abs_sub(&b);
    assert_eq!(r1.to_f64(), 7.0);
    // b < a → 0
    let r2 = b.abs_sub(&a);
    assert!(r2.is_zero());
}

// ---------------------------------------------------------------------------
// Rem operator (required by Num / NumOps)
// ---------------------------------------------------------------------------

#[test]
fn bigfloat_rem_basic_integer() {
    // 10 % 3 = 1 (truncating remainder: trunc(10/3) = 3, 10 - 3*3 = 1)
    let a = BigFloat::from_i64(10, 53, RoundingMode::HalfEven);
    let b = BigFloat::from_i64(3, 53, RoundingMode::HalfEven);
    let r = a % b;
    assert_eq!(r.to_f64(), 1.0, "10 % 3 should be 1, got {}", r.to_f64());
}

#[test]
fn bigfloat_rem_negative_dividend() {
    // -10 % 3 = -1 (truncating toward zero: trunc(-10/3) = -3, -10 - (-3)*3 = -1)
    let a = BigFloat::from_i64(-10, 53, RoundingMode::HalfEven);
    let b = BigFloat::from_i64(3, 53, RoundingMode::HalfEven);
    let r = a % b;
    assert_eq!(
        r.to_f64(),
        -1.0,
        "-10 % 3 (truncating) should be -1, got {}",
        r.to_f64()
    );
}

#[test]
fn bigfloat_rem_fractional() {
    // 0.5 % 0.2 ≈ 0.1 (truncating: trunc(0.5/0.2) = trunc(2.5) = 2, 0.5 - 2*0.2 = 0.1)
    let a = BigFloat::from_f64(0.5, 53).expect("0.5");
    let b = BigFloat::from_f64(0.2, 53).expect("0.2");
    let r = a % b;
    let diff = (r.to_f64() - 0.1_f64).abs();
    assert!(diff < 1e-10, "0.5 % 0.2 should be ~0.1, got {}", r.to_f64());
}

#[test]
fn bigfloat_rem_zero_dividend() {
    // 0 % n = 0
    let zero: BigFloat = Zero::zero();
    let b = BigFloat::from_i64(5, 53, RoundingMode::HalfEven);
    let r = zero % b;
    assert!(r.is_zero(), "0 % 5 should be 0");
}

#[test]
fn bigfloat_rem_by_zero_is_nan() {
    let a = BigFloat::from_i64(5, 53, RoundingMode::HalfEven);
    let zero: BigFloat = Zero::zero();
    let result = a % zero;
    assert!(result.is_nan(), "5 % 0 should be NaN per IEEE 754");
}

// ---------------------------------------------------------------------------
// Generic usage
// ---------------------------------------------------------------------------

#[test]
fn bigfloat_generic_sum() {
    fn sum<T: Zero + std::ops::Add<Output = T>>(xs: Vec<T>) -> T {
        xs.into_iter().fold(T::zero(), |a, b| a + b)
    }
    let vals = vec![
        BigFloat::from_i64(1, 53, RoundingMode::HalfEven),
        BigFloat::from_i64(2, 53, RoundingMode::HalfEven),
        BigFloat::from_i64(3, 53, RoundingMode::HalfEven),
    ];
    let result = sum(vals);
    assert_eq!(result.to_f64(), 6.0);
}
