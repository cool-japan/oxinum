//! Multiplication for [`BigUint`]: schoolbook + Karatsuba + Toom-Cook-3.

use super::int::BigInt;
use super::uint::{normalize, BigUint, KARATSUBA_THRESHOLD, TOOM3_THRESHOLD};
use oxinum_core::Sign;

/// Public dispatcher across three multiplication tiers:
///
/// - `min(len) < KARATSUBA_THRESHOLD` → schoolbook (`O(n*m)`),
/// - `KARATSUBA_THRESHOLD <= min(len) < TOOM3_THRESHOLD` → Karatsuba
///   (`O(n^1.585)`),
/// - `min(len) >= TOOM3_THRESHOLD` → Toom-Cook-3 (`O(n^1.465)`).
///
/// The gate uses `min(a.len, b.len)` so both operands must be large for the
/// higher tier to engage; an asymmetric `len(a) >> len(b)` stays in the tier
/// chosen by the shorter operand.
pub(crate) fn mul(a: &BigUint, b: &BigUint) -> BigUint {
    if a.limbs.is_empty() || b.limbs.is_empty() {
        return BigUint::zero();
    }
    let min_len = a.limbs.len().min(b.limbs.len());
    if min_len < KARATSUBA_THRESHOLD {
        mul_schoolbook(a, b)
    } else if min_len < TOOM3_THRESHOLD {
        mul_karatsuba(a, b)
    } else {
        mul_toom3(a, b)
    }
}

/// Schoolbook long multiplication. O(n*m).
pub(crate) fn mul_schoolbook(a: &BigUint, b: &BigUint) -> BigUint {
    if a.limbs.is_empty() || b.limbs.is_empty() {
        return BigUint::zero();
    }
    let mut out: Vec<u64> = vec![0u64; a.limbs.len() + b.limbs.len()];
    for (i, &ai) in a.limbs.iter().enumerate() {
        let mut carry: u64 = 0;
        for (j, &bj) in b.limbs.iter().enumerate() {
            // prod = ai * bj + out[i+j] + carry (all in u128).
            let prod = (ai as u128) * (bj as u128) + (out[i + j] as u128) + (carry as u128);
            out[i + j] = prod as u64;
            carry = (prod >> 64) as u64;
        }
        // Propagate residual carry into the high limbs.
        out[i + b.limbs.len()] = carry;
    }
    normalize(&mut out);
    BigUint { limbs: out }
}

/// Karatsuba multiplication. Splits operands at half their max length.
///
/// `(a1 B + a0) * (b1 B + b0)`
///   `= a1 b1 B^2 + ((a0+a1)(b0+b1) - a1 b1 - a0 b0) B + a0 b0`
///
/// where `B = 2^(64 * k)`. Recurses on three half-size products.
pub(crate) fn mul_karatsuba(a: &BigUint, b: &BigUint) -> BigUint {
    let na = a.limbs.len();
    let nb = b.limbs.len();
    let n = na.max(nb);
    let k = n / 2;
    // If either side is shorter than `k`, that side has no high half.
    let (a_lo, a_hi) = split_at(&a.limbs, k);
    let (b_lo, b_hi) = split_at(&b.limbs, k);
    let a_lo_u = limbs_to_biguint(a_lo);
    let a_hi_u = limbs_to_biguint(a_hi);
    let b_lo_u = limbs_to_biguint(b_lo);
    let b_hi_u = limbs_to_biguint(b_hi);

    // Three recursive multiplications.
    let z0 = mul(&a_lo_u, &b_lo_u);
    let z2 = mul(&a_hi_u, &b_hi_u);
    // (a_lo + a_hi) * (b_lo + b_hi)
    let a_sum = BigUint::add_ref(&a_lo_u, &a_hi_u);
    let b_sum = BigUint::add_ref(&b_lo_u, &b_hi_u);
    let z1_full = mul(&a_sum, &b_sum);
    // z1 = z1_full - z2 - z0 (never negative because of identity).
    let z1_minus_z2 = z1_full
        .checked_sub(&z2)
        .expect("Karatsuba invariant: z1_full >= z2");
    let z1 = z1_minus_z2
        .checked_sub(&z0)
        .expect("Karatsuba invariant: z1_full - z2 >= z0");

    // result = z0 + (z1 << (64*k)) + (z2 << (128*k))
    let shift_bits_k = (k as u64) * 64;
    let z1_shifted = z1.shl_bits(shift_bits_k);
    let z2_shifted = z2.shl_bits(shift_bits_k * 2);
    let part = BigUint::add_ref(&z0, &z1_shifted);
    BigUint::add_ref(&part, &z2_shifted)
}

