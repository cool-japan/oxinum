//! Integration tests for the `oxinum_core` trait implementations on the
//! native `BigUint` and `BigInt` types.
//!
//! IMPORTANT: All trait dispatch MUST use fully-qualified syntax
//! (e.g. `<BigUint as Roots>::sqrt(...)`) so that we test the trait impl,
//! not the identically-named inherent methods.

use oxinum_core::{
    FromRadix, ModularArithmetic, OxiNum, OxiSigned, Pow, Primality, Roots, Sign, ToRadix,
};
use oxinum_int::native::{prime_sieve, BigInt, BigUint};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn check_oxinum<T: OxiNum>(x: &T, expected_zero: bool, expected_one: bool) {
    assert_eq!(T::is_zero(x), expected_zero);
    assert_eq!(T::is_one(x), expected_one);
}

// ---------------------------------------------------------------------------
// OxiNum
// ---------------------------------------------------------------------------

#[test]
fn oxinum_biguint_is_zero_one() {
    check_oxinum(&BigUint::ZERO, true, false);
    check_oxinum(&BigUint::from_u64(1), false, true);
    check_oxinum(&BigUint::from_u64(42), false, false);
}

#[test]
fn oxinum_bigint_is_zero_one() {
    check_oxinum(&BigInt::ZERO, true, false);
    check_oxinum(&BigInt::one(), false, true);
    check_oxinum(&BigInt::from(42i64), false, false);
}

// ---------------------------------------------------------------------------
// Roots (BigUint)
// ---------------------------------------------------------------------------

#[test]
fn roots_sqrt() {
    assert_eq!(
        <BigUint as Roots>::sqrt(&BigUint::from_u64(9)),
        BigUint::from_u64(3)
    );
    assert_eq!(
        <BigUint as Roots>::sqrt(&BigUint::from_u64(8)),
        BigUint::from_u64(2)
    );
    assert_eq!(<BigUint as Roots>::sqrt(&BigUint::ZERO), BigUint::ZERO);
}

#[test]
fn roots_cbrt() {
    assert_eq!(
        <BigUint as Roots>::cbrt(&BigUint::from_u64(27)),
        BigUint::from_u64(3)
    );
    assert_eq!(
        <BigUint as Roots>::cbrt(&BigUint::from_u64(8)),
        BigUint::from_u64(2)
    );
    assert_eq!(
        <BigUint as Roots>::cbrt(&BigUint::from_u64(26)),
        BigUint::from_u64(2)
    );
}

#[test]
fn roots_nth_root() {
    assert_eq!(
        <BigUint as Roots>::nth_root(&BigUint::from_u64(32), 5),
        BigUint::from_u64(2)
    );
    assert_eq!(
        <BigUint as Roots>::nth_root(&BigUint::from_u64(81), 4),
        BigUint::from_u64(3)
    );
}

#[test]
#[should_panic]
fn roots_nth_root_zero_panics() {
    <BigUint as Roots>::nth_root(&BigUint::from_u64(10), 0);
}

// ---------------------------------------------------------------------------
// Pow<u32>
// ---------------------------------------------------------------------------

#[test]
fn pow_biguint() {
    assert_eq!(
        <BigUint as Pow<u32>>::pow(&BigUint::from_u64(2), 10),
        BigUint::from_u64(1024)
    );
    assert_eq!(
        <BigUint as Pow<u32>>::pow(&BigUint::from_u64(3), 0),
        BigUint::from_u64(1)
    );
    assert_eq!(
        <BigUint as Pow<u32>>::pow(&BigUint::ZERO, 0),
        BigUint::from_u64(1)
    );
}

#[test]
fn pow_bigint_sign() {
    // Positive base raised to odd power — stays positive.
    let pos = BigInt::from(2i64);
    assert_eq!(<BigInt as Pow<u32>>::pow(&pos, 3), BigInt::from(8i64));

    // Negative base, odd exp → negative.
    let neg = BigInt::from(-2i64);
    assert_eq!(<BigInt as Pow<u32>>::pow(&neg, 3), BigInt::from(-8i64));

    // Negative base, even exp → positive.
    assert_eq!(<BigInt as Pow<u32>>::pow(&neg, 2), BigInt::from(4i64));

    // Zero to any positive power → zero.
    assert_eq!(<BigInt as Pow<u32>>::pow(&BigInt::ZERO, 5), BigInt::ZERO);

    // Any non-zero base to the zeroth power → 1.
    assert_eq!(<BigInt as Pow<u32>>::pow(&neg, 0), BigInt::from(1i64));

    // Zeroth power of zero → 1 (mathematical convention).
    assert_eq!(
        <BigInt as Pow<u32>>::pow(&BigInt::ZERO, 0),
        BigInt::from(1i64)
    );
}

