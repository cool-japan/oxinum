//! Integration tests for the `rand` feature on native BigUint / BigInt.
//!
//! All tests are gated behind `#[cfg(feature = "rand")]` so the file
//! compiles cleanly even without the feature enabled.

#[cfg(feature = "rand")]
mod rand_tests {
    use oxinum_int::native::{BigInt, BigUint};
    use rand::SeedableRng;

    // ------------------------------------------------------------------
    // BigUint::random_bits
    // ------------------------------------------------------------------

    #[test]
    fn random_bits_zero_gives_zero() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let v = BigUint::random_bits(&mut rng, 0);
        assert!(v.is_zero(), "random_bits(0) must be zero");
    }

    #[test]
    fn random_bits_64_bounded() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        for _ in 0..1000 {
            let v = BigUint::random_bits(&mut rng, 64);
            assert!(v.bit_length() <= 64, "random_bits(64) exceeded 64 bits");
        }
    }

    #[test]
    fn random_bits_128_bounded() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(43);
        for _ in 0..500 {
            let v = BigUint::random_bits(&mut rng, 128);
            assert!(v.bit_length() <= 128, "random_bits(128) exceeded 128 bits");
        }
    }

    #[test]
    fn random_bits_partial_top_limb_bounded() {
        // 65 bits → two limbs, top limb uses only 1 bit.
        let mut rng = rand::rngs::StdRng::seed_from_u64(44);
        for _ in 0..500 {
            let v = BigUint::random_bits(&mut rng, 65);
            assert!(v.bit_length() <= 65, "random_bits(65) exceeded 65 bits");
        }
    }

    // ------------------------------------------------------------------
    // BigUint::random_in_range
    // ------------------------------------------------------------------

    #[test]
    fn random_in_range_bounds_basic() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(123);
        let low = BigUint::from(100u64);
        let high = BigUint::from(200u64);
        for _ in 0..10_000 {
            let v = BigUint::random_in_range(&mut rng, &low, &high);
            assert!(v >= low, "random_in_range: v < low");
            assert!(v < high, "random_in_range: v >= high");
        }
    }

    #[test]
    fn random_in_range_single_value() {
        // [5, 6) must always return 5.
        let mut rng = rand::rngs::StdRng::seed_from_u64(555);
        let low = BigUint::from(5u64);
        let high = BigUint::from(6u64);
        for _ in 0..100 {
            let v = BigUint::random_in_range(&mut rng, &low, &high);
            assert_eq!(v, low, "range of size 1 must always return low");
        }
    }

    #[test]
    fn random_in_range_uniformity() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(999);
        let low = BigUint::from(0u64);
        let high = BigUint::from(100u64);
        let mut counts = vec![0u32; 100];
        for _ in 0..10_000 {
            let v = BigUint::random_in_range(&mut rng, &low, &high);
            let idx = v.to_u64().unwrap_or(0) as usize;
            if idx < 100 {
                counts[idx] += 1;
            }
        }
        // Each bucket should have roughly 100 ± 75 hits (broad tolerance).
        for (i, &count) in counts.iter().enumerate() {
            assert!(
                (40..=250).contains(&count),
                "bucket {i} has {count} hits — uniformity failure"
            );
        }
    }

    #[test]
    fn random_in_range_large_range() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(777);
        // Range spanning multiple limbs.
        let low = BigUint::from(u64::MAX);
        let high = BigUint::from_le_limbs(&[0, 0, 1]); // 2^128
        for _ in 0..200 {
            let v = BigUint::random_in_range(&mut rng, &low, &high);
            assert!(v >= low, "v < low for large range");
            assert!(v < high, "v >= high for large range");
        }
    }

    // ------------------------------------------------------------------
    // Determinism
    // ------------------------------------------------------------------

    #[test]
    fn deterministic_with_same_seed() {
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        for _ in 0..100 {
            let v1 = BigUint::random_bits(&mut rng1, 128);
            let v2 = BigUint::random_bits(&mut rng2, 128);
            assert_eq!(v1, v2, "same seed must produce same sequence");
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(1);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(2);
        let mut all_equal = true;
        for _ in 0..20 {
            let v1 = BigUint::random_bits(&mut rng1, 256);
            let v2 = BigUint::random_bits(&mut rng2, 256);
            if v1 != v2 {
                all_equal = false;
                break;
            }
        }
        assert!(
            !all_equal,
            "different seeds should produce different values"
        );
    }

    // ------------------------------------------------------------------
    // BigInt::random_in_range
    // ------------------------------------------------------------------

    #[test]
    fn bigint_random_in_range_positive() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(200);
        let low = BigInt::from(0i64);
        let high = BigInt::from(500i64);
        for _ in 0..1000 {
            let v = BigInt::random_in_range(&mut rng, &low, &high);
            assert!(v >= low && v < high);
        }
    }

    #[test]
    fn bigint_random_in_range_mixes_signs() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(77);
        let low = BigInt::from(-100i64);
        let high = BigInt::from(100i64);
        let mut saw_negative = false;
        let mut saw_positive = false;
        for _ in 0..1000 {
            let v = BigInt::random_in_range(&mut rng, &low, &high);
            assert!(v >= low, "v < low");
            assert!(v < high, "v >= high");
            if v.is_negative() {
                saw_negative = true;
            }
            if v.is_positive() {
                saw_positive = true;
            }
        }
        assert!(saw_negative, "expected some negative values");
        assert!(saw_positive, "expected some positive values");
    }

    #[test]
    fn bigint_random_in_range_purely_negative() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(88);
        let low = BigInt::from(-200i64);
        let high = BigInt::from(-100i64);
        for _ in 0..500 {
            let v = BigInt::random_in_range(&mut rng, &low, &high);
            assert!(v >= low && v < high);
            assert!(v.is_negative(), "all values must be negative");
        }
    }

    // ------------------------------------------------------------------
    // Distribution impls
    // ------------------------------------------------------------------

    #[test]
    fn big_uint_bits_distribution() {
        use oxinum_int::native::BigUintBits;
        use rand::distr::Distribution;
        let mut rng = rand::rngs::StdRng::seed_from_u64(10);
        let dist = BigUintBits(64);
        for _ in 0..100 {
            let v: BigUint = dist.sample(&mut rng);
            assert!(v.bit_length() <= 64);
        }
    }

    #[test]
    fn standard_uniform_distribution_bounded() {
        use rand::distr::{Distribution, StandardUniform};
        let mut rng = rand::rngs::StdRng::seed_from_u64(11);
        for _ in 0..50 {
            let v: BigUint = StandardUniform.sample(&mut rng);
            assert!(
                v.bit_length() <= 256,
                "StandardUniform must produce ≤ 256-bit values"
            );
        }
    }
}
