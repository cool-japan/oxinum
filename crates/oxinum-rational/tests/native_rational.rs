//! Integration tests for `oxinum_rational::native::BigRational`.
//!
//! Covers invariant enforcement, hand-picked equalities, proptest-backed
//! algebraic identities, and a cross-validation suite against
//! `dashu_ratio::RBig`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use oxinum_core::OxiNumError;
use oxinum_int::native::{BigInt, BigUint};
use oxinum_rational::native::BigRational;

// Cross-validation imports.
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn r(n: i64, d: u64) -> BigRational {
    BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d)).expect("non-zero denominator")
}

fn hash_of(v: &BigRational) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// Build the equivalent `RBig` from a `(num: i64, den: u64)` pair.
fn rbig_from(n: i64, d: u64) -> RBig {
    RBig::from_parts(IBig::from(n), UBig::from(d))
}

/// Build a `BigRational` from `(i64, u64)` for cross-validation.
fn brat_from(n: i64, d: u64) -> BigRational {
    BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d)).expect("non-zero denominator")
}

// ---------------------------------------------------------------------------
// Invariants & construction
// ---------------------------------------------------------------------------

#[test]
fn from_parts_reduces_six_quarters() {
    let v = brat_from(6, 4);
    assert_eq!(v.num(), &BigInt::from(3i64));
    assert_eq!(v.den(), &BigUint::from_u64(2));
}

#[test]
fn from_parts_handles_negative_numerator_canonical_sign() {
    let v = brat_from(-9, 12);
    // Discriminator: sign-on-num + reduction + Display in one shot.
    assert_eq!(v.to_string(), "-3/4");
    assert_eq!(v.num(), &BigInt::from(-3i64));
    assert_eq!(v.den(), &BigUint::from_u64(4));
}

#[test]
fn from_parts_zero_over_five_is_canonical_zero() {
    let v = brat_from(0, 5);
    assert_eq!(v.num(), &BigInt::ZERO);
    assert_eq!(v.den(), &BigUint::one());
    assert_eq!(v.to_string(), "0");
}

#[test]
fn canonical_zero_is_unique_across_constructions() {
    let a = brat_from(0, 5);
    let b = brat_from(0, 99);
    let c = BigRational::zero();
    assert_eq!(a, b);
    assert_eq!(b, c);
    // Hashes match too because the canonical form is unique.
    assert_eq!(hash_of(&a), hash_of(&c));
    assert_eq!(hash_of(&b), hash_of(&c));
}

#[test]
fn div_by_zero_in_from_parts() {
    assert_eq!(
        BigRational::from_parts(BigInt::from(1i64), BigUint::ZERO),
        Err(OxiNumError::DivByZero)
    );
}

#[test]
fn recip_of_zero_errors() {
    assert_eq!(BigRational::zero().recip(), Err(OxiNumError::DivByZero));
}

// ---------------------------------------------------------------------------
// Hand-picked equalities
// ---------------------------------------------------------------------------

#[test]
fn half_plus_third_equals_five_sixths() {
    assert_eq!(r(1, 2) + r(1, 3), r(5, 6));
}

#[test]
fn two_thirds_times_three_quarters_equals_one_half() {
    assert_eq!(r(2, 3) * r(3, 4), r(1, 2));
}

#[test]
fn one_minus_one_third_equals_two_thirds() {
    let one = BigRational::one();
    assert_eq!(one - r(1, 3), r(2, 3));
}

#[test]
fn add_inverse_pair_is_zero() {
    let sum = r(-3, 4) + r(3, 4);
    assert!(sum.is_zero());
    assert_eq!(sum.to_string(), "0");
}

#[test]
fn integer_display_and_predicate() {
    let i = BigRational::from_i64(42);
    assert!(i.is_integer());
    assert_eq!(i.to_string(), "42");
}

// ---------------------------------------------------------------------------
// Proptest — algebraic identities
// ---------------------------------------------------------------------------

/// Bounded magnitudes to keep proptest cycles fast while still exercising
/// non-trivial sign and reduction interaction.
const RANGE: i64 = 50_000;
const DEN_RANGE: u64 = 50_000;

fn arb_rational() -> impl Strategy<Value = BigRational> {
    (-RANGE..=RANGE, 1u64..=DEN_RANGE).prop_map(|(n, d)| brat_from(n, d))
}

