//! Two's-complement bitwise operations on [`BigInt`]:
//! `BitAnd`/`BitOr`/`BitXor`/`Not`/`Shl`/`Shr`.
//!
//! # Two's-complement semantics
//!
//! `BigInt` represents an infinite-precision signed integer. For bitwise
//! operations we use the mathematical two's-complement model: every value has
//! an infinite conceptual bit string. Positive numbers are padded with infinite
//! zeros on the left; negative numbers are padded with infinite ones (their
//! mathematical two's-complement of their magnitude never terminates).
//!
//! ## Algorithm overview
//!
//! For `&`, `|`, `^` on `BigInt`:
//! 1. Choose `nlimbs = max(|a|.limbs, |b|.limbs) + 1` (the extra limb absorbs
//!    the sign-extension and prevents the result from crossing back through
//!    the sign boundary for most cases).
//! 2. Convert both operands to their two's-complement limb vectors of length
//!    `nlimbs` via [`to_twos_complement`].
//! 3. Apply the operation limb-wise.
//! 4. Decode the result back to a `BigInt` via [`from_twos_complement`].
//!
//! ## `Not`
//!
//! The mathematical rule is `!x = -(x) - 1`:
//! - `!0 = -1`
//! - `!5 = -6`
//! - `!(-1) = 0`
//! - `!(-6) = 5`
//!
//! ## Arithmetic right shift
//!
//! `>>` is **arithmetic** (sign-extending) shift — it equals floor division by
//! `2^k`. For `k >= 64*limbs` the result is `0` (positive) or `-1` (negative).
//!
//! For negative `self = -m` (m > 0):
//! ```text
//! floor(-m / 2^k) = -ceil(m / 2^k) = -(((m - 1) >> k) + 1)
//! ```

use super::int::BigInt;
use super::uint::BigUint;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, Shr};
use oxinum_core::Sign;

// ---------------------------------------------------------------------------
// Two's-complement helpers
// ---------------------------------------------------------------------------

/// Convert `n` to its two's-complement representation as a `Vec<u64>` of
/// exactly `nlimbs` limbs (little-endian).
///
/// - `n >= 0`: copy `n.magnitude().as_limbs()`, zero-pad to `nlimbs`.
/// - `n < 0` (value = `-m`, m > 0): pad `m` to `nlimbs`, then negate in
///   two's complement (flip all bits, add 1 with carry).
fn to_twos_complement(n: &BigInt, nlimbs: usize) -> Vec<u64> {
    let mag = n.magnitude().as_limbs();
    let mut out = vec![0u64; nlimbs];
    // Copy the magnitude limbs (they fit because nlimbs >= mag.len()).
    let copy_len = mag.len().min(nlimbs);
    out[..copy_len].copy_from_slice(&mag[..copy_len]);

    if n.is_negative() {
        // Negate in two's complement: flip all bits then add 1.
        for limb in out.iter_mut() {
            *limb = !*limb;
        }
        // Add 1 with carry propagation.
        let mut carry: u64 = 1;
        for limb in out.iter_mut() {
            let (v, c) = limb.overflowing_add(carry);
            *limb = v;
            carry = c as u64;
            if carry == 0 {
                break;
            }
        }
        // Any remaining carry means we overflowed nlimbs; since the
        // true mathematical value is exact, this can only happen if
        // m == 0 (which is forbidden for negative BigInt by the canonical-zero
        // invariant) or if nlimbs is absurdly small.
    }
    out
}

/// Decode a two's-complement limb vector (little-endian) back to a `BigInt`.
///
/// Sign is determined by the MSB of `limbs[n-1]`. For a positive result,
/// the limbs are directly the magnitude. For a negative result, the magnitude
/// is recovered by two's-complement negation of the limb vector.
fn from_twos_complement(limbs: &[u64]) -> BigInt {
    if limbs.is_empty() {
        return BigInt::zero();
    }
    let top = limbs[limbs.len() - 1];
    if top >> 63 == 0 {
        // Non-negative: the limbs *are* the magnitude.
        BigInt::from_parts(Sign::Positive, BigUint::from_le_limbs(limbs))
    } else {
        // Negative: recover magnitude via two's-complement negation.
        let mut neg: Vec<u64> = limbs.to_vec();
        for v in neg.iter_mut() {
            *v = !*v;
        }
        let mut carry: u64 = 1;
        for v in neg.iter_mut() {
            let (nv, c) = v.overflowing_add(carry);
            *v = nv;
            carry = c as u64;
            if carry == 0 {
                break;
            }
        }
        BigInt::from_parts(Sign::Negative, BigUint::from_le_limbs(&neg))
    }
}