/// Toom-Cook-3 multiplication. Splits each operand into three limb-blocks of
/// `s = ceil(max(len_a, len_b) / 3)` limbs, evaluates both polynomials at the
/// five points `{0, 1, -1, 2, ∞}`, multiplies pointwise (recursing through
/// [`mul`]), interpolates the five product coefficients, and recomposes.
///
/// Asymptotic complexity `O(n^{log 5 / log 3}) ≈ O(n^1.465)`.
///
/// # Evaluation
///
/// With `a(x) = a0 + a1 x + a2 x^2` (and likewise `b`), evaluated at the five
/// points and multiplied pointwise:
///
/// ```text
/// V0   = a(0)·b(0)  = a0·b0
/// V1   = a(1)·b(1)  = (a0 + a1 + a2)(b0 + b1 + b2)
/// Vm1  = a(-1)·b(-1)= (a0 - a1 + a2)(b0 - b1 + b2)   (signed)
/// V2   = a(2)·b(2)  = (a0 + 2a1 + 4a2)(b0 + 2b1 + 4b2)
/// Vinf = a(∞)·b(∞)  = a2·b2
/// ```
///
/// # Interpolation
///
/// The product `c(x) = c0 + c1 x + c2 x^2 + c3 x^3 + c4 x^4` has coefficients
/// recovered by the closed-form solution for this point set:
///
/// ```text
/// c0 = V0
/// c4 = Vinf
/// c2 = (V1 + Vm1)/2 - V0 - Vinf
/// c3 = (3·V0 - 3·V1 - Vm1 + V2 - 12·Vinf) / 6
/// c1 = (V1 - Vm1)/2 - c3
/// ```
///
/// Every division here (`/2` and `/6`) is exact: the dividend is provably
/// divisible, so `BigInt`'s truncating division yields the exact quotient
/// (including for negative dividends, since truncation toward zero is exact
/// when there is no remainder).
///
/// # Recomposition
///
/// `c = c0 + c1·B^s + c2·B^{2s} + c3·B^{3s} + c4·B^{4s}` where `B = 2^64`,
/// summed via shifts and additions so inter-block carries propagate. The five
/// `c_i` are each non-negative (the product of two non-negative operands), so
/// the magnitudes are taken directly.
pub(crate) fn mul_toom3(a: &BigUint, b: &BigUint) -> BigUint {
    let max_len = a.limbs.len().max(b.limbs.len());
    // Block size: ceil(max_len / 3). Both operands must be splittable into
    // three blocks (need at least 3 limbs in the larger one). Otherwise the
    // higher tier is meaningless — fall back to the standard dispatcher, which
    // will route to Karatsuba/schoolbook.
    if max_len < 3 || a.limbs.is_empty() || b.limbs.is_empty() {
        return mul(a, b);
    }
    let s = max_len.div_ceil(3);

    // Three blocks each (high blocks may be shorter or empty after slicing).
    let (a0, a1, a2) = split3(&a.limbs, s);
    let (b0, b1, b2) = split3(&b.limbs, s);

    // Evaluate at {0, 1, -1, 2} as signed BigInts; ∞ is the leading block.
    // a(1) = a0 + a1 + a2, etc. (these stay non-negative but are carried as
    // BigInt so the pointwise products compose with the signed Vm1).
    let a0i = to_int(&a0);
    let a1i = to_int(&a1);
    let a2i = to_int(&a2);
    let b0i = to_int(&b0);
    let b1i = to_int(&b1);
    let b2i = to_int(&b2);

    // a(1) = a0 + a1 + a2
    let a_1 = &(&a0i + &a1i) + &a2i;
    let b_1 = &(&b0i + &b1i) + &b2i;
    // a(-1) = a0 - a1 + a2  (signed)
    let a_m1 = &(&a0i - &a1i) + &a2i;
    let b_m1 = &(&b0i - &b1i) + &b2i;
    // a(2) = a0 + 2 a1 + 4 a2
    let two = BigInt::from(2i64);
    let four = BigInt::from(4i64);
    let a_2 = &(&a0i + &(&two * &a1i)) + &(&four * &a2i);
    let b_2 = &(&b0i + &(&two * &b1i)) + &(&four * &b2i);

    // Pointwise products (recurse through `mul` so large blocks use the
    // appropriate tier). `mul_int` multiplies signed values.
    let v0 = &a0i * &b0i;
    let v1 = &a_1 * &b_1;
    let vm1 = &a_m1 * &b_m1;
    let v2 = &a_2 * &b_2;
    let vinf = &a2i * &b2i;

    // Interpolation (closed form for {0, 1, -1, 2, ∞}).
    let three = BigInt::from(3i64);
    let six = BigInt::from(6i64);
    let twelve = BigInt::from(12i64);

    let c0 = v0.clone();
    let c4 = vinf.clone();
    // c2 = (V1 + Vm1)/2 - V0 - Vinf
    let c2 = &(&(&(&v1 + &vm1) / &two) - &v0) - &vinf;
    // c3 = (3 V0 - 3 V1 - Vm1 + V2 - 12 Vinf) / 6
    let c3_num = &(&(&(&(&three * &v0) - &(&three * &v1)) - &vm1) + &v2) - &(&twelve * &vinf);
    let c3 = &c3_num / &six;
    // c1 = (V1 - Vm1)/2 - c3
    let c1 = &(&(&v1 - &vm1) / &two) - &c3;

    // Recompose: r = Σ c_i · B^{s*i}. Each c_i is non-negative.
    let shift = (s as u64) * 64;
    let r0 = to_uint(&c0);
    let r1 = to_uint(&c1).shl_bits(shift);
    let r2 = to_uint(&c2).shl_bits(shift * 2);
    let r3 = to_uint(&c3).shl_bits(shift * 3);
    let r4 = to_uint(&c4).shl_bits(shift * 4);

    let acc = BigUint::add_ref(&r0, &r1);
    let acc = BigUint::add_ref(&acc, &r2);
    let acc = BigUint::add_ref(&acc, &r3);
    BigUint::add_ref(&acc, &r4)
}

