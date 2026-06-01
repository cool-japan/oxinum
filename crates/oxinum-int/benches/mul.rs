use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_int::native::BigUint;

fn bench_mul_schoolbook(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_schoolbook");
    // Below Karatsuba threshold (~32 limbs)
    for nlimbs in [1usize, 8, 16, 24] {
        let a_limbs: Vec<u64> = (0..nlimbs)
            .map(|i| 0xDEAD_BEEF_u64.wrapping_add(i as u64))
            .collect();
        let b_limbs: Vec<u64> = (0..nlimbs)
            .map(|i| 0xCAFE_BABE_u64.wrapping_add(i as u64))
            .collect();
        let a = BigUint::from_le_limbs(&a_limbs);
        let b = BigUint::from_le_limbs(&b_limbs);
        group.bench_with_input(
            BenchmarkId::from_parameter(nlimbs),
            &(a, b),
            |bench, (a, b)| bench.iter(|| a * b),
        );
    }
    group.finish();
}

fn bench_mul_karatsuba(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_karatsuba");
    // Above Karatsuba threshold (~32 limbs), below Toom-3 (~100 limbs)
    for nlimbs in [32usize, 50, 80] {
        let a_limbs: Vec<u64> = (0..nlimbs)
            .map(|i| 0xDEAD_BEEF_u64.wrapping_add(i as u64))
            .collect();
        let b_limbs: Vec<u64> = (0..nlimbs)
            .map(|i| 0xCAFE_BABE_u64.wrapping_add(i as u64))
            .collect();
        let a = BigUint::from_le_limbs(&a_limbs);
        let b = BigUint::from_le_limbs(&b_limbs);
        group.bench_with_input(
            BenchmarkId::from_parameter(nlimbs),
            &(a, b),
            |bench, (a, b)| bench.iter(|| a * b),
        );
    }
    group.finish();
}

fn bench_mul_toom3(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_toom3");
    // At or above Toom-3 threshold (~100 limbs)
    for nlimbs in [100usize, 200, 400] {
        let a_limbs: Vec<u64> = (0..nlimbs)
            .map(|i| 0xDEAD_BEEF_u64.wrapping_add(i as u64))
            .collect();
        let b_limbs: Vec<u64> = (0..nlimbs)
            .map(|i| 0xCAFE_BABE_u64.wrapping_add(i as u64))
            .collect();
        let a = BigUint::from_le_limbs(&a_limbs);
        let b = BigUint::from_le_limbs(&b_limbs);
        group.bench_with_input(
            BenchmarkId::from_parameter(nlimbs),
            &(a, b),
            |bench, (a, b)| bench.iter(|| a * b),
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_mul_schoolbook,
    bench_mul_karatsuba,
    bench_mul_toom3
);
criterion_main!(benches);
