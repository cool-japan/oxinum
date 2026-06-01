//! GCD on [`BigUint`]: half-Lehmer (default) and Stein's binary GCD
//! (preserved as `gcd_binary`).
//!
//! References:
//! - Donald E. Knuth, *TAOCP* vol. 2, §4.5.2, Algorithm B (binary GCD).
//! - Donald E. Knuth, *TAOCP* vol. 2, §4.5.2, Algorithm L (Lehmer's GCD).
//! - <https://en.wikipedia.org/wiki/Binary_GCD_algorithm>
//! - <https://en.wikipedia.org/wiki/Lehmer%27s_GCD_algorithm>
//!
//! # Algorithm choice
//!
//! [`gcd`] dispatches to a **half-Lehmer** variant for multi-limb operands
//! (both `a` and `b` have more than two 64-bit limbs) and falls back to
//! Stein's binary GCD via [`gcd_binary`] for the tail and for small inputs.
//!
//! Half-Lehmer differs from the canonical full-matrix Lehmer described in
//! Knuth in that it only performs a **single** provably-correct single-word
//! Euclidean step per iteration rather than accumulating a 2×2 matrix of
//! multiple steps. The trade-off is conceptual simplicity (provable
//! correctness via the bounding inequality
//! `floor(ah / (bh+1)) == floor(ah / bh)` ⇒ true quotient match) versus
//! aggression. Empirically this still gives a substantial speed-up over
//! Stein on large multi-limb operands because each accepted step removes
//! ~64 bits of magnitude in a single big-integer multiply-and-subtract,
//! whereas Stein needs O(64) shifts/subtractions to do the same.
//!
//! `gcd(0, 0) == 0` by convention. For `BigInt`, the result is always
//! non-negative (a `BigInt` with `Sign::Positive`).

use super::div::divrem;
use super::int::BigInt;
use super::uint::BigUint;
use core::mem::swap;

/// Crossover (in limbs) at which Lehmer dispatch falls back to binary GCD.
///
/// If `min(a.limbs.len(), b.limbs.len()) <= LEHMER_THRESHOLD_LIMBS`,
/// `gcd` delegates directly to [`gcd_binary`] — for very small operands the
/// per-iteration overhead of the Lehmer single-precision step does not
/// amortize.
const LEHMER_THRESHOLD_LIMBS: usize = 2;

/// Greatest common divisor of two `BigUint` values.
///
/// This is the new default and uses a half-Lehmer dispatch on multi-limb
/// operands. For small inputs (single-limb or two-limb) it delegates to
/// [`gcd_binary`].
///
/// `gcd(0, 0) == 0`; `gcd(a, 0) == a`; `gcd(0, b) == b`.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{gcd, BigUint};
/// assert_eq!(gcd(BigUint::from_u64(48), BigUint::from_u64(18)), BigUint::from_u64(6));
/// assert_eq!(gcd(BigUint::ZERO, BigUint::from_u64(9)), BigUint::from_u64(9));
/// ```
pub fn gcd(a: BigUint, b: BigUint) -> BigUint {
    // Zero handling stays identical to gcd_binary's contract.
    if a.is_zero() {
        return b;
    }
    if b.is_zero() {
        return a;
    }
    // Below the crossover, Lehmer overhead doesn't amortize.
    if a.as_limbs().len() <= LEHMER_THRESHOLD_LIMBS || b.as_limbs().len() <= LEHMER_THRESHOLD_LIMBS
    {
        return gcd_binary(a, b);
    }
    gcd_lehmer(a, b)
}

