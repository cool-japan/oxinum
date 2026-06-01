//! Extended GCD (Bezout coefficients) and modular inverse for [`BigUint`].
//!
//! This module provides:
//! - [`gcd_extended`]: returns `(g, x, y)` such that `a * x + b * y == g`
//!   and `g = gcd(a, b)` — the **Bezout identity**.
//! - [`mod_inv`]: modular inverse — returns `Some(x)` where `a * x ≡ 1 (mod m)`,
//!   or `None` if `gcd(a, m) != 1`.
//!
//! # Algorithm
//!
//! [`gcd_extended`] uses the classical extended Euclidean algorithm, correct
//! for all input sizes. A Lehmer acceleration (half-Lehmer with Bezout tracking)
//! can be layered on top in a future pass; for correctness at this stage the
//! classical approach is preferred.
//!
//! # Sign conventions
//!
//! The returned `g` is always `BigUint` (non-negative). The Bezout coefficients
//! `x` and `y` are signed `BigInt` values; exactly one of them may be negative
//! (or both zero for trivial inputs). The identity `a * x + b * y == g` holds
//! over the signed integers.

use super::div::divrem;
use super::int::BigInt;
use super::uint::BigUint;
use oxinum_core::Sign;

// ---------------------------------------------------------------------------
// Extended GCD
// ---------------------------------------------------------------------------

/// Returns `(g, x, y)` such that `a * x + b * y == g` where `g = gcd(a, b)`.
///
/// - `g` is always non-negative.
/// - `x` and `y` are signed Bezout coefficients satisfying the identity.
/// - If both `a` and `b` are zero, returns `(0, 1, 0)` by convention.
///
/// # Algorithm
///
/// Classical extended Euclidean. A future Lehmer-accelerated variant for
/// multi-limb inputs may be substituted without changing the API.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{gcd_extended, BigUint};
/// use oxinum_int::native::BigInt;
///
/// let (g, x, y) = gcd_extended(&BigUint::from_u64(12), &BigUint::from_u64(8));
/// assert_eq!(g, BigUint::from_u64(4));
/// // Verify Bezout: 12 * x + 8 * y == 4
/// let sum = BigInt::from(12i64) * x + BigInt::from(8i64) * y;
/// assert_eq!(sum, BigInt::from(4i64));
/// ```
pub fn gcd_extended(a: &BigUint, b: &BigUint) -> (BigUint, BigInt, BigInt) {
    // Base cases: match the contract for zero inputs.
    if b.is_zero() {
        // gcd(a, 0) = a;  a*1 + 0*0 = a
        return (a.clone(), BigInt::from(1i64), BigInt::from(0i64));
    }
    if a.is_zero() {
        // gcd(0, b) = b;  0*0 + b*1 = b
        return (b.clone(), BigInt::from(0i64), BigInt::from(1i64));
    }

    ext_gcd_classical(a.clone(), b.clone())
}

/// Classical extended Euclidean algorithm.
///
/// Maintains the invariants:
/// - `old_r * old_s_a + old_t_a * b == old_r`  (actually: `a * old_s + b * old_t == old_r`)
///
/// Loop invariant: `a_orig * old_s + b_orig * old_t == old_r`.
fn ext_gcd_classical(a: BigUint, b: BigUint) -> (BigUint, BigInt, BigInt) {
    // States: (old_r, r) and (old_s, s) and (old_t, t).
    // Invariants: a * old_s + b * old_t == old_r
    //             a * s + b * t == r
    let mut old_r: BigUint = a;
    let mut r: BigUint = b;
    let mut old_s: BigInt = BigInt::from(1i64);
    let mut s: BigInt = BigInt::from(0i64);
    let mut old_t: BigInt = BigInt::from(0i64);
    let mut t: BigInt = BigInt::from(1i64);

    while !r.is_zero() {
        let (q, rem) = divrem(&old_r, &r);
        // Convert quotient (BigUint) to BigInt for signed arithmetic
        let q_int = BigInt::from(q);

        // Update remainders: old_r, r = r, old_r - q * r
        let new_r = rem; // = old_r % r

        // Update s coefficient: old_s, s = s, old_s - q * s
        let new_s = &old_s - &q_int * &s;

        // Update t coefficient: old_t, t = t, old_t - q * t
        let new_t = &old_t - &q_int * &t;

        old_r = r;
        r = new_r;
        old_s = s;
        s = new_s;
        old_t = t;
        t = new_t;
    }

    // old_r is the GCD; (old_s, old_t) are the Bezout coefficients.
    (old_r, old_s, old_t)
}

// ---------------------------------------------------------------------------
// Modular inverse
// ---------------------------------------------------------------------------