// ---------------------------------------------------------------------------
// ModularArithmetic (BigUint)
// ---------------------------------------------------------------------------

#[test]
fn modular_arith_add() {
    // (7 + 5) mod 6 = 12 mod 6 = 0
    let a = BigUint::from_u64(7);
    let b = BigUint::from_u64(5);
    let m = BigUint::from_u64(6);
    assert_eq!(
        <BigUint as ModularArithmetic>::mod_add(&a, &b, &m),
        BigUint::ZERO
    );
}

#[test]
fn modular_arith_sub() {
    // (2 - 5) mod 7 = -3 mod 7 = 4
    let a = BigUint::from_u64(2);
    let b = BigUint::from_u64(5);
    let m = BigUint::from_u64(7);
    assert_eq!(
        <BigUint as ModularArithmetic>::mod_sub(&a, &b, &m),
        BigUint::from_u64(4)
    );

    // (5 - 2) mod 7 = 3
    assert_eq!(
        <BigUint as ModularArithmetic>::mod_sub(&b, &a, &m),
        BigUint::from_u64(3)
    );

    // (n - n) mod m = 0
    let n = BigUint::from_u64(9);
    assert_eq!(
        <BigUint as ModularArithmetic>::mod_sub(&n, &n, &m),
        BigUint::ZERO
    );
}

#[test]
fn modular_arith_mul() {
    // (3 * 4) mod 5 = 12 mod 5 = 2
    let a = BigUint::from_u64(3);
    let b = BigUint::from_u64(4);
    let m = BigUint::from_u64(5);
    assert_eq!(
        <BigUint as ModularArithmetic>::mod_mul(&a, &b, &m),
        BigUint::from_u64(2)
    );
}

#[test]
fn modular_arith_pow() {
    // 2^10 mod 1000 = 1024 mod 1000 = 24
    let base = BigUint::from_u64(2);
    let exp = BigUint::from_u64(10);
    let m = BigUint::from_u64(1000);
    assert_eq!(
        <BigUint as ModularArithmetic>::mod_pow(&base, &exp, &m),
        BigUint::from_u64(24)
    );
}

#[test]
#[should_panic]
fn modular_arith_add_zero_modulus_panics() {
    let a = BigUint::from_u64(5);
    <BigUint as ModularArithmetic>::mod_add(&a, &a, &BigUint::ZERO);
}

#[test]
#[should_panic]
fn modular_arith_sub_zero_modulus_panics() {
    let a = BigUint::from_u64(5);
    <BigUint as ModularArithmetic>::mod_sub(&a, &a, &BigUint::ZERO);
}

// ---------------------------------------------------------------------------
// Primality (BigUint)
// ---------------------------------------------------------------------------

#[test]
fn primality_small_cases() {
    assert!(!<BigUint as Primality>::is_probably_prime(
        &BigUint::ZERO,
        0
    ));
    assert!(!<BigUint as Primality>::is_probably_prime(
        &BigUint::from_u64(1),
        0
    ));
    assert!(<BigUint as Primality>::is_probably_prime(
        &BigUint::from_u64(2),
        0
    ));
    assert!(<BigUint as Primality>::is_probably_prime(
        &BigUint::from_u64(17),
        0
    ));
    assert!(!<BigUint as Primality>::is_probably_prime(
        &BigUint::from_u64(4),
        0
    ));
}

#[test]
fn primality_matches_sieve_up_to_50() {
    let primes: HashSet<u64> = prime_sieve(51).into_iter().collect();
    for n in 2u64..=50 {
        let bn = BigUint::from_u64(n);
        assert_eq!(
            <BigUint as Primality>::is_probably_prime(&bn, 0),
            primes.contains(&n),
            "primality mismatch at n={n}"
        );
    }
}