fn arb_nonzero_rational() -> impl Strategy<Value = BigRational> {
    (-RANGE..=RANGE, 1u64..=DEN_RANGE)
        .prop_filter("nonzero numerator", |(n, _)| *n != 0)
        .prop_map(|(n, d)| brat_from(n, d))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    #[test]
    fn prop_add_commutative(a in arb_rational(), b in arb_rational()) {
        prop_assert_eq!(&a + &b, &b + &a);
    }

    #[test]
    fn prop_mul_commutative(a in arb_rational(), b in arb_rational()) {
        prop_assert_eq!(&a * &b, &b * &a);
    }

    #[test]
    fn prop_add_associative(
        a in arb_rational(),
        b in arb_rational(),
        c in arb_rational(),
    ) {
        prop_assert_eq!(&(&a + &b) + &c, &a + &(&b + &c));
    }

    #[test]
    fn prop_mul_associative(
        a in arb_rational(),
        b in arb_rational(),
        c in arb_rational(),
    ) {
        prop_assert_eq!(&(&a * &b) * &c, &a * &(&b * &c));
    }

    #[test]
    fn prop_left_distributive(
        a in arb_rational(),
        b in arb_rational(),
        c in arb_rational(),
    ) {
        // a * (b + c) == a*b + a*c
        prop_assert_eq!(&a * &(&b + &c), &(&a * &b) + &(&a * &c));
    }

    #[test]
    fn prop_right_distributive(
        a in arb_rational(),
        b in arb_rational(),
        c in arb_rational(),
    ) {
        // (a + b) * c == a*c + b*c
        prop_assert_eq!(&(&a + &b) * &c, &(&a * &c) + &(&b * &c));
    }

    #[test]
    fn prop_add_negation_is_zero(a in arb_rational()) {
        let neg = -&a;
        let sum = &a + &neg;
        prop_assert!(sum.is_zero());
    }

    #[test]
    fn prop_mul_recip_is_one(a in arb_nonzero_rational()) {
        let inv = a.recip().expect("non-zero source");
        let prod = &a * &inv;
        prop_assert!(prod.is_one(), "expected 1, got {prod}");
    }

    #[test]
    fn prop_ord_transitive(
        a in arb_rational(),
        b in arb_rational(),
        c in arb_rational(),
    ) {
        if a <= b && b <= c {
            prop_assert!(a <= c);
        }
        if a >= b && b >= c {
            prop_assert!(a >= c);
        }
    }

    #[test]
    fn prop_hash_eq_consistency(
        a_num in -RANGE..=RANGE,
        a_den in 1u64..=DEN_RANGE,
        scale in 1u64..=128,
    ) {
        // Build two rationals that must be equal: one in unreduced form, one
        // pre-reduced through a separate construction.
        let unreduced = BigRational::from_parts(
            BigInt::from(a_num.saturating_mul(scale as i64)),
            BigUint::from_u64(a_den.saturating_mul(scale)),
        ).expect("non-zero denominator");
        let baseline = brat_from(a_num, a_den);
        prop_assert_eq!(&unreduced, &baseline);
        prop_assert_eq!(hash_of(&unreduced), hash_of(&baseline));
    }
}

// ---------------------------------------------------------------------------
// Cross-validation against dashu_ratio::RBig
// ---------------------------------------------------------------------------

/// Convert our `BigRational` into the canonical "num/den" or "num" string
/// representation. `RBig`'s `Display` already uses the same format, so a
/// string compare suffices as a behavioural cross-check.
fn canonical_string(v: &BigRational) -> String {
    v.to_string()
}

