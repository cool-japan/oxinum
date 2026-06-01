//! Montgomery multiplication context for repeated modular arithmetic.
//!
//! A [`MontgomeryContext`] caches parameters for a fixed odd modulus `m` and
//! enables:
//! - [`MontgomeryContext::mul`]: Montgomery multiplication `REDC(a * b)`.
//! - [`MontgomeryContext::pow`]: Modular exponentiation via the Montgomery ladder.
//! - [`MontgomeryContext::to_mont`] / [`MontgomeryContext::from_mont`]:
//!   convert between normal form and Montgomery form.
//!
//! # Background
//!
//! Montgomery multiplication replaces expensive modular reductions by computing
//! `t * R^{-1} mod m` (REDC) where `R = 2^(n*64)` and `n` is the number of
//! 64-bit limbs of `m`. The key identity is:
//!
//! ```text
//! REDC(a * b) ≡ a * b * R^{-1}  (mod m)
//! ```
//!
//! To multiply `a` and `b` mod `m`:
//! 1. Convert to Montgomery form: `a̅ = REDC(a * R²)`, `b̅ = REDC(b * R²)`.
//! 2. Multiply: `c̅ = REDC(a̅ * b̅) ≡ a * b * R  (mod m)`.
//! 3. Convert back: `c = REDC(c̅) ≡ a * b  (mod m)`.
//!
//! # Precomputed values
//!
//! - `n` = number of 64-bit limbs in `m`.
//! - `R = 2^(n*64)`.
//! - `r_mod_m = R mod m` (represents `1` in Montgomery form).
//! - `r_squared = R² mod m` (used by `to_mont` to convert via a single REDC).
//! - `m_prime = (-m^{-1}) mod 2^64` (a single `u64`, used in REDC loop).
//!
//! # Modulus constraint
//!
//! The modulus must be **odd** and greater than 1. [`MontgomeryContext::new`]
//! returns [`OxiNumError::Domain`] for even moduli.

use super::div::divrem;
use super::uint::BigUint;
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// MontgomeryContext
// ---------------------------------------------------------------------------

/// Montgomery multiplication context for a fixed odd modulus.
///
/// See the module-level documentation for full background and usage.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{MontgomeryContext, BigUint};
///
/// let ctx = MontgomeryContext::new(BigUint::from_u64(7)).expect("odd modulus");
///
/// // 3 * 4 mod 7 = 12 mod 7 = 5
/// let a_mont = ctx.to_mont(&BigUint::from_u64(3));
/// let b_mont = ctx.to_mont(&BigUint::from_u64(4));
/// let c_mont = ctx.mul(&a_mont, &b_mont);
/// let c = ctx.from_mont(&c_mont);
/// assert_eq!(c, BigUint::from_u64(5));
/// ```
pub struct MontgomeryContext {
    /// The modulus `m` (always odd and > 1).
    m: BigUint,
    /// `R mod m` — the Montgomery representation of `1`.
    r_mod_m: BigUint,
    /// `R² mod m` — used by `to_mont` to convert `a → a*R mod m` in one REDC.
    r_squared: BigUint,
    /// `(-m^{-1}) mod 2^64` — the REDC loop constant (per-limb Hensel lift).
    m_prime: u64,
    /// Number of 64-bit limbs in `m` (determines `R = 2^(n*64)`).
    n: usize,
}

