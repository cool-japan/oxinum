//! Division for [`BigUint`]: single-limb fast path + Knuth Algorithm D.
//!
//! References:
//! - Donald E. Knuth, *The Art of Computer Programming*, Vol. 2:
//!   *Seminumerical Algorithms*, §4.3.1, Algorithm D.
//!
//! The single-limb fast path handles `divisor.limbs.len() == 1` in O(n)
//! limb-by-limb long division without normalization. Multi-limb divisors
//! flow through Knuth-D with the full normalization + qhat-correction +
//! multiply-subtract-with-add-back machinery.

use super::uint::{normalize, BigUint};

/// Divide-with-remainder. Returns `(quotient, remainder)`.
///
/// # Panics
///
/// Panics if `divisor` is zero. Use [`checked_divrem`] for a non-panicking
/// variant.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::BigUint;
/// let n = BigUint::from_u64(100);
/// let d = BigUint::from_u64(7);
/// let (q, r) = oxinum_int::native::divrem(&n, &d);
/// assert_eq!(q, BigUint::from_u64(14));
/// assert_eq!(r, BigUint::from_u64(2));
/// ```
pub fn divrem(u: &BigUint, v: &BigUint) -> (BigUint, BigUint) {
    match checked_divrem(u, v) {
        Some(qr) => qr,
        None => panic!("BigUint: division by zero"),
    }
}

/// Divide-with-remainder, returning `None` on zero divisor.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::BigUint;
/// assert!(oxinum_int::native::checked_divrem(
///     &BigUint::from_u64(10),
///     &BigUint::zero()
/// ).is_none());
/// ```
pub fn checked_divrem(u: &BigUint, v: &BigUint) -> Option<(BigUint, BigUint)> {
    if v.limbs.is_empty() {
        return None;
    }
    // Trivial case: u < v => (0, u).
    if u.cmp(v) == core::cmp::Ordering::Less {
        return Some((BigUint::zero(), u.clone()));
    }
    // Single-limb divisor fast path.
    if v.limbs.len() == 1 {
        let d = v.limbs[0];
        return Some(div_single_limb(u, d));
    }
    // Sub-quadratic Burnikel-Ziegler recursive division for large divisors.
    // Knuth-D stays the base case (and the small/medium path).
    if v.limbs.len() >= NEWTON_DIV_THRESHOLD {
        return Some(div_burnikel_ziegler(u, v));
    }
    Some(div_knuth_d(u, v))
}

// ---------------------------------------------------------------------------
// Single-limb divisor fast path
// ---------------------------------------------------------------------------

/// Long division by a single non-zero `u64` divisor. O(n) limbs.
fn div_single_limb(u: &BigUint, d: u64) -> (BigUint, BigUint) {
    debug_assert!(d != 0, "single-limb divisor zero handled at caller");
    let mut q: Vec<u64> = vec![0u64; u.limbs.len()];
    let mut r: u64 = 0;
    // Walk from most-significant down to least.
    for i in (0..u.limbs.len()).rev() {
        let dividend: u128 = ((r as u128) << 64) | (u.limbs[i] as u128);
        q[i] = (dividend / (d as u128)) as u64;
        r = (dividend % (d as u128)) as u64;
    }
    normalize(&mut q);
    let rem = if r == 0 {
        BigUint::zero()
    } else {
        BigUint { limbs: vec![r] }
    };
    (BigUint { limbs: q }, rem)
}

// ---------------------------------------------------------------------------
// Knuth Algorithm D — multi-limb divisor
// ---------------------------------------------------------------------------

