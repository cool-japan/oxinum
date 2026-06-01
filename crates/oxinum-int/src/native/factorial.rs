//! Factorial via balanced binary product tree.
//!
//! # Algorithm
//!
//! `factorial(n)` computes n! by recursively splitting the range [1, n] at
//! the midpoint and multiplying the two halves together. With the Karatsuba
//! multiplication already in place (threshold ≈ 32 limbs), this gives
//! O(M(n) log n) time where M(n) is the cost of multiplying two n-bit
//! numbers — the same asymptotic behaviour as the prime-swing method for the
//! purposes of this crate.
//!
//! The balanced product tree avoids the O(n · M(n)) cost of naive left-to-right
//! multiplication by ensuring all partial products stay roughly the same bit
//! width at each level of the recursion.
//!
//! # Examples
//!
//! ```
//! use oxinum_int::native::{factorial, BigUint};
//!
//! assert_eq!(factorial(0), BigUint::from(1u64));
//! assert_eq!(factorial(5), BigUint::from(120u64));
//! assert_eq!(factorial(10), BigUint::from(3628800u64));
//! ```

use super::uint::BigUint;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute `n!` (n factorial) as a `BigUint`.
///
/// `factorial(0) == factorial(1) == 1`.
///
/// Uses a balanced binary product tree that achieves O(M(n) log n) time
/// with the Karatsuba multiplier already present in `BigUint::mul`.
pub fn factorial(n: u64) -> BigUint {
    if n == 0 {
        return BigUint::one();
    }
    balanced_product(1, n)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute the product of all integers in the inclusive range [lo, hi].
///
/// Recursively splits at the midpoint to keep partial products balanced in
/// bit-width, amortising Karatsuba cost over all levels.
fn balanced_product(lo: u64, hi: u64) -> BigUint {
    debug_assert!(lo <= hi, "balanced_product: lo={lo} > hi={hi}");
    if lo == hi {
        return BigUint::from(lo);
    }
    if hi == lo + 1 {
        // Two consecutive values — base case for the leaf pair.
        return BigUint::from(lo) * BigUint::from(hi);
    }
    let mid = lo + (hi - lo) / 2;
    let left = balanced_product(lo, mid);
    let right = balanced_product(mid + 1, hi);
    left * right
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn bu(n: u64) -> BigUint {
        BigUint::from(n)
    }

    #[test]
    fn factorial_base_cases() {
        assert_eq!(factorial(0), bu(1));
        assert_eq!(factorial(1), bu(1));
        assert_eq!(factorial(2), bu(2));
        assert_eq!(factorial(3), bu(6));
    }

    #[test]
    fn factorial_small_exact() {
        assert_eq!(factorial(5), bu(120));
        assert_eq!(factorial(10), bu(3_628_800));
        assert_eq!(factorial(20), bu(2_432_902_008_176_640_000u64));
    }

    #[test]
    fn factorial_cross_validate_naive() {
        // Cross-validate against a simple naive product for n in 0..=200.
        for n in 0u64..=200 {
            let via_fn = factorial(n);
            let naive: BigUint = (1..=n).fold(BigUint::one(), |acc, k| acc * BigUint::from(k));
            assert_eq!(via_fn, naive, "factorial({}) mismatch", n);
        }
    }

    #[test]
    fn factorial_100_digit_count() {
        // 100! is a 158-digit number.
        let f100 = factorial(100);
        let decimal = f100.to_string();
        assert_eq!(decimal.len(), 158, "100! should have 158 decimal digits");
    }
}