// ---------------------------------------------------------------------------
// Core binary-op helper
// ---------------------------------------------------------------------------

/// Apply a bitwise binary op to two `BigInt` values under two's-complement.
#[inline]
fn bigint_binop<F>(lhs: &BigInt, rhs: &BigInt, op: F) -> BigInt
where
    F: Fn(u64, u64) -> u64,
{
    let llen = lhs.magnitude().as_limbs().len();
    let rlen = rhs.magnitude().as_limbs().len();
    // Extra limb absorbs sign-extension noise for all finite pairs.
    let nlimbs = llen.max(rlen) + 1;
    let ltc = to_twos_complement(lhs, nlimbs);
    let rtc = to_twos_complement(rhs, nlimbs);
    let result: Vec<u64> = ltc
        .iter()
        .zip(rtc.iter())
        .map(|(&a, &b)| op(a, b))
        .collect();
    from_twos_complement(&result)
}

// ---------------------------------------------------------------------------
// BitAnd for BigInt
// ---------------------------------------------------------------------------

impl BitAnd<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn bitand(self, rhs: &BigInt) -> BigInt {
        bigint_binop(self, rhs, |a, b| a & b)
    }
}

impl BitAnd<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn bitand(self, rhs: BigInt) -> BigInt {
        bigint_binop(&self, &rhs, |a, b| a & b)
    }
}

impl BitAnd<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn bitand(self, rhs: &BigInt) -> BigInt {
        bigint_binop(&self, rhs, |a, b| a & b)
    }
}

impl BitAnd<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn bitand(self, rhs: BigInt) -> BigInt {
        bigint_binop(self, &rhs, |a, b| a & b)
    }
}

// ---------------------------------------------------------------------------
// BitOr for BigInt
// ---------------------------------------------------------------------------

impl BitOr<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn bitor(self, rhs: &BigInt) -> BigInt {
        bigint_binop(self, rhs, |a, b| a | b)
    }
}

impl BitOr<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn bitor(self, rhs: BigInt) -> BigInt {
        bigint_binop(&self, &rhs, |a, b| a | b)
    }
}

impl BitOr<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn bitor(self, rhs: &BigInt) -> BigInt {
        bigint_binop(&self, rhs, |a, b| a | b)
    }
}

impl BitOr<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn bitor(self, rhs: BigInt) -> BigInt {
        bigint_binop(self, &rhs, |a, b| a | b)
    }
}

// ---------------------------------------------------------------------------
// BitXor for BigInt
// ---------------------------------------------------------------------------

impl BitXor<&BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn bitxor(self, rhs: &BigInt) -> BigInt {
        bigint_binop(self, rhs, |a, b| a ^ b)
    }
}

impl BitXor<BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn bitxor(self, rhs: BigInt) -> BigInt {
        bigint_binop(&self, &rhs, |a, b| a ^ b)
    }
}

impl BitXor<&BigInt> for BigInt {
    type Output = BigInt;
    #[inline]
    fn bitxor(self, rhs: &BigInt) -> BigInt {
        bigint_binop(&self, rhs, |a, b| a ^ b)
    }
}

impl BitXor<BigInt> for &BigInt {
    type Output = BigInt;
    #[inline]
    fn bitxor(self, rhs: BigInt) -> BigInt {
        bigint_binop(self, &rhs, |a, b| a ^ b)
    }
}

// ---------------------------------------------------------------------------
// Not for BigInt  — !x = -(x) - 1
// ---------------------------------------------------------------------------

impl Not for BigInt {
    type Output = BigInt;

