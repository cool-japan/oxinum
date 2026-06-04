//! SciRS2 integer API compatibility verification.
//!
//! Proves that `oxinum-int` satisfies the exact contract that
//! `scirs2-core/src/numeric/arbitrary_precision.rs` depends on, both at
//! compile time (type signatures) and at runtime (behavioral correctness).
//!
//! The SciRS2 consumer uses:
//!   - `use oxinum_int::{is_prime, IBig, UBig};`
//!   - `IBig::from(n: i64/i32/...)`
//!   - `IBig::from(0i32)` zero check
//!   - `UBig::from_str(&s)` (requires `std::str::FromStr`)
//!   - `oxinum_int::ibig_from_radix(s, radix)` → `OxiNumResult<IBig>`
//!   - `oxinum_int::factorial(n: u32)` → `UBig`
//!   - `oxinum_int::binomial(n, k: u32)` → `UBig`
//!   - `use oxinum_int::Gcd;` + `.gcd()` on IBig → UBig
//!   - `oxinum_int::mod_pow(&base_u, &exp_u, &mod_u)` → `OxiNumResult<UBig>`
//!   - `is_prime(&UBig, reps: u32)` → `bool`
//!   - `IBig::from(u: UBig)` (wrapping UBig in IBig for return path)

use std::str::FromStr;

use oxinum_int::{binomial, factorial, ibig_from_radix, is_prime, mod_pow, Gcd, IBig, UBig};

// -----------------------------------------------------------------------
// Compile-time contract assertions (functions never called)
// -----------------------------------------------------------------------

#[allow(dead_code)]
fn _assert_int_contract() {
    // IBig constructors / ops used by SciRS2.
    let _ibig_zero: IBig = IBig::from(0_i32);
    let _ibig_from_i64: IBig = IBig::from(123_i64);
    let _ibig_from_ubig: IBig = IBig::from(UBig::ONE);

    // UBig constructors.
    let _ubig_one: UBig = UBig::ONE;
    let _ubig_zero: UBig = UBig::ZERO;

    // UBig::from_str must exist (used in mod_pow helper).
    let _ubig_from_str: Result<UBig, _> = UBig::from_str("42");

    // ibig_from_radix signature: (s: &str, radix: u32) -> OxiNumResult<IBig>.
    let _r: oxinum_int::OxiNumResult<IBig> = ibig_from_radix("ff", 16);

    // factorial / binomial return UBig.
    let _f: UBig = factorial(5);
    let _b: UBig = binomial(5, 2);

    // GCD via trait.
    let a = IBig::from(12_i32);
    let b = IBig::from(8_i32);
    let _g: UBig = a.gcd(&b);

    // mod_pow signature.
    let base = UBig::ONE;
    let exp = UBig::ONE;
    let modulus = UBig::from(7_u32);
    let _mp: oxinum_int::OxiNumResult<UBig> = mod_pow(&base, &exp, &modulus);

    // is_prime signature.
    let n = UBig::from(7_u32);
    let _p: bool = is_prime(&n, 0);
}

// -----------------------------------------------------------------------
// Behavioural tests
// -----------------------------------------------------------------------

#[test]
fn ibig_from_zero() {
    let z = IBig::from(0_i32);
    assert_eq!(z, IBig::from(0_i64));
}

#[test]
fn ibig_from_i64_round_trip() {
    for &v in &[-1_i64, 0, 1, i64::MIN, i64::MAX] {
        let ibig = IBig::from(v);
        assert_eq!(ibig.to_string(), v.to_string());
    }
}

#[test]
fn ubig_from_str_decimal() {
    let n = UBig::from_str("12345").expect("parse ok");
    assert_eq!(n, UBig::from(12345_u32));
}

#[test]
fn ibig_from_radix_hex() {
    let n = ibig_from_radix("ff", 16).expect("parse hex ok");
    assert_eq!(n, IBig::from(255_i32));
}

#[test]
fn ibig_from_radix_binary() {
    let n = ibig_from_radix("1010", 2).expect("parse binary ok");
    assert_eq!(n, IBig::from(10_i32));
}

#[test]
fn ibig_from_radix_invalid_digit() {
    assert!(ibig_from_radix("g", 16).is_err(), "g is not hex");
}

#[test]
fn factorial_small() {
    assert_eq!(factorial(0), UBig::from(1_u32));
    assert_eq!(factorial(1), UBig::from(1_u32));
    assert_eq!(factorial(5), UBig::from(120_u32));
    assert_eq!(factorial(10), UBig::from(3628800_u32));
}