fn rbig_canonical_string(v: &RBig) -> String {
    v.to_string()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, ..ProptestConfig::default() })]

    #[test]
    fn cross_add_matches_rbig(
        a_n in -RANGE..=RANGE,
        a_d in 1u64..=DEN_RANGE,
        b_n in -RANGE..=RANGE,
        b_d in 1u64..=DEN_RANGE,
    ) {
        let ours = brat_from(a_n, a_d) + brat_from(b_n, b_d);
        let theirs = rbig_from(a_n, a_d) + rbig_from(b_n, b_d);
        prop_assert_eq!(canonical_string(&ours), rbig_canonical_string(&theirs));
    }

    #[test]
    fn cross_sub_matches_rbig(
        a_n in -RANGE..=RANGE,
        a_d in 1u64..=DEN_RANGE,
        b_n in -RANGE..=RANGE,
        b_d in 1u64..=DEN_RANGE,
    ) {
        let ours = brat_from(a_n, a_d) - brat_from(b_n, b_d);
        let theirs = rbig_from(a_n, a_d) - rbig_from(b_n, b_d);
        prop_assert_eq!(canonical_string(&ours), rbig_canonical_string(&theirs));
    }

    #[test]
    fn cross_mul_matches_rbig(
        a_n in -RANGE..=RANGE,
        a_d in 1u64..=DEN_RANGE,
        b_n in -RANGE..=RANGE,
        b_d in 1u64..=DEN_RANGE,
    ) {
        let ours = brat_from(a_n, a_d) * brat_from(b_n, b_d);
        let theirs = rbig_from(a_n, a_d) * rbig_from(b_n, b_d);
        prop_assert_eq!(canonical_string(&ours), rbig_canonical_string(&theirs));
    }

    #[test]
    fn cross_div_matches_rbig(
        a_n in -RANGE..=RANGE,
        a_d in 1u64..=DEN_RANGE,
        b_n in (-RANGE..=RANGE).prop_filter("nonzero divisor", |v| *v != 0),
        b_d in 1u64..=DEN_RANGE,
    ) {
        let ours = brat_from(a_n, a_d) / brat_from(b_n, b_d);
        let theirs = rbig_from(a_n, a_d) / rbig_from(b_n, b_d);
        prop_assert_eq!(canonical_string(&ours), rbig_canonical_string(&theirs));
    }

    #[test]
    fn cross_ord_matches_rbig(
        a_n in -RANGE..=RANGE,
        a_d in 1u64..=DEN_RANGE,
        b_n in -RANGE..=RANGE,
        b_d in 1u64..=DEN_RANGE,
    ) {
        let ours = brat_from(a_n, a_d).cmp(&brat_from(b_n, b_d));
        let theirs = rbig_from(a_n, a_d).cmp(&rbig_from(b_n, b_d));
        prop_assert_eq!(ours, theirs);
    }
}

// ---------------------------------------------------------------------------
// Continued fractions (CF1) — native API
// ---------------------------------------------------------------------------

fn ints(vals: &[i64]) -> Vec<BigInt> {
    vals.iter().map(|&v| BigInt::from(v)).collect()
}

#[test]
fn cf_classic_415_over_93() {
    // Classic vector: 415/93 = [4; 2, 6, 7].
    assert_eq!(brat_from(415, 93).continued_fraction(), ints(&[4, 2, 6, 7]));
}

#[test]
fn cf_negative_floor_convention() {
    // Floor (not truncation): -415/93 = [-5; 1, 1, 6, 7].
    assert_eq!(
        brat_from(-415, 93).continued_fraction(),
        ints(&[-5, 1, 1, 6, 7])
    );
}

#[test]
fn cf_integers_and_unit_fractions() {
    assert_eq!(brat_from(5, 1).continued_fraction(), ints(&[5]));
    assert_eq!(brat_from(-7, 1).continued_fraction(), ints(&[-7]));
    assert_eq!(BigRational::zero().continued_fraction(), ints(&[0]));
    // 1/n = [0; n].
    for n in 2u64..=12 {
        assert_eq!(
            brat_from(1, n).continued_fraction(),
            vec![BigInt::ZERO, BigInt::from(n as i64)],
            "1/{n} continued fraction"
        );
    }
}

#[test]
fn from_cf_reconstructs_classic() {
    // [3; 7, 16] = 355/113.
    let cf = ints(&[3, 7, 16]);
    let r = BigRational::from_continued_fraction(&cf).expect("non-empty");
    assert_eq!(r, brat_from(355, 113));
}

#[test]
fn from_cf_empty_errors() {
    assert_eq!(
        BigRational::from_continued_fraction(&[]),
        Err(OxiNumError::Parse("empty continued fraction".into()))
    );
}

#[test]
fn convergents_of_pi_rational() {
    let r = brat_from(355, 113);
    let convs = r.convergents();
    // Convergents: 3, 22/7, 355/113.
    assert_eq!(convs.len(), 3);
    assert_eq!(convs[0], BigRational::from_i64(3));
    assert_eq!(convs[1], brat_from(22, 7));
    assert_eq!(*convs.last().expect("non-empty"), r);
}