    fn not(self) -> BigInt {
        if self.is_negative() {
            // self = -m (m > 0); !self = m - 1 (= -(-m) - 1 = m - 1 >= 0)
            let m = self.magnitude().clone();
            let one = BigUint::one();
            match m.checked_sub(&one) {
                Some(result) => BigInt::from_parts(Sign::Positive, result),
                // m was 1 (i.e., self = -1), so m - 1 = 0.
                None => BigInt::zero(),
            }
        } else {
            // self >= 0; !self = -(self + 1) = -(self.mag + 1)
            let mag_plus_one = self.magnitude() + &BigUint::one();
            BigInt::from_parts(Sign::Negative, mag_plus_one)
        }
    }
}

impl Not for &BigInt {
    type Output = BigInt;

    #[inline]
    fn not(self) -> BigInt {
        self.clone().not()
    }
}

// ---------------------------------------------------------------------------
// Shl for BigInt — left shift: sign preserved, magnitude shifted left
// ---------------------------------------------------------------------------

impl Shl<u64> for BigInt {
    type Output = BigInt;

    fn shl(self, k: u64) -> BigInt {
        if self.is_zero() || k == 0 {
            return self;
        }
        let sign = self.sign();
        let shifted_mag = self.magnitude().shl_bits(k);
        BigInt::from_parts(sign, shifted_mag)
    }
}

impl Shl<u64> for &BigInt {
    type Output = BigInt;

    #[inline]
    fn shl(self, k: u64) -> BigInt {
        self.clone().shl(k)
    }
}

// ---------------------------------------------------------------------------
// Shr for BigInt — ARITHMETIC right shift (floor division by 2^k)
//
// For self >= 0: equivalent to BigUint shr (logical shift on magnitude).
// For self = -m (m > 0): floor(-m / 2^k) = -(((m - 1) >> k) + 1)
// ---------------------------------------------------------------------------

impl Shr<u64> for BigInt {
    type Output = BigInt;

    fn shr(self, k: u64) -> BigInt {
        if k == 0 {
            return self;
        }
        if self.is_negative() {
            let m = self.magnitude().clone();
            // (m - 1) >> k; if k >= bit_length(m - 1) the result is 0.
            let m_minus_one = m.checked_sub(&BigUint::one()).unwrap_or_else(BigUint::zero);
            let shifted = m_minus_one.shr_bits(k);
            // Result = -(shifted + 1)
            let mag = &shifted + &BigUint::one();
            BigInt::from_parts(Sign::Negative, mag)
        } else {
            // Non-negative: logical shift on magnitude.
            BigInt::from_parts(Sign::Positive, self.magnitude().shr_bits(k))
        }
    }
}

impl Shr<u64> for &BigInt {
    type Output = BigInt;

    #[inline]
    fn shr(self, k: u64) -> BigInt {
        self.clone().shr(k)
    }
}

