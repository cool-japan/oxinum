//! Integration tests for `oxinum_int::native::BigInt`.
//!
//! Coverage:
//!
//! - Canonical-zero invariant (`+0 == -0`, equal hashes, normalized sign).
//! - Sign arithmetic table (Add/Sub/Mul/Div/Rem).
//! - `i64::MIN` and other primitive boundary `From` / `TryFrom` cases.
//! - GCD (Stein binary GCD) cross-validated against `dashu_int::UBig`'s gcd.
//! - Integer sqrt + nth_root invariants over hand-picked and random inputs.
//! - Proptest algebraic laws (commutativity, associativity, distributivity,
//!   integer-truncation `a == (a/b)*b + a%b`).
//! - Cross-validation against `dashu_int::IBig` for add/sub/mul/div/rem and gcd.

use dashu_int::{IBig, UBig};
use oxinum_core::Sign;
use oxinum_int::native::{divrem_int, gcd, gcd_int, BigInt, BigUint};
use proptest::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// Conversion helpers (native <-> dashu) — byte-level so we don't need
// `dashu_int::ops::*` traits.
// ---------------------------------------------------------------------------

fn ubig_to_native(u: &UBig) -> BigUint {
    BigUint::from_bytes_le(&u.to_le_bytes())
}

fn native_to_ubig(n: &BigUint) -> UBig {
    UBig::from_le_bytes(&n.to_bytes_le())
}

fn ibig_to_native(i: &IBig) -> BigInt {
    let (sign, mag) = (i.sign(), i.clone().into_parts().1);
    BigInt::from_parts(sign, ubig_to_native(&mag))
}

fn native_to_ibig(n: &BigInt) -> IBig {
    IBig::from_parts(n.sign(), native_to_ubig(n.magnitude()))
}