/// Split a limb slice into three blocks of `s` limbs each (low, mid, high).
/// Blocks beyond the slice length are empty. The high block may be shorter
/// than `s` (or empty when `len <= 2s`).
#[inline]
fn split3(limbs: &[u64], s: usize) -> (BigUint, BigUint, BigUint) {
    let len = limbs.len();
    let lo = &limbs[..s.min(len)];
    let mid = if 2 * s <= len {
        &limbs[s..2 * s]
    } else if s < len {
        &limbs[s..]
    } else {
        &[][..]
    };
    let hi = if 2 * s < len {
        &limbs[2 * s..]
    } else {
        &[][..]
    };
    (
        limbs_to_biguint(lo),
        limbs_to_biguint(mid),
        limbs_to_biguint(hi),
    )
}

/// Wrap a non-negative `BigUint` as a `BigInt` (always positive sign).
#[inline]
fn to_int(value: &BigUint) -> BigInt {
    BigInt::from_parts(Sign::Positive, value.clone())
}

/// Extract the magnitude of a `BigInt` that is known to be non-negative.
/// In `mul_toom3` every interpolated coefficient is the product of two
/// non-negative polynomials evaluated on `[0, ∞)`, so the result is `>= 0`.
#[inline]
fn to_uint(value: &BigInt) -> BigUint {
    debug_assert!(
        value.sign() == Sign::Positive || value.is_zero(),
        "Toom-3 interpolation produced a negative coefficient"
    );
    value.magnitude().clone()
}

/// Split a limb slice at index `k`. Lower part is `limbs[..k]`, upper is
/// `limbs[k..]` (or empty if `k >= limbs.len()`).
#[inline]
fn split_at(limbs: &[u64], k: usize) -> (&[u64], &[u64]) {
    if k >= limbs.len() {
        (limbs, &[])
    } else {
        (&limbs[..k], &limbs[k..])
    }
}