#[test]
fn best_approx_semiconvergent_311_over_99() {
    // Discriminating test: the genuinely-best rational with denom <= 100 for
    // 355/113 is the SEMICONVERGENT 311/99, not the convergent 22/7.
    let pi = brat_from(355, 113);
    assert_eq!(
        pi.best_rational_approximation(&BigUint::from_u64(100)),
        brat_from(311, 99)
    );
    // 311/99 is strictly closer to 355/113 than 22/7.
    let err_semi = (&brat_from(311, 99) - &pi).abs();
    let err_conv = (&brat_from(22, 7) - &pi).abs();
    assert!(err_semi < err_conv);
}

#[test]
fn best_approx_e_rational() {
    // e ≈ 2721/1001. Best approx with denom <= 100 is 193/71; with <= 10, 19/7.
    let e = brat_from(2721, 1001);
    assert_eq!(
        e.best_rational_approximation(&BigUint::from_u64(100)),
        brat_from(193, 71)
    );
    assert_eq!(
        e.best_rational_approximation(&BigUint::from_u64(10)),
        brat_from(19, 7)
    );
}

#[test]
fn best_approx_bounds_and_degenerate() {
    let pi = brat_from(355, 113);
    // Denominator high enough -> exact.
    assert_eq!(
        pi.best_rational_approximation(&BigUint::from_u64(113)),
        pi.clone()
    );
    // Small bounds land on the 22/7 convergent.
    assert_eq!(
        pi.best_rational_approximation(&BigUint::from_u64(10)),
        brat_from(22, 7)
    );
    // Denominator 0 -> floor.
    assert_eq!(
        brat_from(7, 2).best_rational_approximation(&BigUint::ZERO),
        BigRational::from_i64(3)
    );
    assert_eq!(
        brat_from(-7, 2).best_rational_approximation(&BigUint::ZERO),
        BigRational::from_i64(-4)
    );
}

// ----- Cross-validation against the dashu-ratio wrapper CF helpers ----------

/// Native `BigInt` and dashu `IBig` both use the standard decimal `Display`,
/// so comparing the stringified coefficient vectors is an exact behavioural
/// cross-check of the continued-fraction expansion.
fn cf_strings_native(v: &BigRational) -> Vec<String> {
    v.continued_fraction()
        .iter()
        .map(|c| c.to_string())
        .collect()
}

fn cf_strings_dashu(n: i64, d: u64) -> Vec<String> {
    let rbig = RBig::from_parts(IBig::from(n), UBig::from(d));
    oxinum_rational::continued_fraction(&rbig)
        .iter()
        .map(|c| c.to_string())
        .collect()
}

