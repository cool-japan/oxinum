use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_int::native::{gcd, gcd_binary, BigUint};

fn rand_biguint(state: &mut u64, nlimbs: usize) -> BigUint {
    let mut limbs = Vec::with_capacity(nlimbs);
    for _ in 0..nlimbs {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        limbs.push(*state | 1); // ensure nonzero
    }
    BigUint::from_le_limbs(&limbs)
}

fn bench_gcd_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("gcd");
    for nlimbs in [4usize, 16, 64, 128] {
        let mut state = 0xDEAD_BEEF_CAFE_BABE_u64;
        let a = rand_biguint(&mut state, nlimbs);
        let b = rand_biguint(&mut state, nlimbs);

        group.bench_with_input(
            BenchmarkId::new("lehmer", nlimbs),
            &(a.clone(), b.clone()),
            |bench, (a, b)| bench.iter(|| gcd(a.clone(), b.clone())),
        );
        group.bench_with_input(
            BenchmarkId::new("binary_stein", nlimbs),
            &(a, b),
            |bench, (a, b)| bench.iter(|| gcd_binary(a.clone(), b.clone())),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_gcd_variants);
criterion_main!(benches);