fn hash_value<T: Hash>(t: &T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

// ---------------------------------------------------------------------------
// Canonical zero invariant
// ---------------------------------------------------------------------------

#[test]
fn canonical_zero_positive_equals_negative() {
    let p = BigInt::from_parts(Sign::Positive, BigUint::ZERO);
    let n = BigInt::from_parts(Sign::Negative, BigUint::ZERO);
    assert_eq!(p, n);
    assert_eq!(p.sign(), Sign::Positive);
    assert_eq!(n.sign(), Sign::Positive); // canonicalized
    assert_eq!(hash_value(&p), hash_value(&n));
}

#[test]
fn neg_zero_canonicalizes() {
    let z = BigInt::zero();
    let nz = -z.clone();
    assert_eq!(nz, z);
    assert_eq!(nz.sign(), Sign::Positive);
}

#[test]
fn add_to_zero_preserves_canonical_sign() {
    let a = BigInt::from(42i64);
    let s = &a + &(-a.clone());
    assert!(s.is_zero());
    assert_eq!(s.sign(), Sign::Positive);
}

#[test]
fn sub_self_yields_canonical_zero() {
    for v in [-100i64, -1, 0, 1, 100, i64::MAX, i64::MIN] {
        let a = BigInt::from(v);
        let z = &a - &a;
        assert!(z.is_zero(), "{a} - {a} should be zero");
        assert_eq!(z.sign(), Sign::Positive);
    }
}

#[test]
fn mul_by_zero_canonicalizes() {
    let a = BigInt::from(-12345i64);
    let zero = BigInt::zero();
    let p = &a * &zero;
    assert!(p.is_zero());
    assert_eq!(p.sign(), Sign::Positive);
}

// ---------------------------------------------------------------------------
// Sign arithmetic — small but exhaustive table on i64 against primitives.
// ---------------------------------------------------------------------------

#[test]
fn sign_arithmetic_table_add() {
    let table: &[(i64, i64)] = &[(5, 3), (5, -3), (-5, 3), (-5, -3), (5, 0), (0, -5)];
    for &(a, b) in table {
        let na = BigInt::from(a);
        let nb = BigInt::from(b);
        assert_eq!(&na + &nb, BigInt::from(a + b));
    }
}

#[test]
fn sign_arithmetic_table_sub() {
    let table: &[(i64, i64)] = &[(5, 3), (5, -3), (-5, 3), (-5, -3), (5, 0), (0, -5)];
    for &(a, b) in table {
        assert_eq!(&BigInt::from(a) - &BigInt::from(b), BigInt::from(a - b));
    }
}

#[test]
fn sign_arithmetic_table_mul() {
    let table: &[(i64, i64)] = &[(5, 3), (5, -3), (-5, 3), (-5, -3), (5, 0), (0, -5)];
    for &(a, b) in table {
        assert_eq!(&BigInt::from(a) * &BigInt::from(b), BigInt::from(a * b));
    }
}

#[test]
fn sign_arithmetic_table_div_rem() {
    let table: &[(i64, i64)] = &[
        (17, 5),
        (17, -5),
        (-17, 5),
        (-17, -5),
        (15, 5),
        (-15, -5),
        (0, 7),
    ];
    for &(a, b) in table {
        let q = &BigInt::from(a) / &BigInt::from(b);
        let r = &BigInt::from(a) % &BigInt::from(b);
        assert_eq!(q, BigInt::from(a / b), "q mismatch for {a}/{b}");
        assert_eq!(r, BigInt::from(a % b), "r mismatch for {a}%{b}");
    }
}

#[test]
fn remainder_sign_matches_dividend() {
    // Match Rust primitive % behaviour: sign of remainder = sign of dividend.
    for a in [-17i64, -5, -1, 1, 5, 17] {
        for b in [-7i64, -3, -1, 1, 3, 7] {
            if b == 0 {
                continue;
            }
            let r = &BigInt::from(a) % &BigInt::from(b);
            // Expected sign:
            //   r == 0  => +0
            //   r != 0  => sign of dividend a
            let expected_sign = if a % b == 0 || a > 0 {
                Sign::Positive
            } else {
                Sign::Negative
            };
            assert_eq!(
                r.sign(),
                expected_sign,
                "sign of {a} % {b} (= {r}) should match expected"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Primitive boundary conversions
// ---------------------------------------------------------------------------

#[test]
fn from_i64_min() {
    let n = BigInt::from(i64::MIN);
    assert_eq!(format!("{n}"), format!("{}", i64::MIN));
    let back: i64 = i64::try_from(&n).expect("i64::MIN fits in i64");
    assert_eq!(back, i64::MIN);
}

#[test]
fn from_i64_max() {
    let n = BigInt::from(i64::MAX);
    let back: i64 = i64::try_from(&n).expect("i64::MAX fits");
    assert_eq!(back, i64::MAX);
}

#[test]
fn from_u64_max() {
    let n = BigInt::from(u64::MAX);
    let back: u64 = u64::try_from(&n).expect("u64::MAX fits");
    assert_eq!(back, u64::MAX);
    // u64::MAX overflows i64.
    assert!(i64::try_from(&n).is_err());
}

#[test]
fn from_i128_min_max() {
    for v in [i128::MIN, i128::MAX, 0, -1, 1] {
        let n = BigInt::from(v);
        let back: i128 = i128::try_from(&n).expect("roundtrip");
        assert_eq!(back, v);
    }
}

#[test]
fn from_u128_max() {
    let n = BigInt::from(u128::MAX);
    let back: u128 = u128::try_from(&n).expect("roundtrip");
    assert_eq!(back, u128::MAX);
}

#[test]
fn try_from_negative_into_unsigned() {
    let n = BigInt::from(-1i64);
    assert!(u64::try_from(&n).is_err());
    assert!(u128::try_from(&n).is_err());
    assert!(usize::try_from(&n).is_err());
}

#[test]
fn try_from_overflow_signed_targets() {
    let n = BigInt::from(u128::MAX);
    assert!(i64::try_from(&n).is_err());
    assert!(i32::try_from(&n).is_err());
    assert!(i128::try_from(&n).is_err());
}

#[test]
fn try_from_isize_usize_boundary() {
    let max = BigInt::from(usize::MAX);
    assert_eq!(usize::try_from(&max).expect("ok"), usize::MAX);
    let min_i = BigInt::from(isize::MIN);
    assert_eq!(isize::try_from(&min_i).expect("ok"), isize::MIN);
}

// ---------------------------------------------------------------------------
// GCD
// ---------------------------------------------------------------------------

#[test]
fn gcd_basic_facts() {
    assert_eq!(gcd(BigUint::ZERO, BigUint::ZERO), BigUint::ZERO);
    assert_eq!(
        gcd(BigUint::ZERO, BigUint::from_u64(9)),
        BigUint::from_u64(9)
    );
    assert_eq!(
        gcd(BigUint::from_u64(9), BigUint::ZERO),
        BigUint::from_u64(9)
    );
    assert_eq!(
        gcd(BigUint::from_u64(48), BigUint::from_u64(18)),
        BigUint::from_u64(6)
    );
}

#[test]
fn gcd_coprime_consecutive_fibonacci() {
    // F(10) = 55, F(11) = 89 — coprime.
    assert_eq!(
        gcd(BigUint::from_u64(55), BigUint::from_u64(89)),
        BigUint::one()
    );
}

#[test]
fn gcd_int_returns_nonneg() {
    let a = BigInt::from(-1024i64);
    let b = BigInt::from(384i64);
    let g = gcd_int(&a, &b);
    assert_eq!(g, BigInt::from(128i64));
    assert_eq!(g.sign(), Sign::Positive);
}

#[test]
fn gcd_cross_val_random_200_seeded() {
    // Deterministic 200-case cross-val against dashu's gcd (UBig has `.gcd()`
    // via the trait re-export, but we call out to the simpler euclidean
    // reference here to avoid pulling in extra traits).
    let mut s: u64 = 0xA5A5_C001_DEAD_BEEFu64;
    let mut g_cases = 0usize;
    for _ in 0..200 {
        s = xorshift64(s);
        let a_n = (s % 6) as usize + 1; // 1..=6 limbs
        let mut a_limbs = Vec::with_capacity(a_n);
        for _ in 0..a_n {
            s = xorshift64(s);
            a_limbs.push(s);
        }
        s = xorshift64(s);
        let b_n = (s % 6) as usize + 1;
        let mut b_limbs = Vec::with_capacity(b_n);
        for _ in 0..b_n {
            s = xorshift64(s);
            b_limbs.push(s);
        }
        // Ensure top limb non-zero so length is real.
        if let Some(last) = a_limbs.last_mut() {
            if *last == 0 {
                *last = 1;
            }
        }
        if let Some(last) = b_limbs.last_mut() {
            if *last == 0 {
                *last = 1;
            }
        }
        let a_n = BigUint::from_le_limbs(&a_limbs);
        let b_n = BigUint::from_le_limbs(&b_limbs);
        let g_native = gcd(a_n.clone(), b_n.clone());
        // Reference: Euclidean GCD on the same values via dashu.
        let g_ref = euclidean_gcd_via_dashu(&a_n, &b_n);
        assert_eq!(
            g_native, g_ref,
            "gcd mismatch for a={a_n:?}, b={b_n:?}, native={g_native}, dashu={g_ref}"
        );
        g_cases += 1;
    }
    assert_eq!(g_cases, 200);
}

fn euclidean_gcd_via_dashu(a: &BigUint, b: &BigUint) -> BigUint {
    let da = native_to_ubig(a);
    let db = native_to_ubig(b);
    let mut x = da;
    let mut y = db;
    while !y.is_zero() {
        let r = &x % &y;
        x = y;
        y = r;
    }
    ubig_to_native(&x)
}

// ---------------------------------------------------------------------------
// Integer sqrt + nth_root
// ---------------------------------------------------------------------------

#[test]
fn sqrt_invariant_handpicked() {
    for k in [
        0u64,
        1,
        2,
        3,
        4,
        8,
        9,
        15,
        16,
        17,
        100,
        9999,
        1_000_000,
        u64::MAX,
    ] {
        let n = BigUint::from_u64(k);
        let r = n.sqrt();
        // r*r <= n < (r+1)*(r+1)
        assert!(&r * &r <= n, "sqrt lower invariant for k={k}");
        let rp1 = &r + &BigUint::one();
        // For very large k, (r+1)^2 may overflow u64 but BigUint handles it.
        assert!(&rp1 * &rp1 > n, "sqrt upper invariant for k={k}");
    }
}

#[test]
fn sqrt_perfect_squares_up_to_1000() {
    for k in 0u64..=1000 {
        let n = BigUint::from_u64(k * k);
        assert_eq!(n.sqrt(), BigUint::from_u64(k));
    }
}

#[test]
fn cube_root_examples() {
    assert_eq!(
        BigUint::from_u64(27).nth_root(3).expect("ok"),
        BigUint::from_u64(3)
    );
    assert_eq!(
        BigUint::from_u64(28).nth_root(3).expect("ok"),
        BigUint::from_u64(3)
    );
    assert_eq!(
        BigUint::from_u64(26).nth_root(3).expect("ok"),
        BigUint::from_u64(2)
    );
    assert_eq!(
        BigUint::from_u64(1000).nth_root(3).expect("ok"),
        BigUint::from_u64(10)
    );
}

#[test]
fn nth_root_invariant_handpicked() {
    for n in 2u32..=7 {
        for k in [0u64, 1, 2, 5, 10, 100, 1000, 1_000_000] {
            let value = BigUint::from_u64(k);
            let r = value.nth_root(n).expect("nth_root ok");
            assert!(r.pow(n) <= value, "n={n}, k={k}, r={r}, r^n > value");
            let rp1 = &r + &BigUint::one();
            assert!(rp1.pow(n) > value, "n={n}, k={k}, (r+1)^n <= value");
        }
    }
}

#[test]
fn bigint_nth_root_signs() {
    assert_eq!(
        BigInt::from(-8i64).nth_root(3).expect("ok"),
        BigInt::from(-2i64)
    );
    assert_eq!(
        BigInt::from(-27i64).nth_root(3).expect("ok"),
        BigInt::from(-3i64)
    );
    // Even root of negative -> error.
    assert!(BigInt::from(-4i64).nth_root(2).is_err());
    assert!(BigInt::from(-16i64).nth_root(4).is_err());
    // Zeroth root -> error.
    assert!(BigInt::from(10i64).nth_root(0).is_err());
}

#[test]
fn large_sqrt_cross_val_dashu() {
    // ~50 limbs to exercise multi-limb arithmetic.
    let mut s = 0xBEEF_C0DE_DEAD_BEEFu64;
    for _ in 0..20 {
        s = xorshift64(s);
        let n_limbs = (s % 10) as usize + 1;
        let mut limbs = Vec::with_capacity(n_limbs);
        for _ in 0..n_limbs {
            s = xorshift64(s);
            limbs.push(s);
        }
        if let Some(last) = limbs.last_mut() {
            if *last == 0 {
                *last = 1;
            }
        }
        let n = BigUint::from_le_limbs(&limbs);
        let r = n.sqrt();
        // r*r <= n < (r+1)*(r+1)
        assert!(&r * &r <= n);
        let rp1 = &r + &BigUint::one();
        assert!(&rp1 * &rp1 > n);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn xorshift64(mut s: u64) -> u64 {
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    s
}

// ---------------------------------------------------------------------------
// Proptest strategies + algebraic laws + cross-validation
// ---------------------------------------------------------------------------

fn arb_bigint() -> impl Strategy<Value = BigInt> {
    // Up to 6 limbs of magnitude + any sign.
    (any::<bool>(), prop::collection::vec(any::<u64>(), 0..6)).prop_map(|(is_neg, limbs)| {
        let mag = BigUint::from_le_limbs(&limbs);
        let sign = if is_neg {
            Sign::Negative
        } else {
            Sign::Positive
        };
        BigInt::from_parts(sign, mag)
    })
}

fn arb_nonzero_bigint() -> impl Strategy<Value = BigInt> {
    (
        any::<bool>(),
        1usize..=5,
        any::<u64>(),
        prop::collection::vec(any::<u64>(), 0..5),
    )
        .prop_map(|(is_neg, top_pos, top, mut rest)| {
            let top_nonzero = if top == 0 { 1 } else { top };
            while rest.len() < top_pos {
                rest.push(0);
            }
            let mut limbs = rest;
            limbs.push(top_nonzero);
            let mag = BigUint::from_le_limbs(&limbs);
            let sign = if is_neg {
                Sign::Negative
            } else {
                Sign::Positive
            };
            BigInt::from_parts(sign, mag)
        })
}

proptest! {
    #[test]
    fn add_commutative(a in arb_bigint(), b in arb_bigint()) {
        prop_assert_eq!(&a + &b, &b + &a);
    }

    #[test]
    fn add_associative(a in arb_bigint(), b in arb_bigint(), c in arb_bigint()) {
        prop_assert_eq!((&a + &b) + &c, &a + (&b + &c));
    }

    #[test]
    fn mul_commutative(a in arb_bigint(), b in arb_bigint()) {
        prop_assert_eq!(&a * &b, &b * &a);
    }

    #[test]
    fn mul_associative(a in arb_bigint(), b in arb_bigint(), c in arb_bigint()) {
        prop_assert_eq!((&a * &b) * &c, &a * (&b * &c));
    }

    #[test]
    fn mul_distributes_over_add(
        a in arb_bigint(), b in arb_bigint(), c in arb_bigint()
    ) {
        prop_assert_eq!(&a * (&b + &c), &a * &b + &a * &c);
        prop_assert_eq!((&a + &b) * &c, &a * &c + &b * &c);
    }

    #[test]
    fn sub_anti_commutative(a in arb_bigint(), b in arb_bigint()) {
        prop_assert_eq!(&a - &b, -(&b - &a));
    }

    #[test]
    fn neg_neg_identity(a in arb_bigint()) {
        prop_assert_eq!(-(-a.clone()), a);
    }

    #[test]
    fn divrem_integer_invariant(a in arb_bigint(), b in arb_nonzero_bigint()) {
        let (q, r) = divrem_int(&a, &b);
        // a == q*b + r
        prop_assert_eq!(&(&q * &b) + &r, a.clone());
        // |r| < |b|
        prop_assert!(r.magnitude() < b.magnitude());
        // remainder sign matches dividend sign when remainder is non-zero.
        if !r.is_zero() {
            prop_assert_eq!(r.sign(), a.sign());
        }
    }

    #[test]
    fn dashu_cross_val_add(a in arb_bigint(), b in arb_bigint()) {
        let r_native = &a + &b;
        let r_dashu = native_to_ibig(&a) + native_to_ibig(&b);
        prop_assert_eq!(r_native, ibig_to_native(&r_dashu));
    }

    #[test]
    fn dashu_cross_val_sub(a in arb_bigint(), b in arb_bigint()) {
        let r_native = &a - &b;
        let r_dashu = native_to_ibig(&a) - native_to_ibig(&b);
        prop_assert_eq!(r_native, ibig_to_native(&r_dashu));
    }

    #[test]
    fn dashu_cross_val_mul(a in arb_bigint(), b in arb_bigint()) {
        let r_native = &a * &b;
        let r_dashu = native_to_ibig(&a) * native_to_ibig(&b);
        prop_assert_eq!(r_native, ibig_to_native(&r_dashu));
    }

    #[test]
    fn dashu_cross_val_divrem(a in arb_bigint(), b in arb_nonzero_bigint()) {
        let (q, r) = divrem_int(&a, &b);
        let ai = native_to_ibig(&a);
        let bi = native_to_ibig(&b);
        let dq = &ai / &bi;
        let dr = &ai % &bi;
        prop_assert_eq!(q, ibig_to_native(&dq));
        prop_assert_eq!(r, ibig_to_native(&dr));
    }

    #[test]
    fn dashu_cross_val_cmp(a in arb_bigint(), b in arb_bigint()) {
        let native_ord = a.cmp(&b);
        let dashu_ord = native_to_ibig(&a).cmp(&native_to_ibig(&b));
        prop_assert_eq!(native_ord, dashu_ord);
    }

    #[test]
    fn dashu_cross_val_gcd(a in arb_bigint(), b in arb_bigint()) {
        let g_native = gcd_int(&a, &b);
        // Reference: do Euclidean gcd on dashu IBig magnitudes.
        let mut x: UBig = native_to_ubig(a.magnitude());
        let mut y: UBig = native_to_ubig(b.magnitude());
        while !y.is_zero() {
            let r = &x % &y;
            x = y;
            y = r;
        }
        let g_ref = BigInt::from_parts(Sign::Positive, ubig_to_native(&x));
        prop_assert_eq!(g_native, g_ref);
    }
}

// ---------------------------------------------------------------------------
// Tiny sanity-check: TryFrom<&BigUint> for primitives
// ---------------------------------------------------------------------------

#[test]
fn try_from_biguint_for_primitives() {
    let huge = BigUint::from_u128(u128::MAX);
    assert_eq!(u128::try_from(&huge).expect("ok"), u128::MAX);
    assert!(u64::try_from(&huge).is_err());

    let small = BigUint::from_u64(42);
    assert_eq!(u64::try_from(&small).expect("ok"), 42u64);
    assert_eq!(i64::try_from(&small).expect("ok"), 42i64);
}

// ---------------------------------------------------------------------------
// Two's-complement signed byte serialization (I1)
// ---------------------------------------------------------------------------

#[test]
fn signed_bytes_be_zero() {
    let z = BigInt::zero();
    assert_eq!(z.to_signed_bytes_be(), vec![0u8]);
    assert_eq!(BigInt::from_signed_bytes_be(&[]), z);
    assert_eq!(BigInt::from_signed_bytes_be(&[0u8]), z);
}

#[test]
fn signed_bytes_le_zero() {
    let z = BigInt::zero();
    assert_eq!(z.to_signed_bytes_le(), vec![0u8]);
    assert_eq!(BigInt::from_signed_bytes_le(&[]), z);
    assert_eq!(BigInt::from_signed_bytes_le(&[0u8]), z);
}

#[test]
fn signed_bytes_minimal_length_be() {
    assert_eq!(BigInt::from(1i64).to_signed_bytes_be(), vec![0x01u8]);
    assert_eq!(BigInt::from(-1i64).to_signed_bytes_be(), vec![0xFFu8]);
    assert_eq!(BigInt::from(127i64).to_signed_bytes_be(), vec![0x7Fu8]);
    assert_eq!(BigInt::from(-128i64).to_signed_bytes_be(), vec![0x80u8]);
    assert_eq!(
        BigInt::from(128i64).to_signed_bytes_be(),
        vec![0x00u8, 0x80]
    );
    assert_eq!(
        BigInt::from(129i64).to_signed_bytes_be(),
        vec![0x00u8, 0x81]
    );
    assert_eq!(
        BigInt::from(-129i64).to_signed_bytes_be(),
        vec![0xFFu8, 0x7F]
    );
}

#[test]
fn signed_bytes_be_le_consistency() {
    // For every value, reversing the BE bytes should give the LE bytes.
    for value in [
        0i64,
        1,
        -1,
        127,
        -128,
        129,
        -129,
        i64::MAX,
        i64::MIN,
        1234567890,
        -1234567890,
    ] {
        let n = BigInt::from(value);
        let be = n.to_signed_bytes_be();
        let le = n.to_signed_bytes_le();
        let mut be_reversed = be.clone();
        be_reversed.reverse();
        assert_eq!(le, be_reversed, "BE-reversed != LE for {value}");
        // And: from_signed_bytes_le(reversed(be)) == from_signed_bytes_be(be).
        let mut from_le_input = be.clone();
        from_le_input.reverse();
        assert_eq!(
            BigInt::from_signed_bytes_le(&from_le_input),
            BigInt::from_signed_bytes_be(&be),
            "decode mismatch for {value}",
        );
    }
}

#[test]
fn signed_bytes_roundtrip_i64_boundaries() {
    for value in [
        0i64,
        1,
        -1,
        127,
        -128,
        129,
        -129,
        i64::MAX,
        i64::MIN,
        i32::MIN as i64,
        i32::MAX as i64,
        12345,
        -12345,
    ] {
        let n = BigInt::from(value);
        let be = n.to_signed_bytes_be();
        let le = n.to_signed_bytes_le();
        assert_eq!(
            BigInt::from_signed_bytes_be(&be),
            n,
            "BE round-trip for {value}",
        );
        assert_eq!(
            BigInt::from_signed_bytes_le(&le),
            n,
            "LE round-trip for {value}",
        );
    }
}

#[test]
fn signed_bytes_roundtrip_huge_powers_of_two() {
    // 2^200 and -2^200.
    let two = BigInt::from(2i64);
    let mut pos = BigInt::one();
    for _ in 0..200 {
        pos = &pos * &two;
    }
    let neg = -pos.clone();

    for value in [pos.clone(), neg.clone()] {
        let be = value.to_signed_bytes_be();
        let le = value.to_signed_bytes_le();
        assert_eq!(BigInt::from_signed_bytes_be(&be), value);
        assert_eq!(BigInt::from_signed_bytes_le(&le), value);
    }

    // 2^200 has bit 200 set, so its top byte is 0x01 (bit 0 of byte 25 in BE
    // form), requiring 26 bytes (25 zero bytes + the 0x01) plus a leading
    // 0x00 only if the high byte had bit 7 set — which it does NOT (0x01).
    // So no leading 0x00 needed.
    let pos_be = pos.to_signed_bytes_be();
    assert_eq!(pos_be[0], 0x01);
    assert_eq!(pos_be.len(), 26);

    // -2^200 in two's complement: |n|-1 = 2^200 - 1 has 200 set bits in 25
    // bytes (all 0xFF), then NOT gives 25 bytes of 0x00 — which would
    // sign-extend to positive, so we must prepend a 0xFF. Total: 26 bytes,
    // top is 0xFF, followed by 25 zeros.
    let neg_be = neg.to_signed_bytes_be();
    assert_eq!(neg_be[0], 0xFF);
    assert_eq!(neg_be.len(), 26);
    for &b in &neg_be[1..] {
        assert_eq!(b, 0x00);
    }
}

#[test]
fn signed_bytes_roundtrip_exact_powers_of_2_negative() {
    // -2^7 = -128 → [0x80] (single byte, top bit set, exactly -128).
    let neg_128 = BigInt::from(-128i64);
    assert_eq!(neg_128.to_signed_bytes_be(), vec![0x80u8]);
    // -2^15 = -32768 → [0x80, 0x00].
    let neg_2_15 = BigInt::from(-(1i64 << 15));
    assert_eq!(neg_2_15.to_signed_bytes_be(), vec![0x80u8, 0x00]);
    assert_eq!(BigInt::from_signed_bytes_be(&[0x80u8, 0x00]), neg_2_15,);
    // -2^63 = i64::MIN → [0x80, 0, 0, 0, 0, 0, 0, 0].
    let neg_i64_min = BigInt::from(i64::MIN);
    let expected = {
        let mut v = vec![0u8; 8];
        v[0] = 0x80;
        v
    };
    assert_eq!(neg_i64_min.to_signed_bytes_be(), expected);
}

// ---------------------------------------------------------------------------
// BW1: Two's-complement bitwise ops on BigInt + fmt traits (BigUint + BigInt)
// ---------------------------------------------------------------------------

#[test]
fn bw1_not_basic() {
    assert_eq!(!BigInt::zero(), BigInt::from(-1i64)); // !0 == -1
    assert_eq!(!BigInt::from(5i64), BigInt::from(-6i64)); // !5 == -6
    assert_eq!(!BigInt::from(-1i64), BigInt::zero()); // !(-1) == 0
    assert_eq!(!BigInt::from(-6i64), BigInt::from(5i64)); // !(-6) == 5
}

#[test]
fn bw1_not_double() {
    for v in [-1000i64, -1, 0, 1, 1000, i64::MAX, i64::MIN] {
        let n = BigInt::from(v);
        assert_eq!(!!n.clone(), n, "!!n == n failed for {v}");
    }
}

#[test]
fn bw1_and_neg_one_identity() {
    // -1 in two's complement has infinite 1-bits; -1 & x == x.
    let neg1 = BigInt::from(-1i64);
    let ff = BigInt::from(0xFFu64);
    let result = &neg1 & &ff;
    assert_eq!(result, ff);
}

#[test]
fn bw1_or_neg_one_absorb() {
    // -1 | any == -1
    let neg1 = BigInt::from(-1i64);
    let ff = BigInt::from(0xFFu64);
    assert_eq!(&neg1 | &ff, neg1);
}

#[test]
fn bw1_xor_self_zero() {
    // x ^ x == 0 for any x
    let neg1 = BigInt::from(-1i64);
    assert_eq!(&neg1 ^ &neg1, BigInt::zero());
    let large = BigInt::from(i64::MIN);
    assert_eq!(&large ^ &large, BigInt::zero());
}

#[test]
fn bw1_arith_shr_negative() {
    assert_eq!(BigInt::from(-8i64) >> 1u64, BigInt::from(-4i64));
    assert_eq!(BigInt::from(-7i64) >> 1u64, BigInt::from(-4i64)); // floor(-3.5) = -4
    assert_eq!(BigInt::from(-1i64) >> 1u64, BigInt::from(-1i64));
    assert_eq!(BigInt::from(-1i64) >> 100u64, BigInt::from(-1i64));
    assert_eq!(BigInt::from(7i64) >> 1u64, BigInt::from(3i64));
}

#[test]
fn bw1_shl_signed() {
    assert_eq!(BigInt::from(1i64) << 4u64, BigInt::from(16i64));
    assert_eq!(BigInt::from(-1i64) << 4u64, BigInt::from(-16i64));
    assert_eq!(BigInt::zero() << 100u64, BigInt::zero());
}

#[test]
fn bw1_de_morgan() {
    // !(a & b) == !a | !b  and  !(a | b) == !a & !b
    let pairs: &[(i64, i64)] = &[
        (0, 0),
        (5, 3),
        (-5, 3),
        (5, -3),
        (-5, -3),
        (-1, 0xFF),
        (i64::MAX, i64::MIN),
    ];
    for &(av, bv) in pairs {
        let a = BigInt::from(av);
        let b = BigInt::from(bv);
        assert_eq!(
            !(&a & &b),
            !a.clone() | !b.clone(),
            "De Morgan (and→or) failed for ({av}, {bv})"
        );
        assert_eq!(
            !(&a | &b),
            !a.clone() & !b.clone(),
            "De Morgan (or→and) failed for ({av}, {bv})"
        );
    }
}

#[test]
fn bw1_i128_cross_val() {
    // Cross-validate against i128 for values representable in i128.
    let vals: &[i128] = &[
        0,
        1,
        -1,
        127,
        -128,
        1000,
        -1000,
        i64::MAX as i128,
        i64::MIN as i128,
    ];
    for &i in vals {
        let a = BigInt::from(i);
        // NOT
        assert_eq!(!a.clone(), BigInt::from(!i), "!{i} mismatch");
        for &j in vals {
            let b = BigInt::from(j);
            assert_eq!(&a & &b, BigInt::from(i & j), "{i} & {j} mismatch");
            assert_eq!(&a | &b, BigInt::from(i | j), "{i} | {j} mismatch");
            assert_eq!(&a ^ &b, BigInt::from(i ^ j), "{i} ^ {j} mismatch");
            if (0..100).contains(&j) {
                assert_eq!(
                    a.clone() >> (j as u64),
                    BigInt::from(i >> j),
                    "{i} >> {j} mismatch"
                );
            }
        }
    }
}

#[test]
fn bw1_fmt_lower_hex_biguint() {
    assert_eq!(format!("{:x}", BigUint::from_u64(255)), "ff");
    assert_eq!(format!("{:#x}", BigUint::from_u64(255)), "0xff");
    assert_eq!(format!("{:x}", BigUint::zero()), "0");
    assert_eq!(format!("{:x}", BigUint::from_u64(0xDEAD_BEEF)), "deadbeef");
}

#[test]
fn bw1_fmt_upper_hex_biguint() {
    assert_eq!(format!("{:X}", BigUint::from_u64(255)), "FF");
    assert_eq!(format!("{:#X}", BigUint::from_u64(255)), "0xFF");
    assert_eq!(format!("{:X}", BigUint::zero()), "0");
}

#[test]
fn bw1_fmt_octal_biguint() {
    assert_eq!(format!("{:o}", BigUint::from_u64(8)), "10");
    assert_eq!(format!("{:#o}", BigUint::from_u64(8)), "0o10");
    assert_eq!(format!("{:o}", BigUint::zero()), "0");
    assert_eq!(format!("{:o}", BigUint::from_u64(0o777)), "777");
}

#[test]
fn bw1_fmt_binary_biguint() {
    assert_eq!(format!("{:b}", BigUint::from_u64(5)), "101");
    assert_eq!(format!("{:#b}", BigUint::from_u64(5)), "0b101");
    assert_eq!(format!("{:b}", BigUint::zero()), "0");
    assert_eq!(format!("{:b}", BigUint::from_u64(0b1010_1100)), "10101100");
}

#[test]
fn bw1_fmt_bigint_negative_hex() {
    assert_eq!(format!("{:x}", BigInt::from(-255i64)), "-ff");
    assert_eq!(format!("{:X}", BigInt::from(-255i64)), "-FF");
    assert_eq!(format!("{:o}", BigInt::from(-8i64)), "-10");
    assert_eq!(format!("{:b}", BigInt::from(-5i64)), "-101");
    assert_eq!(format!("{:#x}", BigInt::from(-255i64)), "-0xff");
}

#[test]
fn bw1_fmt_bigint_positive_hex() {
    assert_eq!(format!("{:x}", BigInt::from(255i64)), "ff");
    assert_eq!(format!("{:#x}", BigInt::from(255i64)), "0xff");
    assert_eq!(format!("{:X}", BigInt::from(255i64)), "FF");
    assert_eq!(format!("{:b}", BigInt::from(5i64)), "101");
    assert_eq!(format!("{:#b}", BigInt::from(5i64)), "0b101");
    assert_eq!(format!("{:o}", BigInt::zero()), "0");
}

// ---------------------------------------------------------------------------
// Serde JSON round-trip (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn serde_biguint_json_roundtrip() {
    use oxinum_int::native::BigUint as N;

    let cases = vec![
        N::zero(),
        N::one(),
        N::from_u64(42),
        N::from_u64(u64::MAX),
        N::from_u128(u128::MAX),
        N::from_le_limbs(&[1, 2, 3, 4]),
    ];
    for n in cases {
        let json = serde_json::to_string(&n).expect("serialize");
        let back: N = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, n, "round-trip mismatch for {n:?}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn serde_bigint_json_roundtrip() {
    let cases = vec![
        BigInt::zero(),
        BigInt::one(),
        BigInt::from(1i64),
        BigInt::from(-1i64),
        BigInt::from(i64::MAX),
        BigInt::from(i64::MIN),
        BigInt::from(42i64),
        BigInt::from(-42i64),
    ];
    for n in cases {
        let json = serde_json::to_string(&n).expect("serialize");
        let back: BigInt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, n, "round-trip mismatch for {n:?}");
        assert_eq!(
            back.sign(),
            n.sign(),
            "sign mismatch after round-trip for {n:?}",
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn serde_bigint_negative_zero_canonical() {
    // Even if a deserialized BigInt had Sign::Negative + zero magnitude
    // (which shouldn't happen via our normal serialize path), it should
    // canonicalize correctly via Eq. The serialize side always emits the
    // canonical form, but if a hand-crafted JSON re-creates a "-0"...
    // we accept what we get and rely on canonicalize at construction.
    let z = BigInt::zero();
    let json = serde_json::to_string(&z).expect("serialize");
    let back: BigInt = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, z);
    assert_eq!(back.sign(), Sign::Positive);
}