/// Brute-force best rational approximation oracle (for small bounds): scan
/// every denominator `q in 1..=max_den`, take the nearest numerator, and keep
/// the closest `p/q`, breaking ties toward the smaller denominator.
fn brute_best_approx(n: i64, d: u64, max_den: u64) -> BigRational {
    let target = brat_from(n, d);
    let mut best: Option<BigRational> = None;
    for q in 1..=max_den {
        // nearest p to n/d * q = (n*q)/d via floor and floor+1.
        let nq = BigInt::from(n) * BigInt::from(q as i64);
        let d_i = BigInt::from(d as i64);
        // floor division (d > 0).
        let (mut p_lo, rem) = {
            let q_t = &nq / &d_i;
            let r_t = &nq - &(&q_t * &d_i);
            (q_t, r_t)
        };
        if rem.is_negative() {
            p_lo = &p_lo - &BigInt::one();
        }
        for cand_p in [p_lo.clone(), &p_lo + &BigInt::one()] {
            let cand = BigRational::from_parts(cand_p, BigUint::from_u64(q)).expect("q >= 1");
            let err = (&cand - &target).abs();
            let take = match &best {
                None => true,
                Some(b) => {
                    let berr = (b - &target).abs();
                    err < berr || (err == berr && cand.den() < b.den())
                }
            };
            if take {
                best = Some(cand);
            }
        }
    }
    best.expect("max_den >= 1")
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// Round-trip: from_cf(cf(r)) == r over random rationals (incl. negatives).
    #[test]
    fn cf_roundtrip_random(
        n in -RANGE..=RANGE,
        d in 1u64..=DEN_RANGE,
    ) {
        let r = brat_from(n, d);
        let cf = r.continued_fraction();
        let back = BigRational::from_continued_fraction(&cf).expect("non-empty");
        prop_assert_eq!(back, r);
    }

    /// CF coefficients agree with the dashu-ratio wrapper (the floor-convention
    /// guard: the wrapper uses a correct `div_floor`). Includes negatives.
    #[test]
    fn cf_coeffs_match_dashu(
        n in -RANGE..=RANGE,
        d in 1u64..=DEN_RANGE,
    ) {
        prop_assert_eq!(
            cf_strings_native(&brat_from(n, d)),
            cf_strings_dashu(n, d)
        );
    }

    /// Convergents strictly improve and the last one is exact.
    #[test]
    fn convergents_improve_random(
        n in -RANGE..=RANGE,
        d in 1u64..=DEN_RANGE,
    ) {
        let r = brat_from(n, d);
        let convs = r.convergents();
        prop_assert!(!convs.is_empty());
        prop_assert_eq!(convs.last().expect("non-empty"), &r);
        let mut prev: Option<BigRational> = None;
        for c in &convs {
            let err = (c - &r).abs();
            if let Some(p) = prev {
                prop_assert!(err < p, "errors must strictly decrease");
            }
            prev = Some(err);
        }
    }

    /// best_rational_approximation matches a brute-force oracle for small
    /// bounds (this validates the genuine-best semiconvergent semantics; the
    /// wrapper is NOT a valid oracle here because it only returns convergents).
    #[test]
    fn best_approx_matches_brute_force(
        n in -2_000i64..=2_000,
        d in 1u64..=2_000,
        max_den in 1u64..=30,
    ) {
        let ours = brat_from(n, d).best_rational_approximation(&BigUint::from_u64(max_den));
        let oracle = brute_best_approx(n, d, max_den);
        // Compare by value (closeness); both must achieve the same minimal error.
        let target = brat_from(n, d);
        let err_ours = (&ours - &target).abs();
        let err_oracle = (&oracle - &target).abs();
        prop_assert_eq!(
            err_ours.clone(), err_oracle.clone(),
            "best_approx({}/{}, {}) = {} (err {}), oracle = {} (err {})",
            n, d, max_den, ours, err_ours, oracle, err_oracle
        );
    }
}

// ---------------------------------------------------------------------------
// Serde JSON round-trip tests (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
mod serde_tests {
    use oxinum_int::native::{BigInt, BigUint};
    use oxinum_rational::native::BigRational;
    use serde_json::Value;

    fn roundtrip(x: &BigRational) {
        let json = serde_json::to_string(x).expect("serialize");
        let back: BigRational = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*x, back, "round-trip failed for {x:?}");
    }

    fn r(n: i64, d: u64) -> BigRational {
        BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d))
            .expect("non-zero denominator")
    }

    #[test]
    fn serde_rational_zero() {
        roundtrip(&BigRational::zero());
    }

    #[test]
    fn serde_rational_one() {
        roundtrip(&BigRational::one());
    }

    #[test]
    fn serde_rational_one_third() {
        roundtrip(&r(1, 3));
    }

    #[test]
    fn serde_rational_negative() {
        roundtrip(&r(-5, 3));
    }

    #[test]
    fn serde_rational_integer() {
        roundtrip(&BigRational::from_i64(42));
        roundtrip(&BigRational::from_i64(-42));
    }

    #[test]
    fn serde_rational_many_fractions() {
        for n in -20i64..=20 {
            for d in 1u64..=20 {
                roundtrip(&r(n, d));
            }
        }
    }

    #[test]
    fn serde_rational_den_zero_rejected() {
        // Mutate a valid serialized BigRational to set den.limbs = [] (zero).
        let valid = r(1, 3);
        let mut val: Value = serde_json::to_value(&valid).expect("serialize");
        val["den"]["limbs"] = Value::Array(vec![]);
        let result: Result<BigRational, _> = serde_json::from_value(val);
        assert!(result.is_err(), "den=0 should fail deserialization");
    }

    #[test]
    fn serde_rational_non_reduced_normalizes() {
        // 2/4 and 1/2 compare equal (canonical form is 1/2).
        let two_fourths = r(2, 4); // auto-reduces to 1/2
        let one_half = r(1, 2);
        // Both serialize identically since both are 1/2 after reduction.
        let json_a = serde_json::to_string(&two_fourths).expect("serialize");
        let json_b = serde_json::to_string(&one_half).expect("serialize");
        assert_eq!(json_a, json_b, "both must serialize as 1/2");
        // Round-trip of either gives 1/2.
        roundtrip(&two_fourths);
        roundtrip(&one_half);
    }

    #[test]
    fn serde_rational_negative_numerator_preserved() {
        let neg = r(-7, 4);
        let json = serde_json::to_string(&neg).expect("serialize");
        let back: BigRational = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.to_string(), "-7/4");
    }
}

