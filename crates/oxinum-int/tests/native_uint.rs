//! Integration tests for `oxinum_int::native::BigUint`.
//!
//! Three test surfaces:
//!
//! 1. Pinned division surface — explicit unit tests for the four mandatory
//!    cases (single-limb fast path, power-of-2, Knuth-D normalization edge,
//!    dashu cross-val sweep).
//! 2. Proptest algebraic laws (add comm/assoc, mul comm/assoc + distrib,
//!    `a == (a/b)*b + a%b`, `(a<<k)>>k == a`).
//! 3. Dashu cross-validation for add/mul/div/rem/cmp over random inputs.

use dashu_int::UBig;
use oxinum_int::native::BigUint;
use proptest::prelude::*;
use proptest::test_runner::Config as PropConfig;

// ---------------------------------------------------------------------------
// Conversion helpers (native <-> dashu) via byte arrays.
// ---------------------------------------------------------------------------

fn to_dashu(n: &BigUint) -> UBig {
    UBig::from_le_bytes(&n.to_bytes_le())
}

fn from_dashu(u: &UBig) -> BigUint {
    BigUint::from_bytes_le(&u.to_le_bytes())
}

fn limbs_to_native(limbs: &[u64]) -> BigUint {
    BigUint::from_le_limbs(limbs)
}

fn limbs_to_dashu(limbs: &[u64]) -> UBig {
    let mut bytes: Vec<u8> = Vec::with_capacity(limbs.len() * 8);
    for &l in limbs {
        bytes.extend_from_slice(&l.to_le_bytes());
    }
    UBig::from_le_bytes(&bytes)
}

// ---------------------------------------------------------------------------
// Pinned division surface
// ---------------------------------------------------------------------------

#[test]
fn pinned_a_single_limb_fast_path() {
    // Take the fast path: divisor.limbs.len() == 1.
    let u = limbs_to_native(&[
        0xDEAD_BEEF_CAFE_BABE,
        0x1234_5678_9ABC_DEF0,
        0x0F0F_0F0F_F0F0_F0F0,
        0x42,
    ]);
    let v = BigUint::from_u64(0xDEAD_BEEF);
    let (q, r) = oxinum_int::native::divrem(&u, &v);
    // Check via dashu.
    let (dq, dr) = (to_dashu(&u) / to_dashu(&v), to_dashu(&u) % to_dashu(&v));
    assert_eq!(q, from_dashu(&dq), "single-limb quotient mismatch");
    assert_eq!(r, from_dashu(&dr), "single-limb remainder mismatch");
    assert!(r < v);
}

#[test]
fn pinned_b_power_of_two_divisor_vs_shr() {
    // Multi-limb power-of-two divisor: 2^192.
    let u = limbs_to_native(&[
        0xAAAA_5555_AAAA_5555,
        0xCAFE_BABE_DEAD_BEEF,
        0x1234_5678_9ABC_DEF0,
        0xFFFF_FFFF_FFFF_FFFF,
        0x42,
    ]);
    let v = limbs_to_native(&[0, 0, 0, 1]); // 2^192
    let (q, r) = oxinum_int::native::divrem(&u, &v);
    assert_eq!(q, u.shr_bits(192));
    let expected_r = limbs_to_native(&[u.as_limbs()[0], u.as_limbs()[1], u.as_limbs()[2]]);
    assert_eq!(r, expected_r);
}

#[test]
fn pinned_c_knuth_d_normalization_edge_high_bit_set() {
    // Top divisor limb is 0x8000_0000_0000_0001 (just above 2^63 — shift=0
    // because high bit already set).
    let v = limbs_to_native(&[0xDEAD_BEEF_CAFE_BABE, 0x8000_0000_0000_0001u64]);
    // Pick several numerators differing in length.
    let numerators = vec![
        limbs_to_native(&[1, 0, 1]),
        limbs_to_native(&[u64::MAX, u64::MAX, u64::MAX, u64::MAX]),
        limbs_to_native(&[0, 0, 0, 0, 1]), // 2^256
        limbs_to_native(&[0xAAAA_5555_AAAA_5555; 6]),
    ];
    for u in numerators {
        let (q, r) = oxinum_int::native::divrem(&u, &v);
        let (dq, dr) = (to_dashu(&u) / to_dashu(&v), to_dashu(&u) % to_dashu(&v));
        assert_eq!(
            q,
            from_dashu(&dq),
            "qhat-corner quotient mismatch for u={u:?}"
        );
        assert_eq!(
            r,
            from_dashu(&dr),
            "qhat-corner remainder mismatch for u={u:?}"
        );
        assert!(r < v);
    }
}