/// Divides `u` by `v` via Knuth Algorithm D. Precondition: `u >= v`,
/// `v.limbs.len() >= 2`. Returns `(quotient, remainder)`.
fn div_knuth_d(u: &BigUint, v: &BigUint) -> (BigUint, BigUint) {
    debug_assert!(v.limbs.len() >= 2);
    debug_assert!(u.cmp(v) != core::cmp::Ordering::Less);

    // ----- D1. Normalize ----------------------------------------------------
    // Shift so the top limb of v has its high bit set (>= 2^63).
    let shift = v.limbs[v.limbs.len() - 1].leading_zeros();
    let v_norm = v.shl_bits(shift as u64);
    // u is shifted by the SAME amount; v_norm shift may grow v by zero limbs
    // (since v's top limb was non-zero); u may grow by one limb at the top.
    let mut u_norm: Vec<u64> = if shift == 0 {
        // Still extend u by one zero limb so that u_norm has length m+n+1
        // where m+n is u.len() (i.e. u_norm.len() = u.len() + 1).
        let mut t = u.limbs.clone();
        t.push(0);
        t
    } else {
        // Manually compute (u << shift) without normalization-trimming so
        // we keep the upper limb (even if zero) and have a predictable length.
        let mut t: Vec<u64> = Vec::with_capacity(u.limbs.len() + 1);
        let mut carry: u64 = 0;
        for &limb in &u.limbs {
            t.push((limb << shift) | carry);
            carry = limb >> (64 - shift);
        }
        t.push(carry);
        t
    };
    let v_norm_limbs = v_norm.limbs.as_slice();
    debug_assert!(v_norm_limbs[v_norm_limbs.len() - 1] >= 1u64 << 63);
    debug_assert_eq!(
        v_norm_limbs.len(),
        v.limbs.len(),
        "shift cannot add a limb to v because top limb was nonzero"
    );

    let n = v_norm_limbs.len();
    debug_assert!(u_norm.len() > n);
    let m = u_norm.len() - n - 1; // u_norm has length m+n+1, quotient has m+1 limbs.

    let mut q: Vec<u64> = vec![0u64; m + 1];

    let b_u128: u128 = 1u128 << 64;

    // ----- D2/D7. Loop on j from m down to 0 --------------------------------
    for j in (0..=m).rev() {
        // ----- D3. Estimate qhat -----
        let u_hi = u_norm[j + n] as u128;
        let u_mid = u_norm[j + n - 1] as u128;
        let dividend: u128 = (u_hi << 64) | u_mid;
        let v_top = v_norm_limbs[n - 1] as u128;
        let mut qhat: u128 = dividend / v_top;
        let mut rhat: u128 = dividend % v_top;
        if qhat >= b_u128 {
            qhat = b_u128 - 1;
            // Recompute rhat consistent with the clamped qhat:
            // dividend = qhat * v_top + rhat'  =>  rhat' = dividend - qhat*v_top
            // (could overflow b_u128 so widen)
            rhat = dividend - qhat * v_top;
        }

        // Two-step qhat correction (Knuth D3 refinement).
        // Compare qhat * v[n-2]  >  b * rhat + u[j+n-2].
        let v_sub1 = v_norm_limbs[n - 2] as u128;
        let u_sub2 = u_norm[j + n - 2] as u128;
        while qhat * v_sub1 > (rhat << 64) | u_sub2 {
            qhat -= 1;
            rhat += v_top;
            if rhat >= b_u128 {
                // rhat overflowed B; correction loop is done.
                break;
            }
        }
        // After this loop qhat is correct, or at worst off by 1 (which D5 catches).

        // ----- D4. Multiply and subtract: u[j..=j+n] -= qhat * v_norm -----
        // Use u128 product accumulation; track borrow as i128.
        let qhat_u64: u64 = qhat as u64;
        let mut borrow: i128 = 0;
        let mut carry_mul: u128 = 0;
        for i in 0..n {
            let prod: u128 = (qhat_u64 as u128) * (v_norm_limbs[i] as u128) + carry_mul;
            let prod_lo: u64 = prod as u64;
            carry_mul = prod >> 64;
            // u_norm[j+i] -= prod_lo + (incoming borrow)
            let cur = u_norm[j + i] as i128;
            let diff = cur - (prod_lo as i128) - borrow;
            if diff < 0 {
                // Add B (2^64) and record borrow=1 for the next limb.
                u_norm[j + i] = (diff + (1i128 << 64)) as u64;
                borrow = 1;
            } else {
                u_norm[j + i] = diff as u64;
                borrow = 0;
            }
        }
        // Final limb at j+n: subtract leftover carry_mul + borrow.
        let cur = u_norm[j + n] as i128;
        let diff = cur - (carry_mul as i128) - borrow;
        let needs_addback: bool;
        if diff < 0 {
            // Negative result at the top limb => qhat was 1 too big.
            u_norm[j + n] = (diff + (1i128 << 64)) as u64;
            needs_addback = true;
        } else {
            u_norm[j + n] = diff as u64;
            needs_addback = false;
        }

        // ----- D5/D6. Test remainder; add-back if necessary -----
        if needs_addback {
            // q[j] = qhat - 1; add v_norm back to u[j..=j+n].
            q[j] = qhat_u64.wrapping_sub(1);
            let mut carry: u64 = 0;
            for i in 0..n {
                let (s1, c1) = u_norm[j + i].overflowing_add(v_norm_limbs[i]);
                let (s2, c2) = s1.overflowing_add(carry);
                u_norm[j + i] = s2;
                carry = (c1 as u64) | (c2 as u64);
            }
            // This carry should cancel the borrow we recorded above.
            let (s_top, _ignored) = u_norm[j + n].overflowing_add(carry);
            u_norm[j + n] = s_top;
        } else {
            q[j] = qhat_u64;
        }
    }

    // ----- D8. Unnormalize remainder ----------------------------------------
    // Remainder lives in u_norm[0..n]; shift right by `shift`.
    let mut rem_limbs: Vec<u64> = u_norm[..n].to_vec();
    normalize(&mut rem_limbs);
    let rem = BigUint { limbs: rem_limbs };
    let rem_unshifted = if shift == 0 {
        rem
    } else {
        rem.shr_bits(shift as u64)
    };

    normalize(&mut q);
    (BigUint { limbs: q }, rem_unshifted)
}