#[test]
fn factorial_20_exact() {
    let f20 = factorial(20);
    assert_eq!(f20.to_string(), "2432902008176640000");
}

#[test]
fn binomial_small() {
    assert_eq!(binomial(5, 0), UBig::from(1_u32));
    assert_eq!(binomial(5, 5), UBig::from(1_u32));
    assert_eq!(binomial(5, 2), UBig::from(10_u32));
    assert_eq!(binomial(10, 3), UBig::from(120_u32));
}

#[test]
fn binomial_k_greater_than_n_is_zero() {
    // SciRS2 guards `k > n` explicitly; verify our impl also returns 0.
    assert_eq!(binomial(3, 5), UBig::ZERO);
}

#[test]
fn gcd_trait_on_ibig() {
    let a = IBig::from(48_i32);
    let b = IBig::from(18_i32);
    let g: UBig = a.gcd(&b);
    assert_eq!(g, UBig::from(6_u32));
}

#[test]
fn gcd_trait_zero_inputs() {
    let zero = IBig::from(0_i32);
    let n = IBig::from(5_i32);
    let g: UBig = zero.gcd(&n);
    assert_eq!(g, UBig::from(5_u32));
}

#[test]
fn mod_pow_basic() {
    // 2^10 mod 1000 = 24.
    let base = UBig::from(2_u32);
    let exp = UBig::from(10_u32);
    let modulus = UBig::from(1000_u32);
    let result = mod_pow(&base, &exp, &modulus).expect("no error");
    assert_eq!(result, UBig::from(24_u32));
}

#[test]
fn mod_pow_fermat_little_theorem() {
    // a^(p-1) ≡ 1 (mod p) for prime p and gcd(a,p)=1.
    for &p in &[7_u32, 13, 101, 65537] {
        let a = UBig::from(3_u32);
        let exp = UBig::from(p - 1);
        let modulus = UBig::from(p);
        let result = mod_pow(&a, &exp, &modulus).expect("no error");
        assert_eq!(result, UBig::ONE, "Fermat failed for p={p}");
    }
}

#[test]
fn mod_pow_zero_modulus_errors() {
    let base = UBig::from(2_u32);
    let exp = UBig::from(3_u32);
    let modulus = UBig::ZERO;
    assert!(mod_pow(&base, &exp, &modulus).is_err());
}

#[test]
fn is_prime_known_primes() {
    // deterministic witnesses (reps=0 triggers deterministic mode).
    for &p in &[2_u32, 3, 5, 7, 11, 13, 97, 65537, 999983] {
        let n = UBig::from(p);
        assert!(is_prime(&n, 0), "{p} should be prime");
    }
}

#[test]
fn is_prime_known_composites() {
    for &c in &[4_u32, 9, 15, 25, 561, 1105, 1729] {
        let n = UBig::from(c);
        assert!(!is_prime(&n, 0), "{c} should be composite");
    }
}

#[test]
fn is_prime_one_is_not_prime() {
    assert!(!is_prime(&UBig::ONE, 0));
}

#[test]
fn scirs2_arbitrary_int_mod_pow_path() {
    // Replicate the SciRS2 mod_pow helper path:
    // convert IBig to string → parse as UBig → call mod_pow.
    let base_ibig = IBig::from(2_i64);
    let exp_ibig = IBig::from(10_i64);
    let mod_ibig = IBig::from(1000_i64);

    let base_u = UBig::from_str(&base_ibig.to_string()).expect("base parse");
    let exp_u = UBig::from_str(&exp_ibig.to_string()).expect("exp parse");
    let mod_u = UBig::from_str(&mod_ibig.to_string()).expect("mod parse");

    let result = mod_pow(&base_u, &exp_u, &mod_u).expect("mod_pow ok");
    assert_eq!(result, UBig::from(24_u32));
}

#[test]
fn scirs2_is_probably_prime_path() {
    // Replicate the SciRS2 is_probably_prime path:
    // IBig → to_string → UBig::from_str → is_prime.
    let prime_ibig = IBig::from(97_i64);
    let composite_ibig = IBig::from(98_i64);

    let prime_u = UBig::from_str(&prime_ibig.to_string()).expect("parse prime");
    let comp_u = UBig::from_str(&composite_ibig.to_string()).expect("parse composite");

    assert!(is_prime(&prime_u, 20));
    assert!(!is_prime(&comp_u, 20));
}