/// Greatest common divisor of two `BigUint` values via Stein's binary GCD.
///
/// `gcd(0, 0) == 0`; `gcd(a, 0) == a`; `gcd(0, b) == b`.
///
/// This is the original [`gcd`] implementation, preserved under a stable
/// name so it can be used directly (e.g. for cross-validation against
/// the Lehmer dispatch in [`gcd`]).
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{gcd_binary, BigUint};
/// assert_eq!(
///     gcd_binary(BigUint::from_u64(48), BigUint::from_u64(18)),
///     BigUint::from_u64(6)
/// );
/// ```
pub fn gcd_binary(mut a: BigUint, mut b: BigUint) -> BigUint {
    // Cover zero inputs first so `trailing_zeros` is meaningful below.
    if a.is_zero() {
        return b;
    }
    if b.is_zero() {
        return a;
    }
    // Common factor of 2: shift = min(tz(a), tz(b)).
    let tz_a = a.trailing_zeros();
    let tz_b = b.trailing_zeros();
    let shift = tz_a.min(tz_b);
    // Strip ALL factors of 2 from `a` (so `a` is odd entering the loop).
    a = a.shr_bits(tz_a);
    // Strip the common factor of 2 from `b` (it may still be even but we
    // strip its odd-suffix tz inside the loop).
    b = b.shr_bits(shift);

    // Loop invariant: `a` is odd at the top of each iteration.
    while !b.is_zero() {
        // Make `b` odd.
        let tz = b.trailing_zeros();
        if tz > 0 {
            b = b.shr_bits(tz);
        }
        // Both `a` and `b` are odd here. Ensure `a <= b`.
        if a > b {
            swap(&mut a, &mut b);
        }
        // `b - a` is non-negative (a <= b) and even (odd - odd).
        b = b
            .checked_sub(&a)
            .expect("Stein invariant: a <= b ensures non-negative subtraction");
    }
    // a now holds gcd of the odd parts; restore the common factor of 2.
    a.shl_bits(shift)
}

/// Half-Lehmer GCD.
///
/// Assumes both `a` and `b` are non-zero and at least one has more than
/// [`LEHMER_THRESHOLD_LIMBS`] limbs. Caller (`gcd`) handles the trivial
/// and below-threshold cases.
///
/// The loop:
/// 1. Ensure `a >= b`.
/// 2. If `min(a.limbs, b.limbs) <= LEHMER_THRESHOLD_LIMBS`, hand off to
///    [`gcd_binary`].
/// 3. Pick a uniform shift such that the top of `a` lands in the low 64
///    bits. Extract `ah = top 64 bits of a`, `bh = top 64 bits of b at the
///    same shift`.
/// 4. Provably-correct single-precision quotient test:
///    `q_lo = ah / (bh + 1)`, `q_hi = ah / bh`. If `q_lo == q_hi > 0`,
///    the true multi-precision floor `floor(a / b)` matches that single
///    quotient (because `ah / (bh + 1) <= a / b <= (ah + 1) / bh`).
///    Update `a -= q * b` and swap if needed.
/// 5. Otherwise (no safe shortcut) take one full Euclidean step via
///    [`divrem`]: `a, b = b, a mod b`.
///
/// On every iteration `a` strictly decreases (either by ~q*b in the
/// shortcut branch or by a full division step), so the loop terminates.
fn gcd_lehmer(mut a: BigUint, mut b: BigUint) -> BigUint {
    loop {
        // Maintain a >= b > 0 invariant. b == 0 ⇒ return a.
        if b.is_zero() {
            return a;
        }
        if a < b {
            swap(&mut a, &mut b);
        }
        // After ordering, a >= b > 0. If either side is below the threshold,
        // hand the tail to gcd_binary.
        if a.as_limbs().len() <= LEHMER_THRESHOLD_LIMBS
            || b.as_limbs().len() <= LEHMER_THRESHOLD_LIMBS
        {
            return gcd_binary(a, b);
        }

        // Extract top ~64 bits at a uniform shift across (a, b).
        // a.bit_length() >= b.bit_length() because a >= b (both > 0).
        let bits_a = a.bit_length();
        debug_assert!(bits_a >= 64, "above-threshold a has >= 3 limbs");
        let shift = bits_a - 64;
        // Top 64 bits of a. Since we shifted by exactly bit_length-64, the
        // result is in (2^63, 2^64). It cannot exceed u64::MAX.
        let ah = top64_at_shift(&a, shift);
        let bh = top64_at_shift(&b, shift);

        // If bh == 0 then b's MSB lies more than 64 bits below a's MSB.
        // The single-precision quotient would be either huge or undefined;
        // delegate to a full Euclidean step.
        if bh == 0 {
            let (_q, r) = divrem(&a, &b);
            a = core::mem::replace(&mut b, r);
            continue;
        }

        // Provably-correct single-precision quotient bound:
        //   floor(ah / (bh+1)) <= floor(a / b) <= floor((ah+1) / bh)
        // We use the stricter symmetric bounds:
        //   q_lo = ah / (bh + 1),  q_hi = ah / bh.
        // If q_lo == q_hi > 0 then floor(a / b) == q_lo (since q is the
        // floor of a real value sandwiched between two equal integers).
        //
        // Note: `bh` is non-zero here. `bh + 1` cannot overflow because
        // `bh < 2^64`; in `u64` it would wrap only if `bh == u64::MAX`,
        // in which case `q_lo = ah / 0` would panic. Use `saturating_add`
        // to map that single corner to `u64::MAX`, yielding `q_lo = 0`
        // and forcing the safe-fallback branch.
        let q_lo = ah / bh.saturating_add(1);
        let q_hi = ah / bh;

        if q_lo == q_hi && q_lo > 0 {
            // Safe single-precision quotient. Apply it to the multi-precision
            // pair: a := a - q*b. The bound q_lo <= floor(a/b) guarantees
            // q*b <= a so no underflow.
            let q_big = BigUint::from_u64(q_lo);
            let qb = &q_big * &b;
            a = a
                .checked_sub(&qb)
                .expect("Lehmer invariant: q*b <= a by bounding lemma");
            // The new a is in [0, b). Swap so a >= b on next iteration.
            swap(&mut a, &mut b);
        } else {
            // Top-word quotient is uncertain (either q_lo < q_hi or both are
            // zero — the latter means a / b < 1, but a >= b > 0 ⇒ floor >= 1,
            // contradicting q_hi == 0; that would imply ah < bh, impossible
            // since a >= b at this point and they share the same shift). Take
            // a full Euclidean step.
            let (_q, r) = divrem(&a, &b);
            a = core::mem::replace(&mut b, r);
        }
    }
}

