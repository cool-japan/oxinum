//! Integration tests for modular arithmetic: extended GCD, mod_inv, mod_mul,
//! mod_pow, and MontgomeryContext.

use oxinum_int::native::{
    divrem, gcd_extended, mod_inv, mod_mul, mod_pow, BigInt, BigUint, MontgomeryContext,
};

fn bu(n: u64) -> BigUint {
    BigUint::from_u64(n)
}

// ---------------------------------------------------------------------------
// Extended GCD
// ---------------------------------------------------------------------------

#[test]
fn ext_gcd_simple() {
    // gcd(12, 8) = 4, should satisfy 12*x + 8*y == 4
    let (g, x, y) = gcd_extended(&bu(12), &bu(8));
    assert_eq!(g, bu(4));
    let sum = BigInt::from(12i64) * x + BigInt::from(8i64) * y;
    assert_eq!(sum, BigInt::from(4i64));
}

#[test]
fn ext_gcd_coprime() {
    let (g, x, y) = gcd_extended(&bu(35), &bu(15));
    assert_eq!(g, bu(5));
    let sum = BigInt::from(35i64) * x + BigInt::from(15i64) * y;
    assert_eq!(sum, BigInt::from(5i64));
}

#[test]
fn ext_gcd_both_zero() {
    let (g, _x, _y) = gcd_extended(&bu(0), &bu(0));
    // gcd(0, 0) == 0 by convention.
    assert_eq!(g, bu(0));
}

#[test]
fn ext_gcd_a_zero() {
    let (g, x, y) = gcd_extended(&bu(0), &bu(9));
    assert_eq!(g, bu(9));
    // 0 * x + 9 * y == 9
    let sum = BigInt::from(0i64) * x + BigInt::from(9i64) * y;
    assert_eq!(sum, BigInt::from(9i64));
}

#[test]
fn ext_gcd_b_zero() {
    let (g, x, y) = gcd_extended(&bu(7), &bu(0));
    assert_eq!(g, bu(7));
    let sum = BigInt::from(7i64) * x + BigInt::from(0i64) * y;
    assert_eq!(sum, BigInt::from(7i64));
}

/// Verify Bezout identity a*x + b*y = gcd(a,b) for 200 pseudo-random pairs.
#[test]
fn ext_gcd_bezout_200_random() {
    // LCG constants for deterministic pseudo-random generation.
    let mut a_val: u64 = 12345;
    let mut b_val: u64 = 67890;

    for _i in 0..200 {
        // Advance LCG state.
        a_val = a_val
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        b_val = b_val
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);

        let a = bu(a_val % (1u64 << 32));
        let b = bu(b_val % (1u64 << 32));

        let (g, x, y) = gcd_extended(&a, &b);

        // Bezout identity: a * x + b * y == g
        let sum = BigInt::from(a.clone()) * x + BigInt::from(b.clone()) * y;
        assert_eq!(
            sum,
            BigInt::from(g.clone()),
            "Bezout failed for a_val={}, b_val={}",
            a_val,
            b_val
        );

        // g must divide a (if a != 0)
        if !a.is_zero() {
            let (_, rem) = divrem(&a, &g);
            assert!(rem.is_zero(), "g does not divide a for a_val={}", a_val);
        }

        // g must divide b (if b != 0)
        if !b.is_zero() {
            let (_, rem) = divrem(&b, &g);
            assert!(rem.is_zero(), "g does not divide b for b_val={}", b_val);
        }
    }
}

/// Verify that gcd_extended gives the same GCD as the existing native::gcd.
#[test]
fn ext_gcd_matches_native_gcd() {
    use oxinum_int::native::gcd;

    let pairs = [
        (0u64, 0u64),
        (0, 7),
        (7, 0),
        (48, 18),
        (100, 75),
        (13, 17),
        (1000000007, 998244353),
        (u32::MAX as u64, u32::MAX as u64 - 1),
    ];
    for (a_val, b_val) in pairs {
        let (g_ext, _x, _y) = gcd_extended(&bu(a_val), &bu(b_val));
        let g_ref = gcd(bu(a_val), bu(b_val));
        assert_eq!(g_ext, g_ref, "GCD mismatch for a={a_val}, b={b_val}");
    }
}