impl MontgomeryContext {
    /// Create a new `MontgomeryContext` for the given odd modulus.
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::DivByZero`] if `m == 0` or `m == 1`.
    /// Returns [`OxiNumError::Domain`] if `m` is even.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::{MontgomeryContext, BigUint};
    ///
    /// assert!(MontgomeryContext::new(BigUint::from_u64(7)).is_ok());
    /// assert!(MontgomeryContext::new(BigUint::from_u64(10)).is_err()); // even
    /// ```
    pub fn new(m: BigUint) -> OxiNumResult<Self> {
        if m.is_zero() || m.is_one() {
            return Err(OxiNumError::DivByZero);
        }

        let limbs = m.as_limbs();

        // Modulus must be odd: lowest bit of the lowest limb must be 1.
        if limbs[0] & 1 == 0 {
            return Err(OxiNumError::Domain(
                "Montgomery multiplication requires an odd modulus".into(),
            ));
        }

        let n = limbs.len();
        let m0 = limbs[0]; // lowest 64-bit limb of m

        // -----------------------------------------------------------------------
        // Compute m_prime = (-m0^{-1}) mod 2^64 via a 6-step Hensel lift.
        //
        // We want:  m0 * m_prime ≡ -1 (mod 2^64)
        // Equivalently: m_prime = 2^64 - (m0^{-1} mod 2^64).
        //
        // Starting from the observation that m0 * 1 ≡ 1 (mod 2) (m0 is odd),
        // each iteration doubles the number of correct bits:
        //   t_{i+1} = t_i * (2 - m0 * t_i)   (mod 2^{2^i})
        // After 6 steps we have 64 bits.
        // -----------------------------------------------------------------------
        let mut t = 1u64;
        for _ in 0..6 {
            t = t.wrapping_mul(2u64.wrapping_sub(m0.wrapping_mul(t)));
        }
        // t = m0^{-1} mod 2^64.  We need -t mod 2^64.
        let m_prime: u64 = t.wrapping_neg();

        // -----------------------------------------------------------------------
        // Compute R mod m, where R = 2^(n*64).
        // Build the BigUint [0, 0, ..., 0, 1] of n+1 limbs (value = 2^(n*64)),
        // then reduce modulo m.
        // -----------------------------------------------------------------------
        let mut r_limbs = vec![0u64; n + 1];
        r_limbs[n] = 1u64;
        let r_big = BigUint::from_le_limbs(&r_limbs);
        let (_q, r_mod_m) = divrem(&r_big, &m);

        // R² mod m = (R mod m)² mod m.
        let r_mod_m_sq = r_mod_m.clone() * r_mod_m.clone();
        let (_q2, r_squared) = divrem(&r_mod_m_sq, &m);

        Ok(MontgomeryContext {
            m,
            r_mod_m,
            r_squared,
            m_prime,
            n,
        })
    }

    // -----------------------------------------------------------------------
    // Public interface
    // -----------------------------------------------------------------------

    /// Convert `a` (in normal form) to Montgomery form: `a_mont = (a * R) mod m`.
    ///
    /// Implemented as `REDC(a * R²)` so that it costs one REDC call.
    #[inline]
    pub fn to_mont(&self, a: &BigUint) -> BigUint {
        self.mont_mul(a, &self.r_squared)
    }

    /// Convert `a_mont` (in Montgomery form) back to normal form:
    /// `a = (a_mont * R^{-1}) mod m`.
    ///
    /// Implemented as `REDC(a_mont * 1)`.
    #[inline]
    pub fn from_mont(&self, a_mont: &BigUint) -> BigUint {
        self.mont_mul(a_mont, &BigUint::one())
    }

    /// Montgomery multiplication: `(a * b * R^{-1}) mod m`.
    ///
    /// Both `a` and `b` must already be in Montgomery form (i.e., values
    /// returned by [`to_mont`](Self::to_mont) or previous calls to `mul`).
    ///
    /// To multiply two normal values `x` and `y` mod `m`:
    /// ```
    /// # use oxinum_int::native::{MontgomeryContext, BigUint};
    /// # let ctx = MontgomeryContext::new(BigUint::from_u64(7)).unwrap();
    /// # let (x, y) = (BigUint::from_u64(3), BigUint::from_u64(4));
    /// let a = ctx.to_mont(&x);
    /// let b = ctx.to_mont(&y);
    /// let c_mont = ctx.mul(&a, &b);
    /// let c = ctx.from_mont(&c_mont);   // c ≡ x * y (mod 7)
    /// assert_eq!(c, BigUint::from_u64(5));
    /// ```
    #[inline]
    pub fn mul(&self, a: &BigUint, b: &BigUint) -> BigUint {
        self.mont_mul(a, b)
    }

