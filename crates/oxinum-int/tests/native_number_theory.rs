//! Integration tests for number-theory primitives in `oxinum_int::native`.
//!
//! Tests cover:
//! - `prime_sieve`: correctness and count checks.
//! - `is_probably_prime`: matches sieve for small n, Carmichael composites,
//!   Mersenne primes.
//! - `factorial`: exact small values, cross-validation vs naive.
//! - `lucas_uv`: Fibonacci/Lucas number sequences, modular reduction.

use oxinum_int::native::{factorial, is_probably_prime, lucas_uv, prime_sieve, BigUint};

fn bu(n: u64) -> BigUint {
    BigUint::from(n)
}

// ---------------------------------------------------------------------------
// Sieve tests
// ---------------------------------------------------------------------------

#[test]
fn sieve_primes_up_to_100() {
    let expected = vec![
        2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83,
        89, 97,
    ];
    assert_eq!(prime_sieve(100), expected);
}

#[test]
fn sieve_count_1000() {
    assert_eq!(prime_sieve(1000).len(), 168);
}

#[test]
fn sieve_count_10000() {
    assert_eq!(prime_sieve(10_000).len(), 1229);
}

#[test]
fn sieve_empty_for_small_limits() {
    assert!(prime_sieve(0).is_empty());
    assert!(prime_sieve(1).is_empty());
}

#[test]
fn sieve_starts_with_two() {
    let primes = prime_sieve(100);
    assert_eq!(primes[0], 2);
}

// ---------------------------------------------------------------------------
// Primality tests
// ---------------------------------------------------------------------------

#[test]
fn primality_trivial_values() {
    assert!(!is_probably_prime(&bu(0)));
    assert!(!is_probably_prime(&bu(1)));
    assert!(is_probably_prime(&bu(2)));
    assert!(is_probably_prime(&bu(3)));
    assert!(!is_probably_prime(&bu(4)));
}

#[test]
fn primality_matches_sieve_up_to_10000() {
    let sieve_primes = prime_sieve(10_000);
    for n in 2u64..10_000 {
        let expected = sieve_primes.binary_search(&n).is_ok();
        let got = is_probably_prime(&bu(n));
        assert_eq!(got, expected, "primality mismatch at n={}", n);
    }
}

#[test]
fn carmichael_composites_are_rejected() {
    // Carmichael numbers pass Fermat's test for all coprime bases but are
    // correctly identified as composite by Miller-Rabin.
    for &n in &[561u64, 1105, 1729, 2465, 2821, 6601, 8911, 10585] {
        assert!(
            !is_probably_prime(&bu(n)),
            "Carmichael {} was incorrectly identified as prime",
            n
        );
    }
}

#[test]
fn mersenne_primes_are_accepted() {
    // 2^p - 1 for known Mersenne prime exponents p.
    for p in [7u32, 13, 17, 19, 31] {
        let m = BigUint::from(2u64)
            .pow(p)
            .checked_sub(&BigUint::one())
            .expect("2^p > 1");
        assert!(is_probably_prime(&m), "2^{}-1 should be prime", p);
    }
}

#[test]
fn mersenne_composite_is_rejected() {
    // 2^11 - 1 = 2047 = 23 × 89 is composite.
    let m2047 = bu(2047);
    assert!(!is_probably_prime(&m2047));
}

#[test]
fn large_known_prime_m31() {
    // 2^31 - 1 = 2,147,483,647 is a well-known Mersenne prime (M_31).
    let m31 = BigUint::from(2u64)
        .pow(31)
        .checked_sub(&BigUint::one())
        .expect("2^31 > 1");
    assert!(is_probably_prime(&m31));
}

// ---------------------------------------------------------------------------
// Factorial tests
// ---------------------------------------------------------------------------

#[test]
fn factorial_base_cases() {
    assert_eq!(factorial(0), bu(1));
    assert_eq!(factorial(1), bu(1));
    assert_eq!(factorial(2), bu(2));
    assert_eq!(factorial(3), bu(6));
}

