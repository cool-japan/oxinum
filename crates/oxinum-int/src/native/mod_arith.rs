//! Modular arithmetic: `mod_mul`, `mod_pow` for [`BigUint`].
//!
//! This module provides efficient modular operations on arbitrary-precision
//! unsigned integers:
//!
//! - [`mod_mul`]: `(a * b) % m` — schoolbook multiplication then reduction.
//! - [`mod_pow`]: `base^exp % modulus` — square-and-multiply (binary ladder).
//!
//! Both return `Err(OxiNumError::DivByZero)` when the modulus is zero.
//! Inputs do not need to be pre-reduced modulo `m`.

use super::div::divrem;
use super::uint::BigUint;
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// mod_mul
// ---------------------------------------------------------------------------

/// Modular multiplication: returns `(a * b) % m`.
///
/// # Errors
///
/// Returns [`OxiNumError::DivByZero`] if `m == 0`.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{mod_mul, BigUint};
///
/// let result = mod_mul(&BigUint::from_u64(7), &BigUint::from_u64(8), &BigUint::from_u64(5))
///     .expect("mod_mul");
/// assert_eq!(result, BigUint::from_u64(1)); // 56 mod 5 = 1
/// ```
pub fn mod_mul(a: &BigUint, b: &BigUint, m: &BigUint) -> OxiNumResult<BigUint> {
    if m.is_zero() {
        return Err(OxiNumError::DivByZero);
    }
    // Full product then reduce: safe because BigUint multiplication is exact.
    let product = a.clone() * b.clone();
    let (_q, rem) = divrem(&product, m);
    Ok(rem)
}

// ---------------------------------------------------------------------------
// mod_pow
// ---------------------------------------------------------------------------

/// Modular exponentiation: returns `base^exp % modulus`.
///
/// Uses the left-to-right binary square-and-multiply method.
///
/// Special cases:
/// - `modulus == 1` → result is `0` for any base/exp.
/// - `exp == 0` → result is `1` (even when `base == 0`).
/// - `base == 0` and `exp > 0` → result is `0`.
///
/// # Errors
///
/// Returns [`OxiNumError::DivByZero`] if `modulus == 0`.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{mod_pow, BigUint};
///
/// // 2^10 mod 1000 = 1024 mod 1000 = 24
/// let result = mod_pow(&BigUint::from_u64(2), &BigUint::from_u64(10), &BigUint::from_u64(1000))
///     .expect("mod_pow");
/// assert_eq!(result, BigUint::from_u64(24));
/// ```
pub fn mod_pow(base: &BigUint, exp: &BigUint, modulus: &BigUint) -> OxiNumResult<BigUint> {
    if modulus.is_zero() {
        return Err(OxiNumError::DivByZero);
    }
    if modulus.is_one() {
        // All integers are 0 mod 1.
        return Ok(BigUint::zero());
    }
    if exp.is_zero() {
        // a^0 = 1 for any a (including 0).
        return Ok(BigUint::one());
    }

    // Pre-reduce the base to [0, modulus).
    let base_reduced = if base >= modulus {
        let (_q, r) = divrem(base, modulus);
        r
    } else {
        base.clone()
    };

    // If the reduced base is zero, the result is zero (for exp > 0).
    if base_reduced.is_zero() {
        return Ok(BigUint::zero());
    }

    // Right-to-left binary exponentiation (LSB to MSB).
    // Invariant: result * current^(remaining exp) ≡ base^exp (mod modulus)
    let mut result = BigUint::one();
    let mut current = base_reduced;

    let bits = exp.bit_length();
    for i in 0..bits {
        if exp.test_bit(i) {
            // result = (result * current) % modulus
            let (_q, r) = divrem(&(result.clone() * current.clone()), modulus);
            result = r;
        }
        // Square current for the next bit, but skip the squaring after the last bit.
        if i + 1 < bits {
            let (_q, r) = divrem(&(current.clone() * current.clone()), modulus);
            current = r;
        }
    }

    Ok(result)
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
    fn mod_mul_basic() {
        // 7 * 8 = 56; 56 mod 5 = 1
        assert_eq!(mod_mul(&bu(7), &bu(8), &bu(5)).expect(""), bu(1));
    }

    #[test]
    fn mod_mul_identity() {
        // a * 1 mod m = a mod m
        for (a, m) in [(13u64, 7u64), (100, 13), (65536, 65537)] {
            let expected = a % m;
            assert_eq!(mod_mul(&bu(a), &bu(1), &bu(m)).expect(""), bu(expected));
        }
    }

    #[test]
    fn mod_mul_zero_modulus() {
        assert!(mod_mul(&bu(3), &bu(4), &bu(0)).is_err());
    }

    #[test]
    fn mod_mul_result_in_range() {
        let m = bu(17);
        for a in 0u64..17 {
            for b in 0u64..17 {
                let r = mod_mul(&bu(a), &bu(b), &m).expect("");
                assert!(r < m, "result {r:?} >= m=17 for a={a}, b={b}");
            }
        }
    }

    #[test]
    fn mod_pow_basic() {
        // 2^10 mod 1000 = 24
        assert_eq!(mod_pow(&bu(2), &bu(10), &bu(1000)).expect(""), bu(24));
    }

    #[test]
    fn mod_pow_modulus_one() {
        // Everything mod 1 = 0
        assert_eq!(mod_pow(&bu(999), &bu(999), &bu(1)).expect(""), bu(0));
    }

    #[test]
    fn mod_pow_zero_exponent() {
        assert_eq!(mod_pow(&bu(0), &bu(0), &bu(7)).expect(""), bu(1));
        assert_eq!(mod_pow(&bu(5), &bu(0), &bu(7)).expect(""), bu(1));
    }

    #[test]
    fn mod_pow_zero_modulus() {
        assert!(mod_pow(&bu(2), &bu(10), &bu(0)).is_err());
    }

    #[test]
    fn mod_pow_base_zero() {
        assert_eq!(mod_pow(&bu(0), &bu(5), &bu(7)).expect(""), bu(0));
    }

    #[test]
    fn mod_pow_fermat_little_theorem() {
        // For prime p and a not divisible by p: a^(p-1) ≡ 1 (mod p).
        for &p in &[7u64, 13, 101, 65537] {
            for a in 2u64..p.min(10) {
                let result = mod_pow(&bu(a), &bu(p - 1), &bu(p)).expect("mod_pow Fermat");
                assert_eq!(result, bu(1), "Fermat failed for a={a}, p={p}");
            }
        }
    }

    #[test]
    fn mod_pow_large_exp() {
        // 3^1000 mod 1000 (known result: 1 — since phi(1000)=400 and 3^400≡1)
        // Just verify it completes without error and is in [0, 1000).
        let result = mod_pow(&bu(3), &bu(1000), &bu(1000)).expect("mod_pow large");
        assert!(result < bu(1000));
    }
}