    /// Modular exponentiation using the Montgomery ladder: `base^exp mod m`.
    ///
    /// `base` is in normal (non-Montgomery) form. The result is also in normal
    /// form.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::{MontgomeryContext, BigUint};
    ///
    /// // 2^10 mod 13 = 1024 mod 13 = 10
    /// let ctx = MontgomeryContext::new(BigUint::from_u64(13)).expect("odd");
    /// assert_eq!(ctx.pow(&BigUint::from_u64(2), &BigUint::from_u64(10)),
    ///            BigUint::from_u64(10));
    /// ```
    pub fn pow(&self, base: &BigUint, exp: &BigUint) -> BigUint {
        if exp.is_zero() {
            return BigUint::one();
        }

        let base_mont = self.to_mont(base);
        // 1 in Montgomery form = r_mod_m (since 1 * R mod m = R mod m).
        let mut result_mont = self.r_mod_m.clone();
        let mut current_mont = base_mont;

        let bits = exp.bit_length();
        for i in 0..bits {
            if exp.test_bit(i) {
                result_mont = self.mont_mul(&result_mont, &current_mont);
            }
            if i + 1 < bits {
                current_mont = self.mont_mul(&current_mont, &current_mont);
            }
        }

        self.from_mont(&result_mont)
    }

    /// Returns a reference to the modulus.
    #[inline]
    pub fn modulus(&self) -> &BigUint {
        &self.m
    }

    // -----------------------------------------------------------------------
    // REDC — Montgomery reduction
    // -----------------------------------------------------------------------

