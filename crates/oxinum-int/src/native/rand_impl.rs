//! Random generation for native [`BigUint`] and [`BigInt`].
//!
//! All items in this module are gated behind the `rand` feature flag.
//! Use `#[cfg(feature = "rand")]` in dependent code.
//!
//! # Examples
//!
//! ```
//! # #[cfg(feature = "rand")]
//! # {
//! use oxinum_int::native::{BigUint, BigInt};
//! use rand::SeedableRng;
//!
//! let mut rng = rand::rngs::StdRng::seed_from_u64(42);
//! let val = BigUint::random_bits(&mut rng, 128);
//! assert!(val.bit_length() <= 128);
//!
//! let low  = BigUint::from(100u64);
//! let high = BigUint::from(200u64);
//! let sample = BigUint::random_in_range(&mut rng, &low, &high);
//! assert!(sample >= low && sample < high);
//! # }
//! ```

use rand::distr::{Distribution, StandardUniform};
use rand::Rng;

use super::{BigInt, BigUint};
use oxinum_core::Sign;

// ---------------------------------------------------------------------------
// BigUint random generation
// ---------------------------------------------------------------------------

impl BigUint {
    /// Returns a uniformly random `BigUint` in `[0, 2^n_bits)`.
    ///
    /// When `n_bits == 0` this always returns zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rand")]
    /// # {
    /// use oxinum_int::native::BigUint;
    /// use rand::SeedableRng;
    /// let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    /// let v = BigUint::random_bits(&mut rng, 64);
    /// assert!(v.bit_length() <= 64);
    /// # }
    /// ```
    pub fn random_bits<R: Rng + ?Sized>(rng: &mut R, n_bits: u64) -> Self {
        if n_bits == 0 {
            return BigUint::zero();
        }
        let n_limbs = n_bits.div_ceil(64);
        let mut limbs = vec![0u64; n_limbs as usize];
        for limb in limbs.iter_mut() {
            *limb = rng.next_u64();
        }
        // Mask the top limb so the result is strictly < 2^n_bits.
        let top_bits = n_bits % 64;
        if top_bits != 0 {
            let mask = (1u64 << top_bits).wrapping_sub(1);
            if let Some(last) = limbs.last_mut() {
                *last &= mask;
            }
        }
        BigUint::from_le_limbs(&limbs)
    }