#[test]
fn next_prime_basic() {
    assert_eq!(
        <BigUint as Primality>::next_prime(&BigUint::ZERO),
        BigUint::from_u64(2)
    );
    assert_eq!(
        <BigUint as Primality>::next_prime(&BigUint::from_u64(1)),
        BigUint::from_u64(2)
    );
    assert_eq!(
        <BigUint as Primality>::next_prime(&BigUint::from_u64(2)),
        BigUint::from_u64(3)
    );
    assert_eq!(
        <BigUint as Primality>::next_prime(&BigUint::from_u64(10)),
        BigUint::from_u64(11)
    );
}

#[test]
fn next_prime_always_greater_and_prime() {
    for n in 0u64..=100 {
        let bn = BigUint::from_u64(n);
        let np = <BigUint as Primality>::next_prime(&bn);
        assert!(np > bn, "next_prime({n}) must be > {n}");
        assert!(
            <BigUint as Primality>::is_probably_prime(&np, 0),
            "next_prime({n}) = {np:?} is not prime"
        );
    }
}

// ---------------------------------------------------------------------------
// FromRadix / ToRadix — BigUint roundtrip
// ---------------------------------------------------------------------------

#[test]
fn biguint_radix_roundtrip() {
    let x = BigUint::from_u64(123_456_789u64);
    for radix in [2u32, 8, 10, 16, 36] {
        let s = <BigUint as ToRadix>::to_radix(&x, radix).expect("to_radix");
        let y = <BigUint as FromRadix>::from_radix(&s, radix).expect("from_radix");
        assert_eq!(x, y, "BigUint roundtrip failed at radix {radix}");
    }
}

#[test]
fn biguint_zero_roundtrip() {
    let x = BigUint::ZERO;
    let s = <BigUint as ToRadix>::to_radix(&x, 10).expect("to_radix");
    let y = <BigUint as FromRadix>::from_radix(&s, 10).expect("from_radix");
    assert_eq!(x, y);
}

// ---------------------------------------------------------------------------
// FromRadix / ToRadix — BigInt roundtrip (including negative values)
// ---------------------------------------------------------------------------

#[test]
fn bigint_radix_roundtrip_positive() {
    let x = BigInt::from(987_654_321i64);
    for radix in [2u32, 10, 16] {
        let s = <BigInt as ToRadix>::to_radix(&x, radix).expect("to_radix");
        let y = <BigInt as FromRadix>::from_radix(&s, radix).expect("from_radix");
        assert_eq!(x, y, "BigInt positive roundtrip at radix {radix}");
    }
}

#[test]
fn bigint_radix_roundtrip_negative() {
    let x = BigInt::from(-123_456_789i64);
    for radix in [2u32, 10, 16] {
        let s = <BigInt as ToRadix>::to_radix(&x, radix).expect("to_radix");
        // Negative representation must start with '-'.
        assert!(
            s.starts_with('-'),
            "negative BigInt radix string should start with '-'"
        );
        let y = <BigInt as FromRadix>::from_radix(&s, radix).expect("from_radix");
        assert_eq!(x, y, "BigInt negative roundtrip at radix {radix}");
    }
}

#[test]
fn bigint_radix_zero() {
    let x = BigInt::ZERO;
    let s = <BigInt as ToRadix>::to_radix(&x, 10).expect("to_radix");
    assert_eq!(s, "0");
    let y = <BigInt as FromRadix>::from_radix(&s, 10).expect("from_radix");
    assert_eq!(x, y);
}

// ---------------------------------------------------------------------------
// OxiSigned (BigInt)
// ---------------------------------------------------------------------------

#[test]
fn oxisigned_bigint() {
    let pos = BigInt::from(5i64);
    let neg = BigInt::from(-5i64);

    assert_eq!(<BigInt as OxiSigned>::signum(&pos), Sign::Positive);
    assert_eq!(<BigInt as OxiSigned>::signum(&neg), Sign::Negative);
    assert_eq!(<BigInt as OxiSigned>::signum(&BigInt::ZERO), Sign::Positive);
    assert_eq!(<BigInt as OxiSigned>::abs(&neg), pos);
    assert_eq!(<BigInt as OxiSigned>::abs(&pos), pos);
    assert_eq!(<BigInt as OxiSigned>::abs(&BigInt::ZERO), BigInt::ZERO);
}
