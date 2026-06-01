//! Integration tests for Toom-Cook-3 multiplication in
//! `oxinum_int::native::BigUint`.
//!
//! The public `*` operator dispatches operands with `min(len) >=
//! TOOM3_THRESHOLD` (100 limbs) to `mul_toom3`. Since `mul_toom3` itself is
//! crate-private, these tests drive the **public** path with operands large
//! enough to route through Toom-3 and cross-validate the result against
//! `dashu_int::UBig` — the same independent oracle used to validate the rest
//! of the native `BigUint` arithmetic. (Direct-call unit tests on small
//! operands live in `src/native/mul.rs`.)

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

/// Reference product via dashu, returned as a native `BigUint`.
fn ref_mul(a: &BigUint, b: &BigUint) -> BigUint {
    from_dashu(&(to_dashu(a) * to_dashu(b)))
}

/// Dependency-free xorshift PRNG (matches the in-crate test PRNG style).
fn next_rand(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn rand_limbs(state: &mut u64, n: usize) -> Vec<u64> {
    (0..n).map(|_| next_rand(state)).collect()
}

// ---------------------------------------------------------------------------
// Threshold boundary: sizes 98..=104 straddling TOOM3_THRESHOLD (100). The
// public `*` routes the 100+ cases to Toom-3 and the others to Karatsuba; all
// must agree with the dashu oracle.
// ---------------------------------------------------------------------------

#[test]
fn toom3_threshold_boundary_all_combinations() {
    let mut st: u64 = 0x9E37_79B9_7F4A_7C15;
    for la in 98..=104usize {
        for lb in 98..=104usize {
            let a = BigUint::from_le_limbs(&rand_limbs(&mut st, la));
            let b = BigUint::from_le_limbs(&rand_limbs(&mut st, lb));
            let got = &a * &b;
            let want = ref_mul(&a, &b);
            assert_eq!(got, want, "boundary mismatch at {la}x{lb}");
        }
    }
}

// ---------------------------------------------------------------------------
// Adversarial limb patterns (all routed through Toom-3 at >= 100 limbs).
// ---------------------------------------------------------------------------

#[test]
fn toom3_all_max_limbs() {
    for len in [100usize, 101, 128, 150, 201, 256] {
        let v = vec![u64::MAX; len];
        let a = BigUint::from_le_limbs(&v);
        let b = BigUint::from_le_limbs(&v);
        assert_eq!(&a * &b, ref_mul(&a, &b), "all-MAX len={len}");
    }
}

#[test]
fn toom3_single_bit_limbs() {
    // Each limb is a single set bit (varying position).
    let a_limbs: Vec<u64> = (0..150).map(|i| 1u64 << (i % 64)).collect();
    let b_limbs: Vec<u64> = (0..150).map(|i| 1u64 << ((i * 7 + 3) % 64)).collect();
    let a = BigUint::from_le_limbs(&a_limbs);
    let b = BigUint::from_le_limbs(&b_limbs);
    assert_eq!(&a * &b, ref_mul(&a, &b), "single-bit limbs");
}

#[test]
fn toom3_power_of_two_operands() {
    // Pure powers of two: 2^k * 2^m == 2^(k+m). k,m chosen above the threshold.
    for (kbits, mbits) in [(7000u64, 9000u64), (6401, 6400), (10000, 12800)] {
        let a = BigUint::one().shl_bits(kbits);
        let b = BigUint::one().shl_bits(mbits);
        assert_eq!(&a * &b, ref_mul(&a, &b), "2^{kbits} * 2^{mbits}");
    }
}

#[test]
fn toom3_internal_zero_limbs() {
    // Operands with scattered internal zero limbs.
    let mut a_limbs = vec![0xDEAD_BEEF_CAFE_BABEu64; 140];
    let mut b_limbs = vec![0x0123_4567_89AB_CDEFu64; 140];
    for i in (0..140).step_by(3) {
        a_limbs[i] = 0;
    }
    for i in (1..140).step_by(4) {
        b_limbs[i] = 0;
    }
    let a = BigUint::from_le_limbs(&a_limbs);
    let b = BigUint::from_le_limbs(&b_limbs);
    assert_eq!(&a * &b, ref_mul(&a, &b), "internal zero limbs");
}

#[test]
fn toom3_highly_asymmetric_lengths() {
    // len(a) >> len(b). Routed through Toom-3 only when min(len) >= 100, but
    // also validate the asymmetric cases where the short side has a 1-limb
    // high block (b2 length 1) and the very-short-side fallback (min < 100,
    // routed to Karatsuba) — all must still match the oracle.
    let mut st: u64 = 0x0F1E_2D3C_4B5A_6978;
    for (la, lb) in [(300, 5), (300, 100), (400, 101), (256, 128), (350, 3)] {
        let a = BigUint::from_le_limbs(&rand_limbs(&mut st, la));
        let b = BigUint::from_le_limbs(&rand_limbs(&mut st, lb));
        assert_eq!(&a * &b, ref_mul(&a, &b), "asymmetric {la}x{lb}");
    }
}

#[test]
fn toom3_short_high_block() {
    // max_len = 121 -> s = 41. With len = 100, the high block (limbs[82..])
    // is only 18 limbs (shorter than s). Both operands sized to force a
    // short / partially-empty high block in the split.
    let mut st: u64 = 0xBADC_0FFE_E0DD_F00D;
    for len in [100usize, 121, 122, 123] {
        let a = BigUint::from_le_limbs(&rand_limbs(&mut st, len));
        let b = BigUint::from_le_limbs(&rand_limbs(&mut st, len));
        assert_eq!(&a * &b, ref_mul(&a, &b), "short high block len={len}");
    }
}

// ---------------------------------------------------------------------------
// Zero / one / trivial-operand handling through the large path.
// ---------------------------------------------------------------------------

#[test]
fn toom3_zero_and_one_operands() {
    let mut st: u64 = 0xC0FF_EE00_1234_5678;
    let big = BigUint::from_le_limbs(&rand_limbs(&mut st, 150));

    // Zero on either side.
    assert!((&big * &BigUint::zero()).is_zero());
    assert!((&BigUint::zero() * &big).is_zero());

    // One on either side (identity).
    assert_eq!(&big * &BigUint::one(), big);
    assert_eq!(&BigUint::one() * &big, big);

    // A2/B2 block of length exactly 1: max_len = 129 -> s = 43, so the high
    // block of a 129-limb operand is exactly limbs[86..129] = 43 limbs; size
    // to 2*s + 1 = 87 limbs so the high block is exactly 1 limb.
    let s = 130usize.div_ceil(3); // 44
    let a = BigUint::from_le_limbs(&rand_limbs(&mut st, 2 * s + 1));
    let b = BigUint::from_le_limbs(&rand_limbs(&mut st, 2 * s + 1));
    assert_eq!(&a * &b, ref_mul(&a, &b), "a2/b2 length 1");
}

// ---------------------------------------------------------------------------
// >= 200 random pairs with limb counts in 100..=400.
// ---------------------------------------------------------------------------

#[test]
fn toom3_random_pairs_dashu_cross_val() {
    let mut st: u64 = 0xDEAD_BEEF_1357_9BDF;
    for _ in 0..220 {
        // limb counts in 100..=400
        let la = 100 + (next_rand(&mut st) % 301) as usize;
        let lb = 100 + (next_rand(&mut st) % 301) as usize;
        let a = BigUint::from_le_limbs(&rand_limbs(&mut st, la));
        let b = BigUint::from_le_limbs(&rand_limbs(&mut st, lb));
        let got = &a * &b;
        let want = ref_mul(&a, &b);
        assert_eq!(got, want, "random pair mismatch at {la}x{lb}");
    }
}

// ---------------------------------------------------------------------------
// Recursive depth: operands large enough that pointwise products themselves
// re-enter Toom-3 (each of the five products is ~min/3 limbs, so min >= 300
// keeps the recursion in the Toom-3 tier for at least one extra level).
// ---------------------------------------------------------------------------

#[test]
fn toom3_recursive_depth() {
    let mut st: u64 = 0x1111_2222_3333_4444;
    let a = BigUint::from_le_limbs(&rand_limbs(&mut st, 360));
    let b = BigUint::from_le_limbs(&rand_limbs(&mut st, 360));
    assert_eq!(&a * &b, ref_mul(&a, &b), "recursive Toom-3 depth");
}

// ---------------------------------------------------------------------------
// Proptest: a * b == dashu(a) * dashu(b) for operands in the Toom-3 range.
// ---------------------------------------------------------------------------

/// Generate a `BigUint` with a limb count in `100..=180` (Toom-3 territory).
fn arb_toom3_biguint() -> impl Strategy<Value = BigUint> {
    prop::collection::vec(any::<u64>(), 100..=180).prop_map(|limbs| BigUint::from_le_limbs(&limbs))
}

proptest! {
    #![proptest_config(PropConfig { cases: 64, ..PropConfig::default() })]

    #[test]
    fn toom3_prop_matches_dashu(
        a in arb_toom3_biguint(),
        b in arb_toom3_biguint(),
    ) {
        let got = &a * &b;
        let want = ref_mul(&a, &b);
        prop_assert_eq!(got, want);
    }

    #[test]
    fn toom3_prop_commutative(
        a in arb_toom3_biguint(),
        b in arb_toom3_biguint(),
    ) {
        prop_assert_eq!(&a * &b, &b * &a);
    }
}