// ---------------------------------------------------------------------------
// Internal unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn bi(v: i64) -> BigInt {
        BigInt::from(v)
    }

    fn bu(v: u64) -> BigUint {
        BigUint::from_u64(v)
    }

    #[test]
    fn not_basic() {
        assert_eq!(!bi(0), bi(-1));
        assert_eq!(!bi(5), bi(-6));
        assert_eq!(!bi(-1), bi(0));
        assert_eq!(!bi(-6), bi(5));
    }

    #[test]
    fn not_double_negation() {
        for v in [-1000i64, -1, 0, 1, 1000] {
            let n = bi(v);
            assert_eq!(!!n.clone(), n, "!!n == n failed for {v}");
        }
    }

    #[test]
    fn and_neg_one_with_ff() {
        // -1 in two's complement has all bits 1; -1 & x == x.
        let neg1 = bi(-1);
        let ff = bi(0xFF);
        assert_eq!(&neg1 & &ff, ff);
    }

    #[test]
    fn or_neg_one_is_neg_one() {
        let neg1 = bi(-1);
        let ff = bi(0xFF);
        assert_eq!(&neg1 | &ff, neg1);
    }

    #[test]
    fn xor_self_is_zero() {
        let neg1 = bi(-1);
        assert_eq!(&neg1 ^ &neg1, BigInt::zero());
        let x = bi(-12345);
        assert_eq!(&x ^ &x, BigInt::zero());
    }

    #[test]
    fn shr_negative_floor_div() {
        assert_eq!(bi(-8) >> 1u64, bi(-4));
        assert_eq!(bi(-7) >> 1u64, bi(-4));
        assert_eq!(bi(-1) >> 1u64, bi(-1));
        assert_eq!(bi(-1) >> 100u64, bi(-1));
        assert_eq!(bi(7) >> 1u64, bi(3));
    }

    #[test]
    fn shl_signed() {
        assert_eq!(bi(1) << 4u64, bi(16));
        assert_eq!(bi(-1) << 4u64, bi(-16));
        assert_eq!(bi(0) << 100u64, BigInt::zero());
    }

    #[test]
    fn to_twos_complement_positive() {
        let n = bi(5);
        let tc = to_twos_complement(&n, 2);
        assert_eq!(tc, vec![5, 0]);
    }

    #[test]
    fn to_twos_complement_negative_one() {
        let n = bi(-1);
        let tc = to_twos_complement(&n, 2);
        // -1 in two's complement: all bits 1.
        assert_eq!(tc, vec![u64::MAX, u64::MAX]);
    }

    #[test]
    fn from_twos_complement_roundtrip() {
        for v in [-1i128, -2, -128, 0, 1, 127, 1000] {
            let n = BigInt::from(v);
            let tc = to_twos_complement(&n, 4);
            let back = from_twos_complement(&tc);
            assert_eq!(back, n, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn de_morgan_and_to_or() {
        // !(a & b) == !a | !b
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
            let a = bi(av);
            let b = bi(bv);
            let lhs = !(&a & &b);
            let rhs = !a.clone() | !b.clone();
            assert_eq!(lhs, rhs, "De Morgan failed for ({av}, {bv})");
        }
    }

    #[test]
    fn de_morgan_or_to_and() {
        // !(a | b) == !a & !b
        let pairs: &[(i64, i64)] = &[(0, 0), (5, 3), (-5, 3), (5, -3), (-5, -3), (-1, 0xFF)];
        for &(av, bv) in pairs {
            let a = bi(av);
            let b = bi(bv);
            let lhs = !(&a | &b);
            let rhs = !a.clone() & !b.clone();
            assert_eq!(lhs, rhs, "De Morgan (or→and) failed for ({av}, {bv})");
        }
    }

    #[test]
    fn i128_cross_val_bitwise_and_shifts() {
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
            // Shifts with small non-negative j
            for j in 0i128..20 {
                let b = BigInt::from(j);
                assert_eq!(&a & &b, BigInt::from(i & j), "{i} & {j} mismatch");
                assert_eq!(&a | &b, BigInt::from(i | j), "{i} | {j} mismatch");
                assert_eq!(&a ^ &b, BigInt::from(i ^ j), "{i} ^ {j} mismatch");
                assert_eq!(
                    a.clone() >> (j as u64),
                    BigInt::from(i >> j),
                    "{i} >> {j} mismatch"
                );
            }
            for &j in vals {
                assert_eq!(
                    &a & &BigInt::from(j),
                    BigInt::from(i & j),
                    "{i} & {j} mismatch"
                );
                assert_eq!(
                    &a | &BigInt::from(j),
                    BigInt::from(i | j),
                    "{i} | {j} mismatch"
                );
                assert_eq!(
                    &a ^ &BigInt::from(j),
                    BigInt::from(i ^ j),
                    "{i} ^ {j} mismatch"
                );
            }
        }
    }

    #[test]
    fn xor_identity_and_complement() {
        // x ^ 0 == x
        for v in [-5i64, -1, 0, 1, 5] {
            let n = bi(v);
            assert_eq!(&n ^ &BigInt::zero(), n.clone());
            // x ^ x == 0
            assert_eq!(&n ^ &n, BigInt::zero());
            // x ^ -1 == !x  (since -1 is all-ones in two's complement)
            let all_ones = bi(-1);
            assert_eq!(&n ^ &all_ones, !n.clone());
        }
    }

    #[test]
    fn shr_positive_matches_biguint_shr() {
        let mag = BigUint::from_u64(1234567890);
        let n = BigInt::from_parts(Sign::Positive, mag.clone());
        for k in [0u64, 1, 7, 15, 31, 63, 64, 65] {
            let expected = BigInt::from_parts(Sign::Positive, mag.shr_bits(k));
            assert_eq!(n.clone() >> k, expected, "positive shr mismatch for k={k}");
        }
    }

    #[test]
    fn bu_unused() {
        // Ensure the bu() helper doesn't trigger unused-function warnings by
        // using it in at least one test.
        assert_eq!(bu(42), BigUint::from_u64(42));
    }
}

