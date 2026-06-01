//! Integration tests for `num_traits` implementations on native integer types.
//!
//! Run with: `cargo nextest run -p oxinum-int --features num-traits`

#![cfg(feature = "num-traits")]

use num_traits::{Num, Signed, Unsigned, Zero};
use oxinum_int::native::{BigInt, BigUint};

// ---------------------------------------------------------------------------
// BigUint — Zero / One / ConstZero
// ---------------------------------------------------------------------------

#[test]
fn biguint_zero_is_zero() {
    let z = BigUint::zero();
    assert!(z.is_zero());
}

#[test]
fn biguint_one_is_one() {
    let o = BigUint::one();
    assert!(o.is_one());
    assert!(!o.is_zero());
}

#[test]
fn biguint_const_zero() {
    let z: BigUint = BigUint::ZERO;
    assert!(z.is_zero());
}

// ---------------------------------------------------------------------------
// BigUint — Num / Unsigned
// ---------------------------------------------------------------------------

#[test]
fn biguint_from_str_radix_decimal() {
    let n: BigUint = Num::from_str_radix("1000", 10).expect("decimal");
    assert_eq!(n, BigUint::from_u64(1000));
}

#[test]
fn biguint_from_str_radix_hex() {
    let n: BigUint = Num::from_str_radix("ff", 16).expect("hex");
    assert_eq!(n, BigUint::from_u64(255));
}

#[test]
fn biguint_from_str_radix_binary() {
    let n: BigUint = Num::from_str_radix("10101010", 2).expect("binary");
    assert_eq!(n, BigUint::from_u64(0b10101010));
}

#[test]
fn biguint_is_unsigned() {
    fn needs_unsigned<T: Unsigned>(_: &T) {}
    let n = BigUint::from_u64(42);
    needs_unsigned(&n);
}

// ---------------------------------------------------------------------------
// BigUint — generic sum via Zero + Add
// ---------------------------------------------------------------------------

#[test]
fn biguint_generic_sum() {
    fn sum<T: Zero + std::ops::Add<Output = T>>(xs: Vec<T>) -> T {
        xs.into_iter().fold(T::zero(), |a, b| a + b)
    }
    let vals = vec![
        BigUint::from_u64(1),
        BigUint::from_u64(2),
        BigUint::from_u64(3),
    ];
    assert_eq!(sum(vals), BigUint::from_u64(6));
}

// ---------------------------------------------------------------------------
// BigUint — Sub operator (added to satisfy Num + NumOps)
// ---------------------------------------------------------------------------

#[test]
fn biguint_sub_basic() {
    let a = BigUint::from_u64(100);
    let b = BigUint::from_u64(40);
    assert_eq!(a - b, BigUint::from_u64(60));
}

#[test]
#[should_panic]
fn biguint_sub_underflow_panics() {
    let a = BigUint::from_u64(5);
    let b = BigUint::from_u64(10);
    let _ = a - b;
}

// ---------------------------------------------------------------------------
// BigInt — Zero / One / ConstZero
// ---------------------------------------------------------------------------

#[test]
fn bigint_zero_is_zero() {
    let z = BigInt::zero();
    assert!(z.is_zero());
}

#[test]
fn bigint_one_is_one() {
    let o = BigInt::one();
    assert!(o.is_one());
    assert!(!o.is_zero());
}

#[test]
fn bigint_const_zero() {
    let z: BigInt = BigInt::ZERO;
    assert!(z.is_zero());
}

// ---------------------------------------------------------------------------
// BigInt — Num
// ---------------------------------------------------------------------------

#[test]
fn bigint_from_str_radix_positive() {
    let n: BigInt = Num::from_str_radix("12345", 10).expect("decimal");
    assert_eq!(n, BigInt::from(12345i64));
}

#[test]
fn bigint_from_str_radix_negative() {
    let n: BigInt = Num::from_str_radix("-99", 10).expect("negative decimal");
    assert_eq!(n, BigInt::from(-99i64));
}

#[test]
fn bigint_from_str_radix_hex() {
    let n: BigInt = Num::from_str_radix("ff", 16).expect("hex");
    assert_eq!(n, BigInt::from(255i64));
}

// ---------------------------------------------------------------------------
// BigInt — Signed
// ---------------------------------------------------------------------------

#[test]
fn bigint_abs() {
    let neg = BigInt::from(-42i64);
    assert_eq!(Signed::abs(&neg), BigInt::from(42i64));
    let pos = BigInt::from(7i64);
    assert_eq!(Signed::abs(&pos), BigInt::from(7i64));
}

#[test]
fn bigint_signum() {
    assert_eq!(Signed::signum(&BigInt::from(-5i64)), BigInt::from(-1i64));
    assert_eq!(Signed::signum(&BigInt::from(5i64)), BigInt::from(1i64));
    assert_eq!(Signed::signum(&BigInt::zero()), BigInt::zero());
}

#[test]
fn bigint_is_positive_negative() {
    let neg = BigInt::from(-10i64);
    assert!(neg.is_negative());
    assert!(!neg.is_positive());
    let pos = BigInt::from(10i64);
    assert!(pos.is_positive());
    assert!(!pos.is_negative());
    assert!(!BigInt::zero().is_positive());
    assert!(!BigInt::zero().is_negative());
}

#[test]
fn bigint_abs_sub() {
    let a = BigInt::from(10i64);
    let b = BigInt::from(3i64);
    // a > b → a - b
    assert_eq!(a.abs_sub(&b), BigInt::from(7i64));
    // a < b → 0
    assert_eq!(b.abs_sub(&a), BigInt::zero());
}

// ---------------------------------------------------------------------------
// BigInt — generic usage
// ---------------------------------------------------------------------------

#[test]
fn bigint_generic_sum() {
    fn sum<T: Zero + std::ops::Add<Output = T>>(xs: Vec<T>) -> T {
        xs.into_iter().fold(T::zero(), |a, b| a + b)
    }
    let vals = vec![
        BigInt::from(-1i64),
        BigInt::from(2i64),
        BigInt::from(-3i64),
        BigInt::from(4i64),
    ];
    assert_eq!(sum(vals), BigInt::from(2i64));
}