// ---------------------------------------------------------------------------
// Burnikel-Ziegler recursive division — sub-quadratic big/big path
// ---------------------------------------------------------------------------
//
// Reference:
// - Christoph Burnikel and Joachim Ziegler, *Fast Recursive Division*,
//   MPI-I-98-1-022, Max-Planck-Institut für Informatik, 1998.
//
// The recursion divides a `2n`-limb dividend by an `n`-limb divisor
// (`div_two_one`) by splitting both at the half-block boundary and performing
// two `3·(n/2)`-by-`n` sub-divisions (`div_three_two`), each of which performs
// one `n`-by-`(n/2)` sub-division (`div_two_one_halfdiv`, re-entering the
// recursion), bottoming out in Knuth Algorithm D once the block size drops to
// `BZ_DIV_LIMB_THRESHOLD` or becomes odd. An outer blocking loop chops a long
// dividend into divisor-sized blocks so very asymmetric inputs (e.g. 1000
// limbs ÷ 60 limbs) stay sub-quadratic.
//
// CORRECTNESS STRATEGY (ultrathink):
// The recursion is run on *normalized* operands (divisor top bit set) and
// yields the quotient only — the quotient is invariant under shifting both
// operands by the same amount. The exact remainder is recovered by the
// universal `correct_quotient` corrector applied to the ORIGINAL operands,
// which both (a) avoids un-shifting the remainder (a classic bug locus) and
// (b) acts as a hard safety net: an explicit two-directional `while` loop
// turns any residual estimate error into a correct answer, with a
// `debug_assert!` iteration bound that converts a gross bug into a loud
// failure rather than a silent wrong result.

/// Threshold (in divisor limbs) at or above which `checked_divrem` switches
/// from Knuth Algorithm D to Burnikel-Ziegler recursive division.
///
/// Below this, Knuth-D's lower constant factor wins; at/above it the
/// sub-quadratic recursion dominates. The name retains the historical
/// `NEWTON_DIV` label from the planning item (ND1 — "Newton's division for
/// large dividends"); the shipped algorithm is Burnikel-Ziegler, the other
/// standard sub-quadratic big/big divide, which reuses the already
/// dashu-cross-validated Knuth-D as its base case.
pub const NEWTON_DIV_THRESHOLD: usize = 50;

/// Half-block size (in limbs) at or below which the Burnikel-Ziegler
/// recursion bottoms out in Knuth Algorithm D. Chosen so the base case is
/// firmly in Knuth-D's efficient regime while keeping recursion depth modest.
const BZ_DIV_LIMB_THRESHOLD: usize = 24;

/// Build a `BigUint` from limbs (little-endian), trimming trailing zeros.
#[inline]
fn from_limbs_vec(mut limbs: Vec<u64>) -> BigUint {
    normalize(&mut limbs);
    BigUint { limbs }
}

/// Extract the half-open limb window `[lo, hi)` of `n` as a normalized
/// `BigUint`. Indices past the end of `n` contribute implicit zero limbs.
#[inline]
fn limb_window(n: &BigUint, lo: usize, hi: usize) -> BigUint {
    let len = n.limbs.len();
    if lo >= hi || lo >= len {
        return BigUint::zero();
    }
    let hi = hi.min(len);
    from_limbs_vec(n.limbs[lo..hi].to_vec())
}

/// Universal exact corrector. Given any quotient *estimate* `q_est` for
/// `u / d` (`d != 0`), return the exact `(quotient, remainder)` with
/// `u == quotient*d + remainder` and `0 <= remainder < d`.
///
/// Operates on the ORIGINAL (un-normalized) operands, so it never has to
/// un-shift a remainder. Both correction directions are true `while` loops,
/// so a wrong estimate is *fixed*, not merely nudged. Returns `None` only on
/// the impossible internal-arithmetic underflow (kept fallible for the
/// crate-wide no-`unwrap` policy).
fn correct_quotient(u: &BigUint, d: &BigUint, q_est: BigUint) -> Option<(BigUint, BigUint)> {
    let mut q = q_est;
    // prod = q * d.
    let mut prod = &q * d;
    let mut iters_down: u32 = 0;
    // Down-correction: estimate too large => q*d overshoots u.
    while prod.cmp(u) == core::cmp::Ordering::Greater {
        q = q.checked_sub(&BigUint::one())?;
        prod = prod.checked_sub(d)?;
        iters_down += 1;
    }
    debug_assert!(
        iters_down <= 8,
        "correct_quotient: down-correction ran {iters_down} iterations — estimate grossly wrong"
    );
    // Now prod <= u; remainder candidate r = u - prod >= 0.
    let mut r = u.checked_sub(&prod)?;
    let mut iters_up: u32 = 0;
    // Up-correction: estimate too small => remainder still >= d.
    while r.cmp(d) != core::cmp::Ordering::Less {
        q = &q + &BigUint::one();
        r = r.checked_sub(d)?;
        iters_up += 1;
    }
    debug_assert!(
        iters_up <= 8,
        "correct_quotient: up-correction ran {iters_up} iterations — estimate grossly wrong"
    );
    Some((q, r))
}