#[test]
fn factorial_small_exact() {
    assert_eq!(factorial(5), bu(120));
    assert_eq!(factorial(10), bu(3_628_800));
    assert_eq!(factorial(20), bu(2_432_902_008_176_640_000u64));
}

#[test]
fn factorial_cross_validate_naive() {
    // Cross-validate factorial() against a simple iterative product.
    for n in 0u64..=200 {
        let via_fn = factorial(n);
        let naive: BigUint = (1..=n).fold(BigUint::one(), |acc, k| acc * BigUint::from(k));
        assert_eq!(via_fn, naive, "factorial({}) mismatch", n);
    }
}

#[test]
fn factorial_100_has_158_digits() {
    let f100 = factorial(100);
    let decimal = f100.to_string();
    assert_eq!(
        decimal.len(),
        158,
        "100! should have 158 decimal digits, got {}",
        decimal.len()
    );
}

// ---------------------------------------------------------------------------
// Lucas U/V sequence tests
// ---------------------------------------------------------------------------

#[test]
fn lucas_uv_fibonacci_u_sequence() {
    // P=1, Q=-1 gives U_n = Fibonacci(n).
    // Fib: 0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55
    let expected_u = [0u64, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
    let m = bu(1_000_000_007);
    for (n, &eu) in expected_u.iter().enumerate() {
        let (u, _v) = lucas_uv(&bu(n as u64), 1, -1, &m).expect("lucas U");
        assert_eq!(u, bu(eu), "U_{} should be Fib({})", n, n);
    }
}

#[test]
fn lucas_uv_v_sequence() {
    // P=1, Q=-1 gives V_n = Lucas numbers: 2, 1, 3, 4, 7, 11, 18, 29, 47, 76
    let expected_v = [2u64, 1, 3, 4, 7, 11, 18, 29, 47, 76];
    let m = bu(1_000_000_007);
    for (n, &ev) in expected_v.iter().enumerate() {
        let (_u, v) = lucas_uv(&bu(n as u64), 1, -1, &m).expect("lucas V");
        assert_eq!(v, bu(ev), "V_{} should be LucasNum({})", n, n);
    }
}

#[test]
fn lucas_uv_modular_reduction() {
    // Fib(12) = 144; 144 mod 101 = 43.
    let m = bu(101); // prime, odd
    let (u, _) = lucas_uv(&bu(12), 1, -1, &m).expect("lucas mod");
    assert_eq!(u, bu(144 % 101));
}

#[test]
fn lucas_uv_larger_index() {
    // Fib(50) = 12586269025 — verify via two large-prime moduli.
    let fib50: u64 = 12_586_269_025;
    // mod a large prime
    let m = bu(1_000_000_007);
    let (u, _) = lucas_uv(&bu(50), 1, -1, &m).expect("lucas Fib50");
    assert_eq!(u, bu(fib50 % 1_000_000_007));
}

#[test]
fn lucas_uv_p2_qneg1_pell() {
    // P=2, Q=-1: U_n gives the Pell numbers: 0, 1, 2, 5, 12, 29, 70, ...
    // Recurrence: U_{n+2} = P*U_{n+1} - Q*U_n = 2*U_{n+1} + 1*U_n.
    let expected_pell = [0u64, 1, 2, 5, 12, 29, 70];
    let m = bu(1_000_000_007);
    for (n, &ep) in expected_pell.iter().enumerate() {
        let (u, _) = lucas_uv(&bu(n as u64), 2, -1, &m).expect("Pell U");
        assert_eq!(u, bu(ep), "Pell U_{} mismatch", n);
    }
}

#[test]
fn lucas_uv_rejects_even_modulus() {
    let m = bu(100); // even
    assert!(lucas_uv(&bu(5), 1, -1, &m).is_err());
}

#[test]
fn lucas_uv_rejects_zero_modulus() {
    assert!(lucas_uv(&bu(5), 1, -1, &BigUint::zero()).is_err());
}