/// Returns `Some(x)` where `a * x ≡ 1 (mod m)`, or `None` if `gcd(a, m) != 1`.
///
/// - Returns `None` if `m == 0`.
/// - Returns `None` if `a == 0` (since gcd(0, m) = m ≠ 1 for m > 1).
/// - Returns `None` if `gcd(a, m) != 1`.
/// - The returned value is always in `[0, m)`.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{mod_inv, BigUint};
///
/// // 3^{-1} mod 7 = 5 (since 3 * 5 = 15 ≡ 1 mod 7)
/// assert_eq!(mod_inv(&BigUint::from_u64(3), &BigUint::from_u64(7)),
///            Some(BigUint::from_u64(5)));
///
/// // gcd(6, 9) = 3 ≠ 1 → no inverse
/// assert_eq!(mod_inv(&BigUint::from_u64(6), &BigUint::from_u64(9)), None);
/// ```
pub fn mod_inv(a: &BigUint, m: &BigUint) -> Option<BigUint> {
    if m.is_zero() {
        return None;
    }

    let (g, x, _y) = gcd_extended(a, m);

    // Inverse exists iff gcd == 1.
    if !g.is_one() {
        return None;
    }

    // x is the Bezout coefficient for a: a * x + m * y == 1.
    // x may be negative; reduce to [0, m) via Euclidean mod.
    let m_int = BigInt::from(m.clone());

    // Euclidean (non-negative) reduction: ((x % m) + m) % m
    // BigInt's % takes the sign of the dividend (truncation), so:
    let x_mod = &x % &m_int; // in (-m, m)
    let x_positive = if x_mod.is_negative() {
        &x_mod + &m_int
    } else {
        x_mod
    };

    // Extract the magnitude; the result must be in [0, m) and non-negative.
    let (sign, mag) = x_positive.into_parts();
    debug_assert!(
        sign == Sign::Positive || mag.is_zero(),
        "mod_inv: reduced coefficient should be non-negative"
    );
    Some(mag)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn bu(n: u64) -> BigUint {
        BigUint::from_u64(n)
    }

    #[test]
    fn ext_gcd_zero_a() {
        let (g, x, y) = gcd_extended(&bu(0), &bu(7));
        assert_eq!(g, bu(7));
        // Bezout: 0*x + 7*y == 7
        let lhs = BigInt::from(0i64) * x + BigInt::from(7i64) * y;
        assert_eq!(lhs, BigInt::from(7i64));
    }

    #[test]
    fn ext_gcd_zero_b() {
        let (g, x, y) = gcd_extended(&bu(5), &bu(0));
        assert_eq!(g, bu(5));
        let lhs = BigInt::from(5i64) * x + BigInt::from(0i64) * y;
        assert_eq!(lhs, BigInt::from(5i64));
    }

    #[test]
    fn ext_gcd_both_zero() {
        let (g, x, y) = gcd_extended(&bu(0), &bu(0));
        // gcd(0,0) = 0; 0*x + 0*y == 0 trivially
        assert_eq!(g, bu(0));
        let _ = (x, y); // any values satisfy 0*x + 0*y == 0
    }

    #[test]
    fn ext_gcd_simple_12_8() {
        let (g, x, y) = gcd_extended(&bu(12), &bu(8));
        assert_eq!(g, bu(4));
        let sum = BigInt::from(12i64) * x + BigInt::from(8i64) * y;
        assert_eq!(sum, BigInt::from(4i64));
    }

    #[test]
    fn ext_gcd_coprime_35_15() {
        let (g, x, y) = gcd_extended(&bu(35), &bu(15));
        assert_eq!(g, bu(5));
        let sum = BigInt::from(35i64) * x + BigInt::from(15i64) * y;
        assert_eq!(sum, BigInt::from(5i64));
    }

    #[test]
    fn ext_gcd_identical() {
        let (g, x, y) = gcd_extended(&bu(42), &bu(42));
        assert_eq!(g, bu(42));
        let sum = BigInt::from(42i64) * x + BigInt::from(42i64) * y;
        assert_eq!(sum, BigInt::from(42i64));
    }

    #[test]
    fn ext_gcd_one_larger() {
        let (g, x, y) = gcd_extended(&bu(100), &bu(1));
        assert_eq!(g, bu(1));
        let sum = BigInt::from(100i64) * x + BigInt::from(1i64) * y;
        assert_eq!(sum, BigInt::from(1i64));
    }

    #[test]
    fn mod_inv_basic() {
        // 3^{-1} mod 7 = 5
        assert_eq!(mod_inv(&bu(3), &bu(7)), Some(bu(5)));
    }

    #[test]
    fn mod_inv_no_inverse() {
        assert_eq!(mod_inv(&bu(6), &bu(9)), None);
    }

    #[test]
    fn mod_inv_zero_modulus() {
        assert_eq!(mod_inv(&bu(3), &bu(0)), None);
    }

    #[test]
    fn mod_inv_result_in_range() {
        for m in [7u64, 13, 101, 65537] {
            for a in 1u64..m.min(20) {
                if let Some(inv) = mod_inv(&bu(a), &bu(m)) {
                    assert!(inv < bu(m), "mod_inv result >= m for a={a}, m={m}");
                }
            }
        }
    }
}