// ---------------------------------------------------------------------------
// mod_inv
// ---------------------------------------------------------------------------

#[test]
fn mod_inv_basic() {
    // 3^{-1} mod 7 = 5 (since 3 * 5 = 15 ≡ 1 mod 7)
    assert_eq!(mod_inv(&bu(3), &bu(7)), Some(bu(5)));
}

#[test]
fn mod_inv_no_inverse_gcd_ne_1() {
    // gcd(6, 9) = 3 ≠ 1 → no inverse
    assert_eq!(mod_inv(&bu(6), &bu(9)), None);
}

#[test]
fn mod_inv_zero_modulus() {
    assert_eq!(mod_inv(&bu(3), &bu(0)), None);
}

#[test]
fn mod_inv_zero_a() {
    // gcd(0, m) = m; m > 1 so no inverse
    assert_eq!(mod_inv(&bu(0), &bu(7)), None);
}

#[test]
fn mod_inv_result_in_range() {
    for &m in &[7u64, 13, 101, 65537] {
        for a in 1u64..m.min(20) {
            if let Some(inv) = mod_inv(&bu(a), &bu(m)) {
                assert!(inv < bu(m), "mod_inv result >= m for a={a}, m={m}");
            }
        }
    }
}

/// Verify the round-trip: (a * mod_inv(a, m)) % m == 1 for coprime a and m.
#[test]
fn mod_inv_roundtrip_200() {
    let primes = [7u64, 13, 101, 65537, 4_294_967_311]; // primes

    for &p in &primes {
        for a in 1u64..p.min(20) {
            if let Some(inv) = mod_inv(&bu(a), &bu(p)) {
                let product_big = bu(a) * inv;
                let (_q, rem) = divrem(&product_big, &bu(p));
                assert_eq!(rem, bu(1), "mod_inv roundtrip failed for a={a}, p={p}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// mod_mul
// ---------------------------------------------------------------------------

#[test]
fn mod_mul_basic() {
    // 7 * 8 = 56; 56 mod 5 = 1
    assert_eq!(mod_mul(&bu(7), &bu(8), &bu(5)).expect("mod_mul"), bu(1));
}

#[test]
fn mod_mul_zero_modulus() {
    assert!(mod_mul(&bu(3), &bu(4), &bu(0)).is_err());
}

#[test]
fn mod_mul_result_in_range() {
    let m = bu(17);
    for a in 0u64..17 {
        for b in 0u64..17 {
            let r = mod_mul(&bu(a), &bu(b), &m).expect("mod_mul");
            assert!(r < m, "mod_mul result >= m for a={a}, b={b}");
        }
    }
}

// ---------------------------------------------------------------------------
// mod_pow
// ---------------------------------------------------------------------------

#[test]
fn mod_pow_basic() {
    // 2^10 mod 1000 = 1024 mod 1000 = 24
    let result = mod_pow(&bu(2), &bu(10), &bu(1000)).expect("mod_pow");
    assert_eq!(result, bu(24));
}

#[test]
fn mod_pow_zero_exp() {
    // a^0 mod m = 1 for any a (including 0)
    assert_eq!(mod_pow(&bu(0), &bu(0), &bu(7)).expect(""), bu(1));
    assert_eq!(mod_pow(&bu(5), &bu(0), &bu(7)).expect(""), bu(1));
}

#[test]
fn mod_pow_zero_modulus() {
    assert!(mod_pow(&bu(2), &bu(10), &bu(0)).is_err());
}

#[test]
fn mod_pow_modulus_one() {
    // All integers are 0 mod 1.
    assert_eq!(mod_pow(&bu(999), &bu(999), &bu(1)).expect(""), bu(0));
}

/// Fermat's little theorem: a^(p-1) ≡ 1 (mod p) for prime p and gcd(a,p)=1.
#[test]
fn mod_pow_fermat_little_theorem() {
    for &p in &[7u64, 13, 101, 65537] {
        for a in 2u64..p.min(10) {
            let result = mod_pow(&bu(a), &bu(p - 1), &bu(p)).expect("Fermat mod_pow");
            assert_eq!(result, bu(1), "Fermat failed for a={a}, p={p}");
        }
    }
}

// ---------------------------------------------------------------------------
// MontgomeryContext
// ---------------------------------------------------------------------------

#[test]
fn montgomery_basic_mul() {
    // 3 * 4 mod 7 = 12 mod 7 = 5
    let ctx = MontgomeryContext::new(bu(7)).expect("Montgomery ctx");
    let a = ctx.to_mont(&bu(3));
    let b = ctx.to_mont(&bu(4));
    let c_mont = ctx.mul(&a, &b);
    let c = ctx.from_mont(&c_mont);
    assert_eq!(c, bu(5));
}

#[test]
fn montgomery_pow_2_10_mod13() {
    // 2^10 = 1024 = 78*13 + 10
    let ctx = MontgomeryContext::new(bu(13)).expect("Montgomery ctx");
    let result = ctx.pow(&bu(2), &bu(10));
    assert_eq!(result, bu(10));
}

#[test]
fn montgomery_rejects_even() {
    assert!(MontgomeryContext::new(bu(10)).is_err());
    assert!(MontgomeryContext::new(bu(2)).is_err());
}

#[test]
fn montgomery_rejects_zero_one() {
    assert!(MontgomeryContext::new(bu(0)).is_err());
    assert!(MontgomeryContext::new(bu(1)).is_err());
}

#[test]
fn montgomery_roundtrip_to_from() {
    let ctx = MontgomeryContext::new(bu(7)).expect("ctx");
    for a in 0u64..7 {
        let a_mont = ctx.to_mont(&bu(a));
        let a_back = ctx.from_mont(&a_mont);
        assert_eq!(a_back, bu(a), "roundtrip failed for a={a}");
    }
}

/// Cross-validate Montgomery mul against schoolbook mod_mul over many cases.
#[test]
fn montgomery_vs_schoolbook_100() {
    let odd_moduli = [7u64, 13, 101, 4093, 65537, 2_147_483_647]; // primes and Mersenne
    for &m in &odd_moduli {
        let ctx = MontgomeryContext::new(bu(m)).expect("ctx");
        let test_vals = [0u64, 1, 2, 5, m / 2, m / 2 + 1, m - 2, m - 1];
        for &a in &test_vals {
            for &b in &test_vals {
                let a = a.min(m - 1);
                let b = b.min(m - 1);
                let expected = mod_mul(&bu(a), &bu(b), &bu(m)).expect("schoolbook");
                let a_mont = ctx.to_mont(&bu(a));
                let b_mont = ctx.to_mont(&bu(b));
                let got_mont = ctx.mul(&a_mont, &b_mont);
                let got = ctx.from_mont(&got_mont);
                assert_eq!(
                    got, expected,
                    "Montgomery vs schoolbook mismatch: a={a}, b={b}, m={m}"
                );
            }
        }
    }
}

/// Cross-validate Montgomery pow against schoolbook mod_pow.
#[test]
fn montgomery_pow_vs_mod_pow() {
    let odd_moduli = [7u64, 13, 101, 65537];
    let exps = [0u64, 1, 2, 5, 10, 100];
    for &m in &odd_moduli {
        let ctx = MontgomeryContext::new(bu(m)).expect("ctx");
        for &a in &[1u64, 2, 3, m - 1] {
            for &e in &exps {
                let expected = mod_pow(&bu(a), &bu(e), &bu(m)).expect("mod_pow");
                let got = ctx.pow(&bu(a), &bu(e));
                assert_eq!(got, expected, "Montgomery pow: a={a}, e={e}, m={m}");
            }
        }
    }
}

/// Fermat's little theorem via Montgomery exponentiation.
#[test]
fn montgomery_fermat() {
    for &p in &[7u64, 13, 101, 65537] {
        let ctx = MontgomeryContext::new(bu(p)).expect("ctx");
        for a in 2u64..p.min(8) {
            let result = ctx.pow(&bu(a), &bu(p - 1));
            assert_eq!(result, bu(1), "Fermat (Montgomery) failed for a={a}, p={p}");
        }
    }
}