#[test]
fn pinned_c_knuth_d_normalization_edge_max_top_limb() {
    let v = limbs_to_native(&[1, u64::MAX]);
    let numerators = vec![
        limbs_to_native(&[0, 0, 1]),
        limbs_to_native(&[u64::MAX, u64::MAX, u64::MAX]),
        limbs_to_native(&[1, 0, 0, 1]),
        limbs_to_native(&[0xAAAA_5555_AAAA_5555; 5]),
    ];
    for u in numerators {
        let (q, r) = oxinum_int::native::divrem(&u, &v);
        let (dq, dr) = (to_dashu(&u) / to_dashu(&v), to_dashu(&u) % to_dashu(&v));
        assert_eq!(q, from_dashu(&dq), "max-top-limb quotient mismatch");
        assert_eq!(r, from_dashu(&dr), "max-top-limb remainder mismatch");
        assert!(r < v);
    }
}

#[test]
fn pinned_d_dashu_cross_val_sweep_random() {
    // ~1000 random (num_limbs in 1..=100, den_limbs in 1..=num_limbs) pairs,
    // sampled with a deterministic xorshift PRNG (no extra deps).
    let mut state: u64 = 0xC001_BEEF_C0FF_EE01;
    let mut iter_count = 0;
    let target = 1_000;
    let mut fast_path_hits = 0usize;
    let mut multi_limb_hits = 0usize;
    while iter_count < target {
        // Choose num_limbs in 1..=100.
        state = xorshift64(state);
        let num_limbs = 1 + (state % 100) as usize;
        state = xorshift64(state);
        let den_limbs = 1 + (state % num_limbs as u64) as usize;
        // Build random limb vectors.
        let mut u_limbs = Vec::with_capacity(num_limbs);
        for _ in 0..num_limbs {
            state = xorshift64(state);
            u_limbs.push(state);
        }
        let mut v_limbs = Vec::with_capacity(den_limbs);
        for _ in 0..den_limbs {
            state = xorshift64(state);
            v_limbs.push(state);
        }
        // Force the top limb of v to be non-zero (otherwise normalization
        // would change the effective divisor length and the test loses purity).
        if v_limbs[den_limbs - 1] == 0 {
            v_limbs[den_limbs - 1] = 1;
        }
        let u = limbs_to_native(&u_limbs);
        let v = limbs_to_native(&v_limbs);
        if v.is_zero() {
            continue;
        }
        let du = limbs_to_dashu(&u_limbs);
        let dv = limbs_to_dashu(&v_limbs);
        let (q, r) = oxinum_int::native::divrem(&u, &v);
        let dq = &du / &dv;
        let dr = &du % &dv;
        assert_eq!(
            q,
            from_dashu(&dq),
            "quotient mismatch: u={u_limbs:?}, v={v_limbs:?}"
        );
        assert_eq!(
            r,
            from_dashu(&dr),
            "remainder mismatch: u={u_limbs:?}, v={v_limbs:?}"
        );
        // Invariant a = q*v + r
        let back = &(&q * &v) + &r;
        assert_eq!(back, u, "reconstruction failed");
        assert!(r < v, "remainder >= divisor");
        if den_limbs == 1 {
            fast_path_hits += 1;
        } else {
            multi_limb_hits += 1;
        }
        iter_count += 1;
    }
    // Sanity: we should have exercised both code paths.
    assert!(fast_path_hits > 0, "no single-limb cases sampled");
    assert!(multi_limb_hits > 0, "no multi-limb (Knuth-D) cases sampled");
}

#[test]
fn pinned_d_dashu_cross_val_add_back_trigger() {
    // Deliberately construct cases that should trigger D6 add-back.
    // The classic recipe: divisor v whose two highest limbs differ such
    // that the linear estimate is too aggressive.
    let v = limbs_to_native(&[u64::MAX, 0x8000_0000_0000_0000]);
    let numerators = vec![
        limbs_to_native(&[u64::MAX, u64::MAX, 0x7FFF_FFFF_FFFF_FFFF]),
        limbs_to_native(&[0, u64::MAX, 0x7FFF_FFFF_FFFF_FFFF]),
        limbs_to_native(&[1, 0, 0x8000_0000_0000_0000]),
        limbs_to_native(&[0, 1, 0, 0x8000_0000_0000_0000]),
    ];
    for u in numerators {
        let (q, r) = oxinum_int::native::divrem(&u, &v);
        let dq = to_dashu(&u) / to_dashu(&v);
        let dr = to_dashu(&u) % to_dashu(&v);
        assert_eq!(q, from_dashu(&dq), "add-back trigger quotient mismatch");
        assert_eq!(r, from_dashu(&dr), "add-back trigger remainder mismatch");
    }
}

// ---------------------------------------------------------------------------
// Helper PRNG: xorshift64 (well-tested, no extra deps).
// ---------------------------------------------------------------------------

fn xorshift64(mut s: u64) -> u64 {
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    s
}

// ---------------------------------------------------------------------------
// Proptest algebraic laws + cross-val for add/mul/cmp.
// ---------------------------------------------------------------------------