    /// REDC: computes `(a * b * R^{-1}) mod m`.
    ///
    /// Algorithm (adapted for pure-BigUint operations):
    ///
    /// 1. `t = a * b`
    /// 2. For each limb index `i` in `0..n`:
    ///    - Extract the `i`-th 64-bit limb of `t`: `t_i = (t >> (64*i)) & (2^64-1)`.
    ///    - Compute `q_i = t_i * m_prime  (wrapping, mod 2^64)`.
    ///    - `t += q_i * m * 2^(64*i)`.
    /// 3. `t >>= n * 64`.
    /// 4. If `t >= m`: `t -= m`.
    ///
    /// This is O(n²) in BigUint operations, which is acceptable for the current
    /// implementation. A native limb-array REDC operating in O(n) additional
    /// operations is deferred to a later optimization pass.
    fn mont_mul(&self, a: &BigUint, b: &BigUint) -> BigUint {
        let mut t: BigUint = a.clone() * b.clone();

        for i in 0..self.n {
            // Extract the i-th 64-bit limb of t.
            // Strategy: shift t right by i*64 bits, then take the lowest limb.
            let shift = (i as u64) * 64;
            let t_shifted = t.shr_bits(shift);
            let t_i: u64 = match t_shifted.as_limbs() {
                [] => 0u64,
                [lo, ..] => *lo,
            };

            // q_i = t_i * m_prime  (mod 2^64, wrapping)
            let q_i: u64 = t_i.wrapping_mul(self.m_prime);

            if q_i != 0 {
                // addition = q_i * m * 2^(i*64)
                let q_big = BigUint::from_u64(q_i);
                let addition = (q_big * self.m.clone()).shl_bits(shift);
                // t += addition  (using Add which is non-negative)
                t += addition;
            }
            // Note: even when q_i == 0, the loop still moves forward because
            // the subsequent right-shift at step 3 handles the accumulated t.
        }

        // Step 3: t >>= n * 64
        t = t.shr_bits((self.n as u64) * 64);

        // Step 4: conditional subtraction to bring t into [0, m).
        if t >= self.m {
            t = t.checked_sub(&self.m).unwrap_or_else(BigUint::zero);
        }

        t
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{mod_mul, mod_pow};

    fn bu(n: u64) -> BigUint {
        BigUint::from_u64(n)
    }

    #[test]
    fn new_rejects_zero() {
        assert!(MontgomeryContext::new(bu(0)).is_err());
    }

    #[test]
    fn new_rejects_one() {
        assert!(MontgomeryContext::new(bu(1)).is_err());
    }

    #[test]
    fn new_rejects_even() {
        assert!(MontgomeryContext::new(bu(10)).is_err());
        assert!(MontgomeryContext::new(bu(2)).is_err());
        assert!(MontgomeryContext::new(bu(100)).is_err());
    }

    #[test]
    fn new_accepts_odd() {
        assert!(MontgomeryContext::new(bu(7)).is_ok());
        assert!(MontgomeryContext::new(bu(13)).is_ok());
        assert!(MontgomeryContext::new(bu(65537)).is_ok());
    }

    #[test]
    fn roundtrip_to_from_mont() {
        let ctx = MontgomeryContext::new(bu(7)).expect("odd");
        for a in 0u64..7 {
            let a_mont = ctx.to_mont(&bu(a));
            let a_back = ctx.from_mont(&a_mont);
            assert_eq!(a_back, bu(a), "roundtrip failed for a={a}");
        }
    }

    #[test]
    fn mul_basic_3x4_mod7() {
        let ctx = MontgomeryContext::new(bu(7)).expect("odd");
        let a_mont = ctx.to_mont(&bu(3));
        let b_mont = ctx.to_mont(&bu(4));
        let c_mont = ctx.mul(&a_mont, &b_mont);
        let c = ctx.from_mont(&c_mont);
        assert_eq!(c, bu(5)); // 3 * 4 = 12 ≡ 5 (mod 7)
    }

    #[test]
    fn pow_2_10_mod13() {
        let ctx = MontgomeryContext::new(bu(13)).expect("odd");
        let result = ctx.pow(&bu(2), &bu(10));
        // 2^10 = 1024 = 78*13 + 10
        assert_eq!(result, bu(10));
    }

    #[test]
    fn pow_zero_exp() {
        let ctx = MontgomeryContext::new(bu(7)).expect("odd");
        // Any base to the 0 = 1
        assert_eq!(ctx.pow(&bu(5), &bu(0)), bu(1));
    }

    #[test]
    fn fermat_little_theorem() {
        // For prime p and a not divisible by p: a^(p-1) ≡ 1 (mod p)
        for &p in &[7u64, 13, 101, 65537] {
            let ctx = MontgomeryContext::new(bu(p)).expect("odd prime");
            for a in 2u64..p.min(8) {
                let result = ctx.pow(&bu(a), &bu(p - 1));
                assert_eq!(result, bu(1), "Fermat failed for a={a}, p={p}");
            }
        }
    }

    #[test]
    fn montgomery_vs_schoolbook() {
        // For a variety of odd moduli, check that Montgomery mul == schoolbook mod_mul.
        let odd_moduli = [7u64, 13, 101, 4093, 65537];
        for &m in &odd_moduli {
            let ctx = MontgomeryContext::new(bu(m)).expect("ctx");
            for a in [0u64, 1, 2, m - 1, m / 2 + 1] {
                for b in [0u64, 1, 3, m - 1, (m / 2).saturating_add(1)] {
                    let a = a.min(m - 1);
                    let b = b.min(m - 1);
                    let expected = mod_mul(&bu(a), &bu(b), &bu(m)).expect("schoolbook");
                    let a_mont = ctx.to_mont(&bu(a));
                    let b_mont = ctx.to_mont(&bu(b));
                    let got_mont = ctx.mul(&a_mont, &b_mont);
                    let got = ctx.from_mont(&got_mont);
                    assert_eq!(
                        got, expected,
                        "Montgomery vs schoolbook mismatch: a={a}, b={b}, m={m}"
                    );
                }
            }
        }
    }

    #[test]
    fn montgomery_pow_vs_mod_pow() {
        // Cross-validate Montgomery pow against schoolbook mod_pow.
        let odd_moduli = [7u64, 13, 101, 65537];
        let exps = [0u64, 1, 2, 10, 100, 1000];
        for &m in &odd_moduli {
            let ctx = MontgomeryContext::new(bu(m)).expect("ctx");
            for a in [1u64, 2, 3, m - 1] {
                for &e in &exps {
                    let expected = mod_pow(&bu(a), &bu(e), &bu(m)).expect("mod_pow");
                    let got = ctx.pow(&bu(a), &bu(e));
                    assert_eq!(
                        got, expected,
                        "Montgomery pow vs mod_pow: a={a}, e={e}, m={m}"
                    );
                }
            }
        }
    }
}