/// Top-level Burnikel-Ziegler driver. Precondition: `u >= v`,
/// `v.limbs.len() >= 2`. Returns `(quotient, remainder)`.
fn div_burnikel_ziegler(u: &BigUint, v: &BigUint) -> (BigUint, BigUint) {
    debug_assert!(v.limbs.len() >= 2);
    debug_assert!(u.cmp(v) != core::cmp::Ordering::Less);

    // The recursion may, in principle, fail the no-`unwrap` `?` chain (it
    // cannot for valid inputs). Fall back to the trusted Knuth-D path rather
    // than panic if that ever happens.
    match bz_divrem_inner(u, v) {
        Some(qr) => qr,
        None => div_knuth_d(u, v),
    }
}

/// Fallible inner driver: normalize, run the blocked recursion to obtain the
/// (shift-invariant) quotient, then derive the exact remainder against the
/// ORIGINAL operands via [`correct_quotient`].
fn bz_divrem_inner(u: &BigUint, v: &BigUint) -> Option<(BigUint, BigUint)> {
    // --- Normalize so the divisor's top bit is set (top limb >= 2^63). ---
    // The quotient is invariant under shifting BOTH operands left by `shift`,
    // so the recursion runs on the shifted operands and we recover the exact
    // remainder from the ORIGINAL operands below (no remainder un-shift).
    let shift = v.limbs[v.limbs.len() - 1].leading_zeros() as u64;
    let v_norm = v.shl_bits(shift);
    let u_norm = u.shl_bits(shift);
    let n = v_norm.limbs.len();
    debug_assert_eq!(
        n,
        v.limbs.len(),
        "normalization cannot grow a nonzero-top divisor"
    );

    // --- Outer blocking loop (handles asymmetric u.len() >> v.len()). ---
    // Walk `u_norm` most-significant block first in chunks of `n` limbs.
    // `rem` carries the running partial remainder (always < v_norm, so the
    // joined `rem * BASE^n + block` is at most 2n limbs and < v_norm*BASE^n,
    // which is exactly the `div_two_one` precondition).
    let u_len = u_norm.limbs.len();
    let n_blocks = u_len.div_ceil(n).max(1);

    let mut rem = BigUint::zero();
    let mut quotient = BigUint::zero();

    for block_idx in (0..n_blocks).rev() {
        let lo = block_idx * n;
        let hi = lo + n;
        let block = limb_window(&u_norm, lo, hi); // n limbs.
                                                  // dividend_chunk = rem * BASE^n + block  (<= 2n limbs).
        let dividend_chunk = &shift_limbs(&rem, n) + &block;
        let (q_block, r_block) = div_two_one(&dividend_chunk, &v_norm, n)?;
        // quotient = quotient * BASE^n + q_block.
        quotient = &shift_limbs(&quotient, n) + &q_block;
        rem = r_block;
    }

    // Recover the exact remainder against the ORIGINAL operands. This is also
    // the hard correctness net: an explicit two-directional correction loop
    // turns any residual estimate error into a correct answer.
    correct_quotient(u, v, quotient)
}

/// Shift a `BigUint` left by `k` whole limbs (multiply by `2^(64*k)`).
#[inline]
fn shift_limbs(n: &BigUint, k: usize) -> BigUint {
    if n.is_zero() || k == 0 {
        return n.clone();
    }
    let mut out: Vec<u64> = Vec::with_capacity(n.limbs.len() + k);
    out.resize(k, 0);
    out.extend_from_slice(&n.limbs);
    from_limbs_vec(out)
}

/// `2^(64*k)` as a `BigUint` (a single set limb at index `k`).
#[inline]
fn base_pow(k: usize) -> BigUint {
    let mut limbs = vec![0u64; k];
    limbs.push(1);
    from_limbs_vec(limbs)
}

/// Divide a `<= 2n`-limb dividend `a` by the `n`-limb normalized divisor `b`
/// (top bit of `b` set), where `a < b * 2^(n*64)` so the quotient fits in `n`
/// limbs. Returns `(quotient, remainder)` with `remainder < b`.
///
/// This is BZ's `D_{2n,n}`: split the divisor at the half-block boundary and
/// perform two `3·(n/2)`-by-`n` sub-divisions. Bottoms out in Knuth-D once
/// the block size is small or odd.
fn div_two_one(a: &BigUint, b: &BigUint, n: usize) -> Option<(BigUint, BigUint)> {
    debug_assert_eq!(b.limbs.len(), n);
    debug_assert!(
        b.limbs[n - 1] >= 1u64 << 63,
        "div_two_one needs a normalized divisor"
    );

    // Base case: small or odd block size -> trusted Knuth-D.
    if n <= BZ_DIV_LIMB_THRESHOLD || n % 2 != 0 {
        if a.cmp(b) == core::cmp::Ordering::Less {
            return Some((BigUint::zero(), a.clone()));
        }
        return Some(div_knuth_d(a, b));
    }

    let half = n / 2;
    // a = a_hi * BASE^n + a_lo; further split into half-blocks below.
    let a_hi = limb_window(a, n, 2 * n);

    // First sub-division: top three half-blocks of `a` ([a3,a2,a1]) by b.
    let a_lo_hi = limb_window(a, half, n); // upper half-block of a_lo
    let high_three = &shift_limbs(&a_hi, half) + &a_lo_hi; // 3 half-blocks
    let (q1, r1) = div_three_two(&high_three, b, half)?;

    // Second sub-division: ([r1, a0]) by b.
    let a_lo_lo = limb_window(a, 0, half); // lower half-block of a_lo
    let low_three = &shift_limbs(&r1, half) + &a_lo_lo;
    let (q0, r0) = div_three_two(&low_three, b, half)?;

    // quotient = q1 * BASE^half + q0.
    let q = &shift_limbs(&q1, half) + &q0;
    Some((q, r0))
}