/// Build a normalized `BigUint` from a limb slice (no trailing zeros).
#[inline]
fn limbs_to_biguint(limbs: &[u64]) -> BigUint {
    let mut v = limbs.to_vec();
    normalize(&mut v);
    BigUint { limbs: v }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schoolbook_small() {
        let a = BigUint::from_u64(123);
        let b = BigUint::from_u64(456);
        assert_eq!(mul_schoolbook(&a, &b), BigUint::from_u64(123 * 456));
    }

    #[test]
    fn schoolbook_high_limb() {
        let a = BigUint::from_u64(u64::MAX);
        let b = BigUint::from_u64(2);
        let r = mul_schoolbook(&a, &b);
        // 2 * (2^64 - 1) = 2^65 - 2 = (limbs: [u64::MAX - 1, 1])
        assert_eq!(r.as_limbs(), &[u64::MAX - 1, 1]);
    }

    #[test]
    fn karatsuba_matches_schoolbook_random() {
        // Construct 40-limb operands and compare both algorithms.
        let mut a_limbs: Vec<u64> = Vec::with_capacity(40);
        let mut b_limbs: Vec<u64> = Vec::with_capacity(40);
        let mut state: u64 = 0xDEAD_BEEF_CAFE_BABE;
        for _ in 0..40 {
            // Xorshift-style PRNG to avoid extra deps.
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            a_limbs.push(state);
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            b_limbs.push(state);
        }
        let a = BigUint::from_le_limbs(&a_limbs);
        let b = BigUint::from_le_limbs(&b_limbs);
        assert_eq!(mul_schoolbook(&a, &b), mul_karatsuba(&a, &b));
    }

    #[test]
    fn karatsuba_unequal_lengths() {
        let a = BigUint::from_le_limbs(&vec![0xAAAA_5555_AAAA_5555u64; 40]);
        let b = BigUint::from_u64(0xDEAD_BEEF_CAFE_BABE);
        assert_eq!(mul_schoolbook(&a, &b), mul(&a, &b));
    }

    #[test]
    fn zero_mul() {
        let a = BigUint::zero();
        let b = BigUint::from_u64(42);
        assert!(mul(&a, &b).is_zero());
        assert!(mul(&b, &a).is_zero());
    }

    // -----------------------------------------------------------------------
    // Toom-Cook-3 unit tests (direct calls, below the public dispatch
    // threshold so we can exercise the function on small inputs too).
    // -----------------------------------------------------------------------

    /// Tiny xorshift PRNG so the test stays dependency-free.
    fn next_rand(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    fn rand_limbs(state: &mut u64, n: usize) -> Vec<u64> {
        (0..n).map(|_| next_rand(state)).collect()
    }

    /// Interpolation isolation: the hand-verified vector
    /// `a=(1,2,3) · b=(4,5,6)` (single-limb blocks, s=1) must equal
    /// `c = 4 + 13·B + 28·B² + 27·B³ + 18·B⁴`. This pins the interpolation
    /// arithmetic independently of split/recompose edge cases.
    #[test]
    fn toom3_interpolation_isolated_vector() {
        // s = 1 limb per block. a has limbs [1, 2, 3], b has [4, 5, 6].
        let a = BigUint::from_le_limbs(&[1, 2, 3]);
        let b = BigUint::from_le_limbs(&[4, 5, 6]);
        let got = mul_toom3(&a, &b);
        let want = mul_schoolbook(&a, &b);
        assert_eq!(got, want, "Toom-3 interpolation vector mismatch");
        // Also confirm the exact coefficient layout: 4,13,28,27,18 with B=2^64.
        let expect = BigUint::from_le_limbs(&[4, 13, 28, 27, 18]);
        assert_eq!(got, expect, "Toom-3 coefficient layout mismatch");
    }

    #[test]
    fn toom3_small_matches_schoolbook() {
        // Direct-call cross-val on small operands (s small).
        let mut st: u64 = 0x1234_5678_9ABC_DEF0;
        for len in 3..=40usize {
            let av = rand_limbs(&mut st, len);
            let bv = rand_limbs(&mut st, len);
            let a = BigUint::from_le_limbs(&av);
            let b = BigUint::from_le_limbs(&bv);
            assert_eq!(
                mul_toom3(&a, &b),
                mul_schoolbook(&a, &b),
                "toom3 != schoolbook at len={len}"
            );
        }
    }

    #[test]
    fn toom3_asymmetric_and_short_high_block() {
        // len(a) >> len(b): exercises empty/short high blocks of b.
        let mut st: u64 = 0xCAFE_F00D_1234_5678;
        for (la, lb) in [(120, 5), (300, 7), (200, 3), (101, 1)] {
            let a = BigUint::from_le_limbs(&rand_limbs(&mut st, la));
            let b = BigUint::from_le_limbs(&rand_limbs(&mut st, lb));
            assert_eq!(mul_toom3(&a, &b), mul(&a, &b), "toom3 asymmetric {la}x{lb}");
        }
    }

    #[test]
    fn toom3_adversarial_limb_patterns() {
        let max = vec![u64::MAX; 130];
        let a = BigUint::from_le_limbs(&max);
        let b = BigUint::from_le_limbs(&max);
        assert_eq!(mul_toom3(&a, &b), mul(&a, &b), "all-MAX");

        // Power-of-two limbs (single high bit each).
        let pow: Vec<u64> = (0..130).map(|i| 1u64 << (i % 64)).collect();
        let a = BigUint::from_le_limbs(&pow);
        assert_eq!(mul_toom3(&a, &a), mul(&a, &a), "power-of-two limbs");

        // Internal zero limbs.
        let mut z = vec![0xFFFF_FFFF_FFFF_FFFFu64; 130];
        for i in (0..130).step_by(3) {
            z[i] = 0;
        }
        let a = BigUint::from_le_limbs(&z);
        assert_eq!(mul_toom3(&a, &a), mul(&a, &a), "internal zeros");
    }

    #[test]
    fn toom3_threshold_boundary_via_dispatch() {
        // Sizes straddling TOOM3_THRESHOLD; mul() routes >=100 to Toom-3.
        let mut st: u64 = 0xABCD_1234_DEAD_BEEF;
        for la in 98..=104usize {
            for lb in 98..=104usize {
                let a = BigUint::from_le_limbs(&rand_limbs(&mut st, la));
                let b = BigUint::from_le_limbs(&rand_limbs(&mut st, lb));
                assert_eq!(
                    mul_toom3(&a, &b),
                    mul_karatsuba(&a, &b),
                    "boundary {la}x{lb}"
                );
            }
        }
    }
}