// ---------------------------------------------------------------------------
// BitAndAssign / BitOrAssign / BitXorAssign for native::BigUint
//
// The core BitAnd/BitOr/BitXor impls (4 ref-combinations each) live in
// `uint.rs` and are wired to call `simd_ops` kernels.  Here we add only the
// assign variants, keeping bitwise-op code grouped in this file.
// ---------------------------------------------------------------------------

impl BitAndAssign<BigUint> for BigUint {
    #[inline]
    fn bitand_assign(&mut self, rhs: BigUint) {
        *self = &*self & &rhs;
    }
}

impl BitAndAssign<&BigUint> for BigUint {
    #[inline]
    fn bitand_assign(&mut self, rhs: &BigUint) {
        *self = &*self & rhs;
    }
}

impl BitOrAssign<BigUint> for BigUint {
    #[inline]
    fn bitor_assign(&mut self, rhs: BigUint) {
        *self = &*self | &rhs;
    }
}

impl BitOrAssign<&BigUint> for BigUint {
    #[inline]
    fn bitor_assign(&mut self, rhs: &BigUint) {
        *self = &*self | rhs;
    }
}

impl BitXorAssign<BigUint> for BigUint {
    #[inline]
    fn bitxor_assign(&mut self, rhs: BigUint) {
        *self = &*self ^ &rhs;
    }
}

impl BitXorAssign<&BigUint> for BigUint {
    #[inline]
    fn bitxor_assign(&mut self, rhs: &BigUint) {
        *self = &*self ^ rhs;
    }
}

// ---------------------------------------------------------------------------
// Unit tests for BigUint bitwise ops
// ---------------------------------------------------------------------------

#[cfg(test)]
mod biguint_bitwise_tests {
    use super::*;

    fn bu(v: u64) -> BigUint {
        BigUint::from_u64(v)
    }

    // ------- AND -------

    #[test]
    fn and_basic() {
        assert_eq!(&bu(0b1100) & &bu(0b1010), bu(0b1000));
        assert_eq!(&bu(0xFF) & &bu(0x0F), bu(0x0F));
        assert_eq!(&bu(0) & &bu(0xFF), bu(0));
    }

    #[test]
    fn and_zero_identity() {
        let x = bu(0xDEAD_BEEF);
        assert_eq!(&x & &bu(0), bu(0), "a & 0 == 0");
    }

    #[test]
    fn and_self_identity() {
        let x = bu(0xDEAD_BEEF);
        assert_eq!(&x & &x, x.clone(), "a & a == a");
    }

    #[test]
    fn and_unequal_limbs() {
        // a has 2 limbs, b has 1 limb: result should only be 1 limb (the AND of
        // the overlap; higher limbs of a vanish because AND with 0 is 0).
        let mut limbs_a = vec![0xFFFF_FFFF_FFFF_FFFFu64, 0xFFFF_FFFF_FFFF_FFFFu64];
        let limbs_b = vec![0xAAAA_AAAA_AAAA_AAAAu64];
        let a = BigUint::from_le_limbs(&limbs_a);
        let b = BigUint::from_le_limbs(&limbs_b);
        let result = &a & &b;
        assert_eq!(result, BigUint::from_le_limbs(&limbs_b));
        // Confirm normalization: a & 0 in higher limbs yields no trailing zeros
        limbs_a[1] = 0;
        let a2 = BigUint::from_le_limbs(&limbs_a);
        let result2 = &a2 & &b;
        assert_eq!(result2, BigUint::from_le_limbs(&limbs_b));
    }

    #[test]
    fn and_assign() {
        let mut x = bu(0b1111);
        x &= bu(0b1010);
        assert_eq!(x, bu(0b1010));
    }

    // ------- OR -------

