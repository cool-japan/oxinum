//! Enumeration of rationals: Stern-Brocot tree and Farey sequences.
//!
//! The Stern-Brocot tree is an infinite binary tree whose nodes are exactly
//! the positive rationals in lowest terms.  Each node is the mediant of its
//! two ancestors in the tree.  This module exposes a path-encoding
//! (`L = false`, `R = true`) plus the inverse, and Farey sequence
//! generation in `[0, 1]` for any order `n`.
//!
//! The implementation operates on signed numerator / unsigned denominator
//! pairs (matching `RBig::from_parts`) and reuses `rbig_from_signed` from
//! `ops.rs` for safe construction.

use crate::ops::rbig_from_signed;
use crate::{IBig, RBig, UBig};
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// Stern-Brocot tree
// ---------------------------------------------------------------------------

/// Compute the Stern-Brocot path encoding for a positive rational.
///
/// Returns a sequence of `bool` turns where `false` means "go left" (the
/// rational is below the current mediant) and `true` means "go right" (the
/// rational is above).  The root of the tree is `1/1`, so passing `1/1`
/// yields the empty path.
///
/// # Errors
///
/// Returns [`OxiNumError::Parse`] when `x <= 0`.  The Stern-Brocot tree
/// only enumerates positive rationals.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{stern_brocot_path, from_stern_brocot_path, RBig, IBig, UBig};
/// // 22/7: path encodes the descent down the Stern-Brocot tree.
/// let r = RBig::from_parts(IBig::from(22), UBig::from(7u32));
/// let path = stern_brocot_path(&r).unwrap();
/// // Round-trip back to 22/7.
/// let back = from_stern_brocot_path(&path);
/// assert_eq!(back, r);
/// ```
pub fn stern_brocot_path(x: &RBig) -> OxiNumResult<Vec<bool>> {
    if x.numerator() <= &IBig::ZERO {
        return Err(OxiNumError::Parse(
            "Stern-Brocot tree only enumerates positive rationals".into(),
        ));
    }

    // Boundaries as IBig pairs.  Left starts at 0/1, right at 1/0.  We
    // never construct an `RBig` for the right boundary (denominator 0 is
    // invalid).  The current node is always the mediant of the boundaries.
    let mut left_n = IBig::ZERO;
    let mut left_d = IBig::ONE;
    let mut right_n = IBig::ONE;
    let mut right_d = IBig::ZERO;

    let target_n = x.numerator().clone();
    let target_d: IBig = x.denominator().clone().into();

    let mut path = Vec::new();
    loop {
        let mid_n = &left_n + &right_n;
        let mid_d = &left_d + &right_d;
        // Compare target_n / target_d to mid_n / mid_d via cross-multiply
        // (both denominators are positive).
        let lhs = &target_n * &mid_d;
        let rhs = &mid_n * &target_d;
        match lhs.cmp(&rhs) {
            core::cmp::Ordering::Equal => break,
            core::cmp::Ordering::Less => {
                // target < mid -> go left
                path.push(false);
                right_n = mid_n;
                right_d = mid_d;
            }
            core::cmp::Ordering::Greater => {
                // target > mid -> go right
                path.push(true);
                left_n = mid_n;
                left_d = mid_d;
            }
        }
    }

    Ok(path)
}

/// Reconstruct a positive rational from its Stern-Brocot path.
///
/// The inverse of [`stern_brocot_path`].  An empty path returns `1/1`.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{from_stern_brocot_path, RBig};
/// assert_eq!(from_stern_brocot_path(&[]), RBig::ONE);
/// ```
pub fn from_stern_brocot_path(path: &[bool]) -> RBig {
    let mut left_n = IBig::ZERO;
    let mut left_d = IBig::ONE;
    let mut right_n = IBig::ONE;
    let mut right_d = IBig::ZERO;

    for &turn in path {
        let mid_n = &left_n + &right_n;
        let mid_d = &left_d + &right_d;
        if turn {
            // R: replace left with the mediant
            left_n = mid_n;
            left_d = mid_d;
        } else {
            // L: replace right with the mediant
            right_n = mid_n;
            right_d = mid_d;
        }
    }

    // Final value is the mediant of the boundaries (the current node).
    let final_n = &left_n + &right_n;
    let final_d = &left_d + &right_d;
    rbig_from_signed(&final_n, &final_d)
}

// ---------------------------------------------------------------------------
// Farey sequences
// ---------------------------------------------------------------------------

/// Generate the order-`n` Farey sequence `F_n`.
///
/// `F_n` is the strictly-ascending sequence of all reduced fractions in
/// `[0, 1]` with denominators in `1..=n`.  The output begins with `0/1`
/// and ends with `1/1`.
///
/// Uses the standard neighbour recurrence:
/// given consecutive `a/b` and `c/d` in `F_n`, the next term `p/q` is
///
/// ```text
/// k = floor((n + b) / d)
/// p = k * c - a
/// q = k * d - b
/// ```
///
/// which is `O(|F_n|)` time and avoids sorting.
///
/// # Examples
///
/// ```
/// use oxinum_rational::{farey_sequence, RBig, IBig, UBig};
/// let f5 = farey_sequence(5);
/// assert_eq!(f5.len(), 11);
/// assert_eq!(f5[0],  RBig::ZERO);
/// assert_eq!(f5[10], RBig::ONE);
/// ```
pub fn farey_sequence(n: u64) -> Vec<RBig> {
    let mut out = Vec::new();
    if n == 0 {
        // F_0 is conventionally empty (no denominators in 1..=0).
        return out;
    }

    // Use i128 internally for the neighbour recurrence so n up to ~10^18
    // is supported without overflow on the (n+b)/d divisions.  All terms
    // are returned as RBig.
    let n_i = n as i128;
    let (mut a, mut b) = (0_i128, 1_i128);
    let (mut c, mut d) = (1_i128, n_i);

    out.push(rbig_from_pair(a, b)); // 0/1
    loop {
        out.push(rbig_from_pair(c, d));
        // We've emitted 1/1 -- recurrence is complete.
        if c == 1 && d == 1 {
            break;
        }
        let k = (n_i + b) / d;
        let next_p = k * c - a;
        let next_q = k * d - b;
        a = c;
        b = d;
        c = next_p;
        d = next_q;
    }
    out
}