/// BZ's `D_{3,2}` on half-blocks: divide a value `a` of (at most) three
/// `half`-limb blocks by the `n = 2*half`-limb normalized divisor `b`, where
/// the quotient fits in `half` limbs. Returns `(quotient, remainder)` with
/// `remainder < b`.
///
/// Let `BETA = 2^(half*64)`, `b = b1*BETA + b0`, `a = a_hi2*BETA + a0`.
/// Then `r = a - q*b = (a_hi2 - q*b1)*BETA + a0 - q*b0 = c*BETA + a0 - q*b0`
/// where `c = a_hi2 - q*b1`. We compute `r_hi = c*BETA + a0` and `sub = q*b0`
/// (with the initial `q`), then add `b` to `r_hi` (decrementing `q`) until
/// `r_hi >= sub`. Each decrement adds `b` to the true `a - q*b`, so the loop
/// is exact; BZ guarantees at most two iterations.
fn div_three_two(a: &BigUint, b: &BigUint, half: usize) -> Option<(BigUint, BigUint)> {
    let n = 2 * half;
    debug_assert_eq!(b.limbs.len(), n);

    // Split divisor b = b1 * BETA + b0  (each `half` limbs).
    let b1 = limb_window(b, half, n);
    let b0 = limb_window(b, 0, half);

    // Split a: a_hi2 = top two half-blocks [a2,a1]; a0 = low half-block.
    let a_hi2 = limb_window(a, half, 3 * half);
    let a0 = limb_window(a, 0, half);

    // (q, c): q = floor(a_hi2 / b1) clamped to < BETA; c = a_hi2 - q*b1.
    let beta = base_pow(half);
    let (mut q, c) = if a_hi2.cmp(&(&b1 * &beta)) != core::cmp::Ordering::Less {
        // Quotient would reach/overflow BETA: clamp q = BETA - 1.
        let q_max = beta.checked_sub(&BigUint::one())?;
        let c = a_hi2.checked_sub(&(&b1 * &q_max))?; // non-negative (a_hi2 > b1*q_max)
        (q_max, c)
    } else {
        div_two_one_halfdiv(&a_hi2, &b1, half)?
    };

    // sub = q * b0 (fixed for the loop below).
    let sub = &q * &b0;
    // r_hi = c * BETA + a0.
    let mut r_hi = &shift_limbs(&c, half) + &a0;

    // Add `b` until `r_hi >= sub` (equivalently, until the true remainder
    // a - q*b is non-negative). BZ bounds this at two iterations.
    let mut guard: u32 = 0;
    while r_hi.cmp(&sub) == core::cmp::Ordering::Less {
        q = q.checked_sub(&BigUint::one())?;
        r_hi = &r_hi + b;
        guard += 1;
        debug_assert!(guard <= 4, "div_three_two add-back exceeded the BZ bound");
    }
    let r = r_hi.checked_sub(&sub)?;
    Some((q, r))
}