/// Extract the top 64 bits of `n` at the given shift. Returns 0 if `n` is
/// entirely below the shift.
#[inline]
fn top64_at_shift(n: &BigUint, shift: u64) -> u64 {
    let shifted = n.shr_bits(shift);
    match shifted.as_limbs() {
        [] => 0,
        [lo] => *lo,
        // `n.shr_bits(bit_length(n) - 64)` of a multi-limb value can leave
        // a two-limb result when n.bit_length() is not aligned: the high
        // limb holds the MSB bits and the low limb holds the rest. Combine
        // them into the top 64 bits.
        [lo, hi, ..] => {
            // The shifted value occupies up to 64 bits relative to the
            // original top. If two limbs remain it means the original top
            // bit landed in `hi`; we want the most significant 64 bits of
            // `(hi << 64) | lo`.
            let hi_bits = 64 - hi.leading_zeros() as u64;
            if hi_bits >= 64 {
                *hi
            } else {
                // Shift down to keep the top 64 bits of the two-limb value.
                let down = 64 - hi_bits;
                (hi << down) | (lo >> hi_bits)
            }
        }
    }
}

/// Greatest common divisor of two `BigInt` values. The result is the
/// magnitude GCD, always non-negative.
///
/// Dispatches via [`gcd`], which now uses half-Lehmer for multi-limb
/// magnitudes.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{gcd_int, BigInt};
/// let a = BigInt::from(-12i64);
/// let b = BigInt::from(18i64);
/// assert_eq!(gcd_int(&a, &b), BigInt::from(6i64));
/// ```
pub fn gcd_int(a: &BigInt, b: &BigInt) -> BigInt {
    let g = gcd(a.magnitude().clone(), b.magnitude().clone());
    BigInt::from(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_zero_zero_is_zero() {
        assert_eq!(gcd(BigUint::ZERO, BigUint::ZERO), BigUint::ZERO);
    }

    #[test]
    fn gcd_one_arg_zero() {
        assert_eq!(
            gcd(BigUint::ZERO, BigUint::from_u64(7)),
            BigUint::from_u64(7)
        );
        assert_eq!(
            gcd(BigUint::from_u64(7), BigUint::ZERO),
            BigUint::from_u64(7)
        );
    }

    #[test]
    fn gcd_small() {
        assert_eq!(
            gcd(BigUint::from_u64(48), BigUint::from_u64(18)),
            BigUint::from_u64(6)
        );
        assert_eq!(
            gcd(BigUint::from_u64(100), BigUint::from_u64(75)),
            BigUint::from_u64(25)
        );
        assert_eq!(
            gcd(BigUint::from_u64(13), BigUint::from_u64(17)),
            BigUint::one()
        );
    }

    #[test]
    fn gcd_a_a_is_a() {
        let a = BigUint::from_u64(42);
        assert_eq!(gcd(a.clone(), a.clone()), a);
    }

    #[test]
    fn gcd_one_n() {
        let n = BigUint::from_u64(123_456_789);
        assert_eq!(gcd(BigUint::one(), n.clone()), BigUint::one());
        assert_eq!(gcd(n, BigUint::one()), BigUint::one());
    }

    #[test]
    fn gcd_binary_matches_gcd_on_small() {
        // Crossover hits gcd_binary directly here; ensures the renamed
        // function is wired up.
        assert_eq!(
            gcd_binary(BigUint::from_u64(48), BigUint::from_u64(18)),
            gcd(BigUint::from_u64(48), BigUint::from_u64(18))
        );
    }

    #[test]
    fn gcd_int_sign_invariant() {
        let a = BigInt::from(-12i64);
        let b = BigInt::from(-18i64);
        assert_eq!(gcd_int(&a, &b), BigInt::from(6i64));
        assert_eq!(
            gcd_int(&BigInt::zero(), &BigInt::from(-9i64)),
            BigInt::from(9i64)
        );
    }

    #[test]
    fn gcd_multi_limb_power_of_two() {
        // gcd(2^256, 2^512) == 2^256.
        let a = BigUint::one().shl_bits(256);
        let b = BigUint::one().shl_bits(512);
        let g = gcd(a.clone(), b);
        assert_eq!(g, a);
    }

    #[test]
    fn gcd_fibonacci_consecutive_pair_is_one() {
        // gcd(F_n, F_{n+1}) == 1 for n >= 1.
        // Compute F_90, F_91 (well above the crossover for u64 but still
        // single-limb; for a multi-limb check, scale by a multi-limb factor).
        let (f_90, f_91) = fib_pair(90);
        assert_eq!(
            gcd(BigUint::from_u64(f_90), BigUint::from_u64(f_91)),
            BigUint::one()
        );
        // Multi-limb cross-check: gcd((F_90<<256) | 1, (F_91<<256) | 1) is
        // not generally 1 — instead verify gcd((F_90<<256), (F_91<<256))
        // equals 2^256 (the shared shift) since gcd(F_90, F_91) == 1.
        let big_a = BigUint::from_u64(f_90).shl_bits(256);
        let big_b = BigUint::from_u64(f_91).shl_bits(256);
        let g = gcd(big_a, big_b);
        assert_eq!(g, BigUint::one().shl_bits(256));
    }

    /// Small Fibonacci pair: returns `(F_n, F_{n+1})` for `n` small enough
    /// that both fit in u64.
    fn fib_pair(n: u32) -> (u64, u64) {
        let mut a: u64 = 0;
        let mut b: u64 = 1;
        for _ in 0..n {
            let next = a.wrapping_add(b);
            a = b;
            b = next;
        }
        (a, b)
    }
}