fn arb_biguint() -> impl Strategy<Value = BigUint> {
    // Up to 8 limbs to keep proptest cases small enough.
    prop::collection::vec(any::<u64>(), 0..8).prop_map(|limbs| BigUint::from_le_limbs(&limbs))
}

fn arb_nonzero_biguint() -> impl Strategy<Value = BigUint> {
    // Always non-empty; force the top limb to be nonzero.
    (
        1usize..=6,
        any::<u64>(),
        prop::collection::vec(any::<u64>(), 0..6),
    )
        .prop_map(|(top_pos, top, mut rest)| {
            let top_nonzero = if top == 0 { 1 } else { top };
            while rest.len() < top_pos {
                rest.push(0);
            }
            let mut limbs = rest;
            limbs.push(top_nonzero);
            BigUint::from_le_limbs(&limbs)
        })
}

proptest! {
    #![proptest_config(PropConfig::with_cases(256))]

    #[test]
    fn add_commutative(a in arb_biguint(), b in arb_biguint()) {
        prop_assert_eq!(&a + &b, &b + &a);
    }

    #[test]
    fn add_associative(a in arb_biguint(), b in arb_biguint(), c in arb_biguint()) {
        prop_assert_eq!((&a + &b) + &c, &a + (&b + &c));
    }

    #[test]
    fn mul_commutative(a in arb_biguint(), b in arb_biguint()) {
        prop_assert_eq!(&a * &b, &b * &a);
    }

    #[test]
    fn mul_associative(a in arb_biguint(), b in arb_biguint(), c in arb_biguint()) {
        prop_assert_eq!((&a * &b) * &c, &a * (&b * &c));
    }

    #[test]
    fn mul_distributes_over_add(
        a in arb_biguint(), b in arb_biguint(), c in arb_biguint()
    ) {
        prop_assert_eq!(&a * (&b + &c), &a * &b + &a * &c);
        prop_assert_eq!((&a + &b) * &c, &a * &c + &b * &c);
    }

    #[test]
    fn divrem_invariant(a in arb_biguint(), b in arb_nonzero_biguint()) {
        let (q, r) = oxinum_int::native::divrem(&a, &b);
        prop_assert_eq!(&(&q * &b) + &r, a);
        prop_assert!(r < b);
    }

    #[test]
    fn shl_shr_inverse(a in arb_biguint(), k in 0u64..200) {
        let back = (&a << k) >> k;
        prop_assert_eq!(back, a);
    }

    #[test]
    fn dashu_cross_val_add(a in arb_biguint(), b in arb_biguint()) {
        let r_native = &a + &b;
        let r_dashu = to_dashu(&a) + to_dashu(&b);
        prop_assert_eq!(r_native, from_dashu(&r_dashu));
    }

    #[test]
    fn dashu_cross_val_mul(a in arb_biguint(), b in arb_biguint()) {
        let r_native = &a * &b;
        let r_dashu = to_dashu(&a) * to_dashu(&b);
        prop_assert_eq!(r_native, from_dashu(&r_dashu));
    }

    #[test]
    fn dashu_cross_val_divrem(a in arb_biguint(), b in arb_nonzero_biguint()) {
        let (q, r) = oxinum_int::native::divrem(&a, &b);
        let dq = to_dashu(&a) / to_dashu(&b);
        let dr = to_dashu(&a) % to_dashu(&b);
        prop_assert_eq!(q, from_dashu(&dq));
        prop_assert_eq!(r, from_dashu(&dr));
    }

    #[test]
    fn dashu_cross_val_cmp(a in arb_biguint(), b in arb_biguint()) {
        let native_ord = a.cmp(&b);
        let dashu_ord = to_dashu(&a).cmp(&to_dashu(&b));
        prop_assert_eq!(native_ord, dashu_ord);
    }
}

// ---------------------------------------------------------------------------
// Smaller hand-rolled sanity tests for edge cases the proptest may miss.
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_bytes_match_dashu() {
    let n = limbs_to_native(&[0xDEAD_BEEF_CAFE_BABE, 0x1234_5678_9ABC_DEF0, 0x42]);
    let dn = to_dashu(&n);
    assert_eq!(n.to_bytes_be(), dn.to_be_bytes().to_vec());
    assert_eq!(n.to_bytes_le(), dn.to_le_bytes().to_vec());
}

#[test]
fn radix_decimal_matches_dashu() {
    let n = limbs_to_native(&[0xAAAA_5555_AAAA_5555, 0xCAFE_BABE_DEAD_BEEF, 0x42]);
    let dn = to_dashu(&n);
    assert_eq!(n.to_radix(10).expect("decimal"), format!("{dn}"));
}

#[test]
fn radix_hex_matches_dashu() {
    let n = limbs_to_native(&[0xAAAA_5555_AAAA_5555, 0xCAFE_BABE_DEAD_BEEF, 0x42]);
    let dn = to_dashu(&n);
    assert_eq!(n.to_radix(16).expect("hex"), format!("{}", dn.in_radix(16)));
}