/// Divide a value `a` of (at most) two `half`-limb blocks by the
/// `half`-limb normalized divisor `b1`, quotient fitting in `half` limbs.
///
/// This is the `D_{2,1}` step that `div_three_two` recurses into. It re-enters
/// the general `div_two_one` recursion (so deep inputs stay sub-quadratic),
/// with Knuth-D as the small base case.
fn div_two_one_halfdiv(a: &BigUint, b1: &BigUint, half: usize) -> Option<(BigUint, BigUint)> {
    debug_assert_eq!(b1.limbs.len(), half);
    debug_assert!(
        b1.limbs[half - 1] >= 1u64 << 63,
        "halfdiv needs a normalized divisor"
    );
    if a.cmp(b1) == core::cmp::Ordering::Less {
        return Some((BigUint::zero(), a.clone()));
    }
    div_two_one(a, b1, half)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_limb_basic() {
        let u = BigUint::from_u64(100);
        let v = BigUint::from_u64(7);
        let (q, r) = divrem(&u, &v);
        assert_eq!(q, BigUint::from_u64(14));
        assert_eq!(r, BigUint::from_u64(2));
    }

    #[test]
    fn single_limb_multilimb_dividend() {
        let u = BigUint::from_le_limbs(&[0, 1]); // 2^64
        let v = BigUint::from_u64(2);
        let (q, r) = divrem(&u, &v);
        assert_eq!(q, BigUint::from_le_limbs(&[1u64 << 63]));
        assert!(r.is_zero());
    }

    #[test]
    fn checked_div_by_zero() {
        let u = BigUint::from_u64(10);
        let v = BigUint::zero();
        assert!(checked_divrem(&u, &v).is_none());
    }

    #[test]
    fn dividend_smaller_than_divisor() {
        let u = BigUint::from_u64(7);
        let v = BigUint::from_le_limbs(&[0, 1]); // 2^64
        let (q, r) = divrem(&u, &v);
        assert!(q.is_zero());
        assert_eq!(r, u);
    }

    #[test]
    fn divisor_equals_dividend() {
        let u = BigUint::from_le_limbs(&[u64::MAX, u64::MAX, 1]);
        let (q, r) = divrem(&u, &u.clone());
        assert_eq!(q, BigUint::from_u64(1));
        assert!(r.is_zero());
    }

    #[test]
    fn knuth_d_top_already_normalized() {
        // Divisor top limb already has high bit set => shift=0.
        let v = BigUint::from_le_limbs(&[0xDEAD_BEEF_CAFE_BABEu64, 0x8000_0000_0000_0001u64]);
        let u = BigUint::from_le_limbs(&[0, 0, 0, 1]); // 2^192
        let (q, r) = divrem(&u, &v);
        // Reconstruct via mul + add.
        let back = &(&q * &v) + &r;
        assert_eq!(back, u);
        assert!(r < v);
    }

    #[test]
    fn knuth_d_max_top_limb() {
        let v = BigUint::from_le_limbs(&[1, u64::MAX]);
        let u = BigUint::from_le_limbs(&[0, 0, 0, 1]);
        let (q, r) = divrem(&u, &v);
        let back = &(&q * &v) + &r;
        assert_eq!(back, u);
        assert!(r < v);
    }

    #[test]
    fn power_of_two_divisor_matches_shr() {
        let u = BigUint::from_le_limbs(&[0xDEAD_BEEF_CAFE_BABE, 0x1234_5678_9ABC_DEF0, 0x42]);
        // Divide by 2^64 (single-limb? Yes: divisor has one nonzero limb only via shl).
        // Use a true multi-limb power of two: 2^128.
        let v = BigUint::from_le_limbs(&[0, 0, 1]); // 2^128
        let (q, r) = divrem(&u, &v);
        let expected_q = u.shr_bits(128);
        assert_eq!(q, expected_q);
        // remainder = low 128 bits of u
        let expected_r = BigUint::from_le_limbs(&[u.limbs[0], u.limbs[1]]);
        assert_eq!(r, expected_r);
    }

    // -----------------------------------------------------------------------
    // Burnikel-Ziegler internal-path tests.
    //
    // The mandated correctness oracle is the already-(dashu-)cross-validated
    // Knuth-D path. Every test below asserts the BZ path produces EXACTLY the
    // same `(quotient, remainder)` as `div_knuth_d`, and the Euclidean
    // invariant `u == q*d + r` with `0 <= r < d`. These call the internal
    // functions directly so they exercise the new path independently of
    // `NEWTON_DIV_THRESHOLD`.
    // -----------------------------------------------------------------------

    /// Deterministic xorshift64 PRNG (no extra deps).
    fn xorshift64(mut s: u64) -> u64 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    }

    /// Build a `BigUint` with exactly `n` limbs (top limb forced non-zero),
    /// driven by the rolling PRNG `state`.
    fn rand_biguint_exact(state: &mut u64, n: usize) -> BigUint {
        let mut limbs = Vec::with_capacity(n);
        for _ in 0..n {
            *state = xorshift64(*state);
            limbs.push(*state);
        }
        // Force the top limb non-zero so the value has exactly `n` limbs.
        if limbs[n - 1] == 0 {
            limbs[n - 1] = 1;
        }
        BigUint::from_le_limbs(&limbs)
    }

    /// Assert BZ == Knuth-D and the Euclidean invariant for `u / v`.
    fn assert_bz_matches_knuth(u: &BigUint, v: &BigUint) {
        let (qk, rk) = div_knuth_d(u, v);
        let (qb, rb) = div_burnikel_ziegler(u, v);
        assert_eq!(qb, qk, "quotient mismatch vs Knuth-D");
        assert_eq!(rb, rk, "remainder mismatch vs Knuth-D");
        // Euclidean invariant against the ORIGINAL operands.
        let back = &(&qb * v) + &rb;
        assert_eq!(&back, u, "u == q*v + r failed");
        assert!(rb < *v, "remainder not < divisor");
    }

    #[test]
    fn bz_corrector_fixes_bounded_estimate_errors() {
        // The corrector handles small bounded estimate errors (±1-8 iterations)
        // as produced by the BZ algorithm — not arbitrary estimates from zero.
        let u = BigUint::from_le_limbs(&[0xDEAD_BEEF, 0x1234, 0x99]);
        let d = BigUint::from_le_limbs(&[0xCAFE_BABE, 0x55]);
        let (q_true, r_true) = div_knuth_d(&u, &d);
        for delta in [0u64, 1, 2, 3, 5] {
            // Over-estimate.
            let over = &q_true + &BigUint::from_u64(delta);
            let (q1, r1) = correct_quotient(&u, &d, over).expect("corrector");
            assert_eq!(q1, q_true);
            assert_eq!(r1, r_true);
            // Under-estimate (clamped at zero).
            let under = q_true
                .checked_sub(&BigUint::from_u64(delta))
                .unwrap_or_else(BigUint::zero);
            let (q2, r2) = correct_quotient(&u, &d, under).expect("corrector");
            assert_eq!(q2, q_true);
            assert_eq!(r2, r_true);
        }
    }

    #[test]
    fn bz_corrector_exact_multiple_remainder_zero() {
        // u = q*d exactly: remainder must be zero, exercising the down/up edge.
        let d = BigUint::from_le_limbs(&[0x12345, 0x6789A, 0xBCDEF]);
        let q = BigUint::from_le_limbs(&[0xFFFF_0000, 0x1, 0xABCD]);
        let u = &q * &d;
        let (qc, rc) = correct_quotient(&u, &d, &q + &BigUint::from_u64(4)).expect("corrector");
        assert_eq!(qc, q);
        assert!(rc.is_zero());
    }

    #[test]
    fn bz_three_two_matches_knuth_small() {
        // Directly exercise div_three_two at a small even half-block size.
        let half = 2usize;
        let n = 2 * half;
        let mut state = 0x0123_4567_89AB_CDEFu64;
        // Build a normalized n-limb divisor.
        let mut blimbs = Vec::with_capacity(n);
        for _ in 0..n {
            state = xorshift64(state);
            blimbs.push(state);
        }
        blimbs[n - 1] |= 1u64 << 63; // normalize
        let b = BigUint::from_le_limbs(&blimbs);
        // Build a ~3-half-block dividend strictly below b*BETA so the quotient
        // fits in `half` limbs.
        let beta = base_pow(half);
        let upper_bound = &b * &beta; // values must be < this
        for _ in 0..50 {
            let mut alimbs = Vec::with_capacity(3 * half);
            for _ in 0..(3 * half) {
                state = xorshift64(state);
                alimbs.push(state);
            }
            let mut a = BigUint::from_le_limbs(&alimbs);
            // Reduce a below the bound.
            if a.cmp(&upper_bound) != core::cmp::Ordering::Less {
                a = &a % &upper_bound;
            }
            let (q, r) = div_three_two(&a, &b, half).expect("three_two");
            // Cross-check vs the trusted Knuth-D divrem on the same operands.
            let (qk, rk) = if a.cmp(&b) == core::cmp::Ordering::Less {
                (BigUint::zero(), a.clone())
            } else {
                div_knuth_d(&a, &b)
            };
            assert_eq!(q, qk, "three_two quotient mismatch");
            assert_eq!(r, rk, "three_two remainder mismatch");
            assert!(r < b);
        }
    }

    #[test]
    fn bz_two_one_matches_knuth_at_threshold() {
        // Exercise div_two_one right at an even block size above the base-case
        // threshold so the recursion actually splits.
        let n = (BZ_DIV_LIMB_THRESHOLD + 1) & !1; // smallest even > threshold
        let n = if n <= BZ_DIV_LIMB_THRESHOLD { n + 2 } else { n };
        let mut state = 0xFEDC_BA98_7654_3210u64;
        let mut blimbs = Vec::with_capacity(n);
        for _ in 0..n {
            state = xorshift64(state);
            blimbs.push(state);
        }
        blimbs[n - 1] |= 1u64 << 63; // normalize
        let b = BigUint::from_le_limbs(&blimbs);
        let beta_n = base_pow(n);
        let bound = &b * &beta_n;
        for _ in 0..30 {
            let mut alimbs = Vec::with_capacity(2 * n);
            for _ in 0..(2 * n) {
                state = xorshift64(state);
                alimbs.push(state);
            }
            let mut a = BigUint::from_le_limbs(&alimbs);
            if a.cmp(&bound) != core::cmp::Ordering::Less {
                a = &a % &bound;
            }
            let (q, r) = div_two_one(&a, &b, n).expect("two_one");
            let (qk, rk) = if a.cmp(&b) == core::cmp::Ordering::Less {
                (BigUint::zero(), a.clone())
            } else {
                div_knuth_d(&a, &b)
            };
            assert_eq!(q, qk, "two_one quotient mismatch");
            assert_eq!(r, rk, "two_one remainder mismatch");
            assert!(r < b);
        }
    }

    #[test]
    fn bz_random_sweep_vs_knuth() {
        // ~200 random (u, v) pairs across a range of sizes, all checked
        // exactly against Knuth-D.
        let mut state = 0xA5A5_5A5A_C3C3_3C3Cu64;
        for _ in 0..200 {
            // v length in [threshold .. threshold+12]; u length >= v length.
            state = xorshift64(state);
            let vlen = NEWTON_DIV_THRESHOLD + (state as usize % 13);
            state = xorshift64(state);
            let ulen = vlen + (state as usize % 40);
            let v = rand_biguint_exact(&mut state, vlen);
            let u = rand_biguint_exact(&mut state, ulen);
            if u.cmp(&v) == core::cmp::Ordering::Less {
                continue;
            }
            assert_bz_matches_knuth(&u, &v);
        }
    }

    #[test]
    fn bz_power_of_two_divisor() {
        // d = 2^(64*K) for K above threshold; exact shifts.
        let k = NEWTON_DIV_THRESHOLD + 3;
        let v = base_pow(k);
        let mut state = 0x1357_9BDF_2468_ACE0u64;
        let u = rand_biguint_exact(&mut state, k + 17);
        assert_bz_matches_knuth(&u, &v);
    }

    #[test]
    fn bz_divisor_one_bit_below_dividend() {
        // d just one bit below u (quotient is 1, remainder = u - d).
        let mut state = 0x2468_ACE0_1357_9BDFu64;
        let v = rand_biguint_exact(&mut state, NEWTON_DIV_THRESHOLD + 5);
        // u = v with its top bit pushed up by one bit position (still close).
        let u = &v + &v.shr_bits(1); // v <= u < 2v  => quotient 1
        assert_bz_matches_knuth(&u, &v);
    }

    #[test]
    fn bz_exact_multiple_remainder_zero_path() {
        // u = q*v exactly, above threshold: remainder must be zero.
        let mut state = 0x0F0F_F0F0_0F0F_F0F0u64;
        let v = rand_biguint_exact(&mut state, NEWTON_DIV_THRESHOLD + 7);
        let q = rand_biguint_exact(&mut state, 23);
        let u = &q * &v;
        let (qb, rb) = div_burnikel_ziegler(&u, &v);
        assert_eq!(qb, q);
        assert!(rb.is_zero());
        // And it agrees with Knuth-D.
        assert_bz_matches_knuth(&u, &v);
    }

    #[test]
    fn bz_top_limb_exactly_2_pow_63() {
        // Divisor top limb exactly 2^63 (already normalized boundary).
        let mut state = 0xDEAD_C0DE_FACE_B00Cu64;
        let mut vlimbs = Vec::with_capacity(NEWTON_DIV_THRESHOLD + 1);
        for _ in 0..(NEWTON_DIV_THRESHOLD + 1) {
            state = xorshift64(state);
            vlimbs.push(state);
        }
        vlimbs[NEWTON_DIV_THRESHOLD] = 1u64 << 63; // top limb exactly 2^63
        let v = BigUint::from_le_limbs(&vlimbs);
        let u = rand_biguint_exact(&mut state, NEWTON_DIV_THRESHOLD + 25);
        if u.cmp(&v) != core::cmp::Ordering::Less {
            assert_bz_matches_knuth(&u, &v);
        }
    }

    #[test]
    fn bz_top_limb_2_pow_63_plus_1() {
        let mut state = 0xB00C_FACE_C0DE_DEADu64;
        let mut vlimbs = Vec::with_capacity(NEWTON_DIV_THRESHOLD + 1);
        for _ in 0..(NEWTON_DIV_THRESHOLD + 1) {
            state = xorshift64(state);
            vlimbs.push(state);
        }
        vlimbs[NEWTON_DIV_THRESHOLD] = (1u64 << 63) + 1; // top limb 2^63 + 1
        let v = BigUint::from_le_limbs(&vlimbs);
        let u = rand_biguint_exact(&mut state, NEWTON_DIV_THRESHOLD + 25);
        if u.cmp(&v) != core::cmp::Ordering::Less {
            assert_bz_matches_knuth(&u, &v);
        }
    }

    #[test]
    fn bz_highly_asymmetric_1000_by_60() {
        // Stress the outer blocking loop: ~1000-limb u by ~60-limb v.
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let v = rand_biguint_exact(&mut state, 60);
        let u = rand_biguint_exact(&mut state, 1000);
        assert_bz_matches_knuth(&u, &v);
    }

    #[test]
    fn bz_dividend_just_above_threshold() {
        // Smallest interesting sizes: both u and v just above the threshold.
        let mut state = 0x1111_2222_3333_4444u64;
        let v = rand_biguint_exact(&mut state, NEWTON_DIV_THRESHOLD);
        let u = rand_biguint_exact(&mut state, NEWTON_DIV_THRESHOLD + 1);
        if u.cmp(&v) != core::cmp::Ordering::Less {
            assert_bz_matches_knuth(&u, &v);
        }
    }
}
