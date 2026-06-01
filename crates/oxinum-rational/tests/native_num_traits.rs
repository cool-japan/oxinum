//! Integration tests for `num_traits` implementations on native `BigRational`.
//!
//! Run with: `cargo nextest run -p oxinum-rational --features num-traits`

#![cfg(feature = "num-traits")]

use num_traits::{Num, Signed, Zero};
use oxinum_int::native::{BigInt, BigUint};
use oxinum_rational::native::BigRational;

fn rat(n: i64, d: u64) -> BigRational {
    BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d)).expect("non-zero denominator")
}

// ---------------------------------------------------------------------------
// Zero / One
// ---------------------------------------------------------------------------

#[test]
fn bigrational_zero_is_zero() {
    let z = BigRational::zero();
    assert!(z.is_zero());
}

#[test]
fn bigrational_one_is_one() {
    let o = BigRational::one();
    assert!(o.is_one());
    assert!(!o.is_zero());
}

// ---------------------------------------------------------------------------
// Num (from_str_radix)
// ---------------------------------------------------------------------------

#[test]
fn bigrational_from_str_integer() {
    let n: BigRational = Num::from_str_radix("7", 10).expect("integer");
    assert_eq!(n, BigRational::from_i64(7));
}

#[test]
fn bigrational_from_str_fraction() {
    let r: BigRational = Num::from_str_radix("3/4", 10).expect("fraction");
    assert_eq!(r, rat(3, 4));
}

#[test]
fn bigrational_from_str_negative() {
    let r: BigRational = Num::from_str_radix("-5/2", 10).expect("negative fraction");
    assert_eq!(r, rat(-5, 2));
}

#[test]
fn bigrational_from_str_reduces() {
    // 6/4 → 3/2
    let r: BigRational = Num::from_str_radix("6/4", 10).expect("reducible");
    assert_eq!(r, rat(3, 2));
}

#[test]
fn bigrational_from_str_non_decimal_error() {
    let result: Result<BigRational, _> = Num::from_str_radix("ff/1", 16);
    assert!(result.is_err(), "non-decimal radix should fail");
}

// ---------------------------------------------------------------------------
// Signed
// ---------------------------------------------------------------------------

#[test]
fn bigrational_abs() {
    let neg = rat(-3, 4);
    let result = Signed::abs(&neg);
    assert_eq!(result, rat(3, 4));
}

#[test]
fn bigrational_signum() {
    let pos = rat(1, 2);
    let neg = rat(-1, 2);
    let s_pos: BigRational = Signed::signum(&pos);
    let s_neg: BigRational = Signed::signum(&neg);
    let s_zero: BigRational = Signed::signum(&BigRational::zero());
    assert_eq!(s_pos, BigRational::from_i64(1));
    assert_eq!(s_neg, BigRational::from_i64(-1));
    assert!(s_zero.is_zero());
}

#[test]
fn bigrational_is_positive_negative() {
    let pos = rat(1, 3);
    let neg = rat(-1, 3);
    assert!(pos.is_positive());
    assert!(!pos.is_negative());
    assert!(neg.is_negative());
    assert!(!neg.is_positive());
    assert!(!BigRational::zero().is_positive());
    assert!(!BigRational::zero().is_negative());
}

#[test]
fn bigrational_abs_sub() {
    let a = rat(7, 4);
    let b = rat(1, 2);
    // a > b → a - b = 7/4 - 2/4 = 5/4
    let r1 = a.abs_sub(&b);
    assert_eq!(r1, rat(5, 4));
    // b < a → 0
    let r2 = b.abs_sub(&a);
    assert!(r2.is_zero());
}

// ---------------------------------------------------------------------------
// Generic usage
// ---------------------------------------------------------------------------

#[test]
fn bigrational_generic_sum() {
    fn sum<T: Zero + std::ops::Add<Output = T>>(xs: Vec<T>) -> T {
        xs.into_iter().fold(T::zero(), |a, b| a + b)
    }
    let vals = vec![rat(1, 2), rat(1, 3), rat(1, 6)];
    // 1/2 + 1/3 + 1/6 = 3/6 + 2/6 + 1/6 = 6/6 = 1
    assert_eq!(sum(vals), BigRational::one());
}