    /// Returns a uniformly random `BigUint` drawn from `[low, high)`.
    ///
    /// Uses rejection sampling; the expected number of draws is less than 2.
    ///
    /// # Panics
    ///
    /// Panics if `high <= low`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rand")]
    /// # {
    /// use oxinum_int::native::BigUint;
    /// use rand::SeedableRng;
    /// let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    /// let low  = BigUint::from(10u64);
    /// let high = BigUint::from(20u64);
    /// let v = BigUint::random_in_range(&mut rng, &low, &high);
    /// assert!(v >= low && v < high);
    /// # }
    /// ```
    pub fn random_in_range<R: Rng + ?Sized>(rng: &mut R, low: &BigUint, high: &BigUint) -> Self {
        assert!(
            high > low,
            "random_in_range: high must be strictly greater than low"
        );
        // range = high - low  (guaranteed non-zero since high > low)
        let range = high
            .checked_sub(low)
            .expect("random_in_range: high > low guarantees non-underflow");
        let n_bits = range.bit_length();
        loop {
            let candidate = BigUint::random_bits(rng, n_bits);
            if candidate < range {
                return low.clone() + candidate;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BigInt random generation
// ---------------------------------------------------------------------------

impl BigInt {
    /// Returns a uniformly random `BigInt` drawn from `[low, high)`.
    ///
    /// Works correctly across negative and positive ranges.
    ///
    /// # Panics
    ///
    /// Panics if `high <= low`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rand")]
    /// # {
    /// use oxinum_int::native::BigInt;
    /// use rand::SeedableRng;
    /// let mut rng = rand::rngs::StdRng::seed_from_u64(2);
    /// let low  = BigInt::from(-50i64);
    /// let high = BigInt::from( 50i64);
    /// let v = BigInt::random_in_range(&mut rng, &low, &high);
    /// assert!(v >= low && v < high);
    /// # }
    /// ```
    pub fn random_in_range<R: Rng + ?Sized>(rng: &mut R, low: &BigInt, high: &BigInt) -> Self {
        assert!(
            high > low,
            "random_in_range: high must be strictly greater than low"
        );
        // Compute the range magnitude as a BigUint (positive since high > low).
        let diff = high.clone() - low.clone();
        let (_, range) = diff.into_parts();
        let n_bits = range.bit_length();
        loop {
            let offset = BigUint::random_bits(rng, n_bits);
            if offset < range {
                let offset_int = BigInt::from_parts(Sign::Positive, offset);
                return low.clone() + offset_int;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Distribution impls
// ---------------------------------------------------------------------------

/// A [`Distribution`] that samples a random `BigUint` with exactly `n_bits`
/// of precision (uniformly in `[0, 2^n_bits)`).
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "rand")]
/// # {
/// use oxinum_int::native::rand_impl::BigUintBits;
/// use rand::distr::Distribution;
/// use rand::SeedableRng;
/// let mut rng = rand::rngs::StdRng::seed_from_u64(99);
/// let v: oxinum_int::native::BigUint = BigUintBits(128).sample(&mut rng);
/// assert!(v.bit_length() <= 128);
/// # }
/// ```
pub struct BigUintBits(pub u64);

impl Distribution<BigUint> for BigUintBits {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BigUint {
        BigUint::random_bits(rng, self.0)
    }
}

/// Implements the standard-uniform distribution for `BigUint`.
///
/// Sampling via [`StandardUniform`] generates a random 256-bit value
/// (uniformly in `[0, 2^256)`). For a different bit width use [`BigUintBits`].
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "rand")]
/// # {
/// use rand::distr::Distribution;
/// use rand::distr::StandardUniform;
/// use rand::SeedableRng;
/// use oxinum_int::native::BigUint;
/// let mut rng = rand::rngs::StdRng::seed_from_u64(7);
/// let v: BigUint = StandardUniform.sample(&mut rng);
/// assert!(v.bit_length() <= 256);
/// # }
/// ```
impl Distribution<BigUint> for StandardUniform {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BigUint {
        BigUint::random_bits(rng, 256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn random_bits_zero_bits_is_zero() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let v = BigUint::random_bits(&mut rng, 0);
        assert!(v.is_zero());
    }

    #[test]
    fn random_bits_single_limb_bounded() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        for _ in 0..200 {
            let v = BigUint::random_bits(&mut rng, 10);
            assert!(v.bit_length() <= 10);
        }
    }

    #[test]
    fn random_bits_multi_limb_bounded() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        for _ in 0..100 {
            let v = BigUint::random_bits(&mut rng, 130);
            assert!(v.bit_length() <= 130);
        }
    }

    #[test]
    fn random_in_range_basic() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let low = BigUint::from(0u64);
        let high = BigUint::from(100u64);
        for _ in 0..500 {
            let v = BigUint::random_in_range(&mut rng, &low, &high);
            assert!(v < high);
        }
    }

    #[test]
    fn bigint_range_contains_negatives_and_positives() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        let low = BigInt::from(-50i64);
        let high = BigInt::from(50i64);
        let mut saw_neg = false;
        let mut saw_pos = false;
        for _ in 0..500 {
            let v = BigInt::random_in_range(&mut rng, &low, &high);
            if v.is_negative() {
                saw_neg = true;
            }
            if v.is_positive() {
                saw_pos = true;
            }
        }
        assert!(saw_neg);
        assert!(saw_pos);
    }

    #[test]
    fn distribution_big_uint_bits() {
        use rand::distr::Distribution;
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        let dist = BigUintBits(64);
        for _ in 0..100 {
            let v: BigUint = dist.sample(&mut rng);
            assert!(v.bit_length() <= 64);
        }
    }

    #[test]
    fn distribution_standard_uniform() {
        use rand::distr::Distribution;
        let mut rng = rand::rngs::StdRng::seed_from_u64(6);
        for _ in 0..50 {
            let v: BigUint = StandardUniform.sample(&mut rng);
            assert!(v.bit_length() <= 256);
        }
    }
}
