use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_int::native::{is_probably_prime, BigUint};

fn bench_primality(c: &mut Criterion) {
    let mut group = c.benchmark_group("primality");

    // Large primes using u64 single-limb path (deterministic Miller-Rabin)
    let primes_u64: &[u64] = &[
        9_007_199_254_740_881,     // large 53-bit prime
        4_611_686_018_427_387_847, // near 2^62, prime
        9_223_372_036_854_775_783, // near i64::MAX, prime
    ];

    for &p in primes_u64 {
        let n = BigUint::from_u64(p);
        group.bench_with_input(BenchmarkId::new("mr_u64", p), &n, |bench, n| {
            bench.iter(|| is_probably_prime(n))
        });
    }

    // M89 = 2^89 - 1 (Mersenne prime, known to be prime) — above u128::MAX
    // bound for deterministic MR, so routes through BPSW.
    // 2^89 - 1 in little-endian 64-bit limbs:
    //   limb0 = u64::MAX (bits 0..63)
    //   limb1 = (2^25 - 1) = 0x01FF_FFFF (bits 64..88)
    let m89 = BigUint::from_le_limbs(&[u64::MAX, 0x01FF_FFFF]);
    group.bench_with_input(BenchmarkId::new("bpsw_m89", "M89"), &m89, |bench, n| {
        bench.iter(|| is_probably_prime(n))
    });

    group.finish();
}

criterion_group!(benches, bench_primality);
criterion_main!(benches);