fn rbig_from_pair(num: i128, den: i128) -> RBig {
    // The recurrence keeps `den > 0` for every emitted term; we assert it
    // implicitly by treating `den` as positive.
    let num_b = IBig::from(num);
    let den_b = if den >= 0 {
        UBig::from(den as u128)
    } else {
        // Should not happen for a valid Farey recurrence in [0,1], but
        // handle defensively.
        UBig::from(den.unsigned_abs())
    };
    RBig::from_parts(num_b, den_b)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i32, d: u32) -> RBig {
        RBig::from_parts(IBig::from(n), UBig::from(d))
    }

    #[test]
    fn sb_path_one_is_empty() {
        let path = stern_brocot_path(&RBig::ONE).expect("ok");
        assert!(path.is_empty());
    }

    #[test]
    fn sb_path_one_half() {
        // 1/2 is the left child of 1/1 -> one L turn
        let path = stern_brocot_path(&r(1, 2)).expect("ok");
        assert_eq!(path, vec![false]);
    }

    #[test]
    fn sb_path_two_over_one() {
        // 2/1 is the right child of 1/1 -> one R turn
        let path = stern_brocot_path(&r(2, 1)).expect("ok");
        assert_eq!(path, vec![true]);
    }

    #[test]
    fn sb_path_zero_errors() {
        assert!(stern_brocot_path(&RBig::ZERO).is_err());
    }

    #[test]
    fn sb_path_negative_errors() {
        let neg = RBig::from_parts(IBig::from(-3), UBig::from(4u32));
        assert!(stern_brocot_path(&neg).is_err());
    }

    #[test]
    fn sb_from_empty_is_one() {
        assert_eq!(from_stern_brocot_path(&[]), RBig::ONE);
    }

    #[test]
    fn sb_from_single_l() {
        assert_eq!(from_stern_brocot_path(&[false]), r(1, 2));
    }

    #[test]
    fn sb_from_single_r() {
        assert_eq!(from_stern_brocot_path(&[true]), r(2, 1));
    }

    #[test]
    fn sb_roundtrip_22_over_7() {
        let r = r(22, 7);
        let path = stern_brocot_path(&r).expect("ok");
        let back = from_stern_brocot_path(&path);
        assert_eq!(back, r);
    }

    #[test]
    fn sb_roundtrip_355_over_113() {
        let r = r(355, 113);
        let path = stern_brocot_path(&r).expect("ok");
        let back = from_stern_brocot_path(&path);
        assert_eq!(back, r);
    }

    #[test]
    fn sb_roundtrip_various() {
        let cases = [
            (1, 1u32),
            (1, 2),
            (2, 1),
            (3, 7),
            (22, 7),
            (355, 113),
            (17, 12),
            (100, 3),
            (5, 8),
        ];
        for (n, d) in cases {
            let original = r(n, d);
            let path = stern_brocot_path(&original).expect("ok");
            let back = from_stern_brocot_path(&path);
            assert_eq!(back, original, "roundtrip failed for {n}/{d}");
        }
    }

    #[test]
    fn farey_zero() {
        assert!(farey_sequence(0).is_empty());
    }

    #[test]
    fn farey_one() {
        // F_1 = [0/1, 1/1]
        let f = farey_sequence(1);
        assert_eq!(f, vec![RBig::ZERO, RBig::ONE]);
    }

    #[test]
    fn farey_five_exact() {
        // The canonical reference value used in test fixtures.
        let f5 = farey_sequence(5);
        let expected = vec![
            r(0, 1),
            r(1, 5),
            r(1, 4),
            r(1, 3),
            r(2, 5),
            r(1, 2),
            r(3, 5),
            r(2, 3),
            r(3, 4),
            r(4, 5),
            r(1, 1),
        ];
        assert_eq!(f5, expected);
    }

    #[test]
    fn farey_five_strictly_ascending() {
        let f5 = farey_sequence(5);
        for w in f5.windows(2) {
            assert!(w[0] < w[1], "Farey sequence not strictly ascending");
        }
    }

    #[test]
    fn farey_endpoints() {
        for n in 1..=8 {
            let f = farey_sequence(n);
            assert_eq!(f.first(), Some(&RBig::ZERO));
            assert_eq!(f.last(), Some(&RBig::ONE));
        }
    }

    #[test]
    fn farey_ten_length() {
        // |F_n| = 1 + sum_{k=1..n} phi(k). For n = 10 the length is 33.
        let f = farey_sequence(10);
        assert_eq!(f.len(), 33);
        for w in f.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn farey_terms_reduced() {
        let f = farey_sequence(7);
        for x in &f {
            let num = x.numerator().clone();
            let den: IBig = x.denominator().clone().into();
            // Confirm gcd(|num|, den) == 1 by re-canonicalising.
            let abs_num = if num < IBig::ZERO {
                -num.clone()
            } else {
                num.clone()
            };
            let canonical = rbig_from_signed(&abs_num, &den);
            // rbig_from_signed runs through RBig::from_parts_signed which
            // canonicalises; the result must match the original numerator
            // structure (i.e. it was already reduced).
            assert_eq!(canonical.numerator(), &abs_num);
        }
    }
}