// ---------------------------------------------------------------------------
// Phase 6: BigRational → BigInt and BigInt → BigRational conversions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod rational_to_bigint {
    use oxinum_int::native::{BigInt, BigUint};
    use oxinum_rational::native::BigRational;

    fn rat(n: i64, d: u64) -> BigRational {
        BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d))
            .expect("non-zero denominator")
    }

    #[test]
    fn positive_seven_halves() {
        // 7/2 = 3.5 → trunc=3, floor=3, ceil=4
        let r = rat(7, 2);
        assert_eq!(r.to_bigint_trunc(), BigInt::from(3i64), "trunc(7/2)");
        assert_eq!(r.to_bigint_floor(), BigInt::from(3i64), "floor(7/2)");
        assert_eq!(r.to_bigint_ceil(), BigInt::from(4i64), "ceil(7/2)");
    }

    #[test]
    fn negative_seven_halves() {
        // -7/2 = -3.5 → trunc=-3, floor=-4, ceil=-3
        let s = rat(-7, 2);
        assert_eq!(s.to_bigint_trunc(), BigInt::from(-3i64), "trunc(-7/2)");
        assert_eq!(s.to_bigint_floor(), BigInt::from(-4i64), "floor(-7/2)");
        assert_eq!(s.to_bigint_ceil(), BigInt::from(-3i64), "ceil(-7/2)");
    }

    #[test]
    fn exact_integer_rational() {
        // 6/1 → all three = 6
        let r = rat(6, 1);
        assert_eq!(r.to_bigint_trunc(), BigInt::from(6i64));
        assert_eq!(r.to_bigint_floor(), BigInt::from(6i64));
        assert_eq!(r.to_bigint_ceil(), BigInt::from(6i64));
    }

    #[test]
    fn negative_exact_integer_rational() {
        // -5/1 → all three = -5
        let r = rat(-5, 1);
        assert_eq!(r.to_bigint_trunc(), BigInt::from(-5i64));
        assert_eq!(r.to_bigint_floor(), BigInt::from(-5i64));
        assert_eq!(r.to_bigint_ceil(), BigInt::from(-5i64));
    }

    #[test]
    fn zero_rational() {
        let r = BigRational::zero();
        assert_eq!(r.to_bigint_trunc(), BigInt::zero());
        assert_eq!(r.to_bigint_floor(), BigInt::zero());
        assert_eq!(r.to_bigint_ceil(), BigInt::zero());
    }

    #[test]
    fn one_third_positive() {
        // 1/3 → trunc/floor=0, ceil=1
        let r = rat(1, 3);
        assert_eq!(r.to_bigint_trunc(), BigInt::zero(), "trunc(1/3)");
        assert_eq!(r.to_bigint_floor(), BigInt::zero(), "floor(1/3)");
        assert_eq!(r.to_bigint_ceil(), BigInt::one(), "ceil(1/3)");
    }

    #[test]
    fn one_third_negative() {
        // -1/3 → trunc/ceil=0, floor=-1
        let r = rat(-1, 3);
        assert_eq!(r.to_bigint_trunc(), BigInt::zero(), "trunc(-1/3)");
        assert_eq!(r.to_bigint_floor(), BigInt::from(-1i64), "floor(-1/3)");
        assert_eq!(r.to_bigint_ceil(), BigInt::zero(), "ceil(-1/3)");
    }

    #[test]
    fn bigint_to_rational_via_from() {
        // BigInt → BigRational via From<BigInt>: denominator must be 1.
        let n = BigInt::from(5i64);
        let r = BigRational::from(n);
        assert_eq!(r.to_string(), "5");
        assert!(r.is_integer());
    }

    #[test]
    fn bigint_to_rational_negative() {
        let n = BigInt::from(-42i64);
        let r = BigRational::from(n);
        assert_eq!(r.to_string(), "-42");
        assert!(r.is_integer());
    }

    #[test]
    fn bigint_to_rational_zero() {
        let r = BigRational::from(BigInt::zero());
        assert!(r.is_zero());
        assert_eq!(r.to_string(), "0");
    }
}
