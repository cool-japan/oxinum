//! Binary-splitting engine for hypergeometric-like series.
//!
//! This module implements the standard "binary splitting" divide-and-conquer
//! algorithm for evaluating series of the form
//!
//! ```text
//! S = Σ_{k=lo}^{hi-1} a(k) · P(lo) · P(lo+1) · … · P(k)
//!                          / (Q(lo) · Q(lo+1) · … · Q(k)
//!                             · B(lo) · B(lo+1) · … · B(k))
//! ```
//!
//! where `P(k)`, `Q(k)`, `B(k)`, `a(k)` are integer-valued functions of `k`.
//!
//! # Algorithm
//!
//! Each recursive call returns a `BSSplit { p, q, b, t }` struct where
//! `t / (q · b)` equals the partial sum over `[lo, hi)`. The combine step is:
//!
//! ```text
//! p  = p_L · p_R
//! q  = q_L · q_R
//! b  = b_L · b_R
//! t  = t_L · q_R · b_R + t_R · p_L
//! ```
//!
//! This is `O(M(n) log n)` where `M(n)` is the cost of multiplying two n-digit
//! integers (Karatsuba in this implementation).
//!
//! # Usage
//!
//! Implement [`BSSeries`] for your series, then call [`binary_split`]:
//!
//! ```no_run
//! # use oxinum_float::native::binary_splitting::{BSSeries, BSSplit, binary_split};
//! # use oxinum_int::native::BigInt;
//! struct MySeries;
//! impl BSSeries for MySeries {
//!     fn term(&self, k: u64) -> (BigInt, BigInt, BigInt, BigInt) {
//!         (BigInt::one(), BigInt::one(), BigInt::one(), BigInt::one())
//!     }
//! }
//! let split = binary_split(&MySeries, 0, 10);
//! ```

use oxinum_int::native::BigInt;

// ---------------------------------------------------------------------------
// Public data type
// ---------------------------------------------------------------------------

/// Result of binary-splitting over a range `[lo, hi)`.
///
/// The partial sum equals `t / (q · b)`.
pub struct BSSplit {
    /// Cumulative numerator factor `P(lo) · P(lo+1) · … · P(hi-1)`.
    pub p: BigInt,
    /// Cumulative denominator factor `Q(lo) · Q(lo+1) · … · Q(hi-1)`.
    pub q: BigInt,
    /// Cumulative denominator factor `B(lo) · B(lo+1) · … · B(hi-1)`.
    pub b: BigInt,
    /// Accumulated partial-sum numerator (over shared denominator `q · b`).
    pub t: BigInt,
}

// ---------------------------------------------------------------------------
// Series trait
// ---------------------------------------------------------------------------

/// Trait that defines the per-term factors of a binary-splittable series.
///
/// For term index `k`, implementors return `(p_k, q_k, b_k, a_k)`:
///
/// * `p_k` — numerator factor at position `k`.
/// * `q_k` — denominator factor at position `k`.
/// * `b_k` — auxiliary denominator factor at position `k` (often `1`).
/// * `a_k` — coefficient / weight of the `k`-th term (can be negative).
///
/// The partial sum is then:
/// ```text
/// Σ a(k) · P(0..k) / (Q(0..k) · B(0..k))
/// ```
/// where `P(0..k) = p(0)·p(1)·…·p(k)`, etc.
pub trait BSSeries {
    /// Returns `(p_k, q_k, b_k, a_k)` for term index `k`.
    fn term(&self, k: u64) -> (BigInt, BigInt, BigInt, BigInt);
}

// ---------------------------------------------------------------------------
// Core engine
// ---------------------------------------------------------------------------

/// Evaluate `Σ_{k=lo}^{hi-1}` using binary splitting.
///
/// `hi` must be strictly greater than `lo`.
///
/// # Panics
///
/// Panics if `hi <= lo`.
pub fn binary_split<S: BSSeries>(series: &S, lo: u64, hi: u64) -> BSSplit {
    assert!(hi > lo, "binary_split: hi ({hi}) must be > lo ({lo})");

    if hi == lo + 1 {
        // Base case: single term.
        let (p, q, b, a) = series.term(lo);
        let t = &a * &p;
        return BSSplit { p, q, b, t };
    }

    let mid = lo + (hi - lo) / 2;
    let l = binary_split(series, lo, mid);
    let r = binary_split(series, mid, hi);

    // Combine:
    //   p = p_L · p_R
    //   q = q_L · q_R
    //   b = b_L · b_R
    //   t = t_L · q_R · b_R  +  t_R · p_L
    let p = &l.p * &r.p;
    let q = &l.q * &r.q;
    let b = &l.b * &r.b;
    let t = &l.t * &r.q * &r.b + &r.t * &l.p;

    BSSplit { p, q, b, t }
}

// ---------------------------------------------------------------------------
// Unit tests for the combining rule
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Simplest possible series: Σ 1 for k in [0, N).  Sum should be N.
    struct ConstantSeries;
    impl BSSeries for ConstantSeries {
        fn term(&self, _k: u64) -> (BigInt, BigInt, BigInt, BigInt) {
            (BigInt::one(), BigInt::one(), BigInt::one(), BigInt::one())
        }
    }

    #[test]
    fn constant_series_base() {
        let r = binary_split(&ConstantSeries, 0, 1);
        // t=1, q=1, b=1  =>  sum = 1/1 = 1
        assert_eq!(r.t, BigInt::one());
        assert_eq!(r.q, BigInt::one());
    }

    #[test]
    fn constant_series_n() {
        // Σ_{k=0}^{N-1} 1 = N.  sum = t/(q*b).  p_total = 1^N = 1, q = 1, b = 1.
        // With a(k)=1 and p(k)=1, t after binary split should equal N.
        for n in 2u64..=20 {
            let r = binary_split(&ConstantSeries, 0, n);
            let expected_t = BigInt::from(n as i64);
            assert_eq!(r.t, expected_t, "N={n}");
        }
    }

    /// Geometric series: Σ_{k=0}^{N-1} (1/2)^k.
    /// p(k)=1, q(k)=2, b(k)=1, a(k)=1.
    /// Result = t/(q*b).  At N terms: sum ≈ 2·(1 - 1/2^N).
    struct GeomHalf;
    impl BSSeries for GeomHalf {
        fn term(&self, _k: u64) -> (BigInt, BigInt, BigInt, BigInt) {
            (
                BigInt::one(),
                BigInt::from(2i64),
                BigInt::one(),
                BigInt::one(),
            )
        }
    }

    #[test]
    fn geometric_half_n4() {
        // With p(k)=1, q(k)=2, b(k)=1, a(k)=1 for k in 0..4:
        //   The series is Σ_{k=0}^{3} (1/q_prefix)  where q_prefix(k) = 2^(k+1).
        //   sum = 1/2 + 1/4 + 1/8 + 1/16 = 15/16.
        //
        // Binary splitting gives t/(q*b):
        //   Q = 2^4 = 16, B = 1, T = 15  →  sum = 15/16.
        let r = binary_split(&GeomHalf, 0, 4);
        let q16 = BigInt::from(16i64);
        let b1 = BigInt::one();
        assert_eq!(r.q, q16, "q should be 2^4 = 16");
        assert_eq!(r.b, b1, "b should be 1");
        assert_eq!(r.t, BigInt::from(15i64), "t should be 15");
    }
}