    #[test]
    fn or_basic() {
        assert_eq!(&bu(0b1100) | &bu(0b1010), bu(0b1110));
        assert_eq!(&bu(0xF0) | &bu(0x0F), bu(0xFF));
    }

    #[test]
    fn or_zero_identity() {
        let x = bu(0xDEAD_BEEF);
        assert_eq!(&x | &bu(0), x.clone(), "a | 0 == a");
    }

    #[test]
    fn or_self_identity() {
        let x = bu(0xDEAD_BEEF);
        assert_eq!(&x | &x, x.clone(), "a | a == a");
    }

    #[test]
    fn or_unequal_limbs() {
        let limbs_a = vec![0x1111_1111_1111_1111u64, 0x2222_2222_2222_2222u64];
        let limbs_b = vec![0x4444_4444_4444_4444u64];
        let a = BigUint::from_le_limbs(&limbs_a);
        let b = BigUint::from_le_limbs(&limbs_b);
        let result = &a | &b;
        // Lower limb: OR, upper limb: preserved from a
        let expected = BigUint::from_le_limbs(&[
            0x1111_1111_1111_1111u64 | 0x4444_4444_4444_4444u64,
            0x2222_2222_2222_2222u64,
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn or_assign() {
        let mut x = bu(0b1010);
        x |= bu(0b0101);
        assert_eq!(x, bu(0b1111));
    }

    // ------- XOR -------

    #[test]
    fn xor_basic() {
        assert_eq!(&bu(0b1100) ^ &bu(0b1010), bu(0b0110));
        assert_eq!(&bu(0xFF) ^ &bu(0x0F), bu(0xF0));
    }

    #[test]
    fn xor_self_is_zero() {
        let x = bu(0xDEAD_BEEF_CAFE_BABEu64);
        assert_eq!(&x ^ &x, bu(0), "a ^ a == 0");
    }

    #[test]
    fn xor_zero_identity() {
        let x = bu(0xDEAD_BEEF);
        assert_eq!(&x ^ &bu(0), x.clone(), "a ^ 0 == a");
    }

    #[test]
    fn xor_unequal_limbs() {
        let limbs_a = vec![0xFFFF_FFFF_FFFF_FFFFu64, 0xFFFF_FFFF_FFFF_FFFFu64];
        let limbs_b = vec![0xFFFF_FFFF_FFFF_FFFFu64];
        let a = BigUint::from_le_limbs(&limbs_a);
        let b = BigUint::from_le_limbs(&limbs_b);
        let result = &a ^ &b;
        // Lower limb XOR cancels to 0; upper limb is 0xFFFF... ^ 0 = 0xFFFF...
        let expected = BigUint::from_le_limbs(&[0u64, 0xFFFF_FFFF_FFFF_FFFFu64]);
        assert_eq!(result, expected);
    }

    #[test]
    fn xor_double_is_identity() {
        // a ^ b ^ b == a
        let a = BigUint::from_le_limbs(&[0xDEAD_BEEF, 0xCAFE_BABE, 0x1234_5678]);
        let b = BigUint::from_le_limbs(&[0x1111_2222, 0x3333_4444]);
        let c = &(&a ^ &b) ^ &b;
        assert_eq!(c, a);
    }

    #[test]
    fn xor_normalization() {
        // XOR of equal numbers must normalize to zero (empty limbs).
        let big = BigUint::from_le_limbs(&[
            0xAAAA_BBBB_CCCC_DDDDu64,
            0xEEEE_FFFF_0000_1111u64,
            0x2222_3333_4444_5555u64,
        ]);
        let result: BigUint = &big ^ &big;
        assert_eq!(result, BigUint::zero());
        assert!(result.is_zero());
    }

    #[test]
    fn xor_assign() {
        let mut x = bu(0b1111);
        x ^= bu(0b1010);
        assert_eq!(x, bu(0b0101));
    }

    // ------- owned variants compile and produce correct results -------

    #[test]
    fn owned_ops() {
        let a = bu(0b1110);
        let b = bu(0b1011);
        assert_eq!(a.clone() & b.clone(), bu(0b1010));
        assert_eq!(a.clone() | b.clone(), bu(0b1111));
        assert_eq!(a.clone() ^ b.clone(), bu(0b0101));
    }
}
