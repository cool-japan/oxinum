//! Benchmark: oxinum native BigUint vs num-bigint for common operations.
//!
//! Run with:
//!   cargo bench -p oxinum --bench vs_num_bigint

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use num_bigint::BigUint as NumBigUint;
use oxinum::native::BigUint as OxiBigUint;

/// Build a pair of 100-limb test values.
fn make_100_limb_pair() -> (Vec<u64>, Vec<u64>) {
    let a: Vec<u64> = (0..100)
        .map(|i: u64| 0xDEAD_BEEF_u64.wrapping_add(i))
        .collect();
    let b: Vec<u64> = (0..100)
        .map(|i: u64| 0xCAFE_BABE_u64.wrapping_add(i))
        .collect();
    (a, b)
}

fn bench_mul_100_limbs(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_100limbs");

    let (a_limbs, b_limbs) = make_100_limb_pair();

    // oxinum native
    let a_oxi = OxiBigUint::from_le_limbs(&a_limbs);
    let b_oxi = OxiBigUint::from_le_limbs(&b_limbs);

    // num-bigint (little-endian bytes)
    let a_bytes: Vec<u8> = a_limbs.iter().flat_map(|&l| l.to_le_bytes()).collect();
    let b_bytes: Vec<u8> = b_limbs.iter().flat_map(|&l| l.to_le_bytes()).collect();
    let a_num = NumBigUint::from_bytes_le(&a_bytes);
    let b_num = NumBigUint::from_bytes_le(&b_bytes);

    group.bench_function(BenchmarkId::new("oxinum_native", "100-limbs"), |bench| {
        bench.iter(|| &a_oxi * &b_oxi)
    });
    group.bench_function(BenchmarkId::new("num_bigint", "100-limbs"), |bench| {
        bench.iter(|| &a_num * &b_num)
    });

    group.finish();
}

fn bench_add_100_limbs(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_100limbs");

    let (a_limbs, b_limbs) = make_100_limb_pair();

    // oxinum native
    let a_oxi = OxiBigUint::from_le_limbs(&a_limbs);
    let b_oxi = OxiBigUint::from_le_limbs(&b_limbs);

    // num-bigint
    let a_bytes: Vec<u8> = a_limbs.iter().flat_map(|&l| l.to_le_bytes()).collect();
    let b_bytes: Vec<u8> = b_limbs.iter().flat_map(|&l| l.to_le_bytes()).collect();
    let a_num = NumBigUint::from_bytes_le(&a_bytes);
    let b_num = NumBigUint::from_bytes_le(&b_bytes);

    group.bench_function(BenchmarkId::new("oxinum_native", "100-limbs"), |bench| {
        bench.iter(|| &a_oxi + &b_oxi)
    });
    group.bench_function(BenchmarkId::new("num_bigint", "100-limbs"), |bench| {
        bench.iter(|| &a_num + &b_num)
    });

    group.finish();
}

fn bench_mul_10_limbs(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_10limbs");

    let a_limbs: Vec<u64> = (0..10)
        .map(|i: u64| 0xDEAD_BEEF_u64.wrapping_add(i))
        .collect();
    let b_limbs: Vec<u64> = (0..10)
        .map(|i: u64| 0xCAFE_BABE_u64.wrapping_add(i))
        .collect();

    let a_oxi = OxiBigUint::from_le_limbs(&a_limbs);
    let b_oxi = OxiBigUint::from_le_limbs(&b_limbs);

    let a_bytes: Vec<u8> = a_limbs.iter().flat_map(|&l| l.to_le_bytes()).collect();
    let b_bytes: Vec<u8> = b_limbs.iter().flat_map(|&l| l.to_le_bytes()).collect();
    let a_num = NumBigUint::from_bytes_le(&a_bytes);
    let b_num = NumBigUint::from_bytes_le(&b_bytes);

    group.bench_function(BenchmarkId::new("oxinum_native", "10-limbs"), |bench| {
        bench.iter(|| &a_oxi * &b_oxi)
    });
    group.bench_function(BenchmarkId::new("num_bigint", "10-limbs"), |bench| {
        bench.iter(|| &a_num * &b_num)
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mul_100_limbs,
    bench_add_100_limbs,
    bench_mul_10_limbs
);
criterion_main!(benches);
