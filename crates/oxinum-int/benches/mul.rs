use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dashu_int::UBig;
use num_bigint::BigUint as NumBigUint;
use oxinum_int::native::BigUint;
use std::hint::black_box;

fn make_limbs(seed: u64, n: usize) -> Vec<u64> {
    (0..n).map(|i| seed.wrapping_add(i as u64)).collect()
}

fn bench_mul_schoolbook(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_schoolbook");
    // Below Karatsuba threshold (~32 limbs)
    for nlimbs in [1usize, 8, 16, 24] {
        let a_limbs = make_limbs(0xDEAD_BEEF_u64, nlimbs);
        let b_limbs = make_limbs(0xCAFE_BABE_u64, nlimbs);
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
        let a_limbs = make_limbs(0xDEAD_BEEF_u64, nlimbs);
        let b_limbs = make_limbs(0xCAFE_BABE_u64, nlimbs);
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
        let a_limbs = make_limbs(0xDEAD_BEEF_u64, nlimbs);
        let b_limbs = make_limbs(0xCAFE_BABE_u64, nlimbs);
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

fn bench_mul_vs_baselines(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_vs_baselines");
    // Sizes spanning schoolbook / Karatsuba / Toom-3 / large Toom-3
    for nlimbs in [8usize, 32, 100, 400] {
        let a_limbs = make_limbs(0xDEAD_BEEF_CAFE_0000, nlimbs);
        let b_limbs = make_limbs(0xCAFE_BABE_0000_1234, nlimbs);

        // oxinum native
        let a_oxi = BigUint::from_le_limbs(&a_limbs);
        let b_oxi = BigUint::from_le_limbs(&b_limbs);

        // dashu (Word = u64 on 64-bit; from_words takes &[Word] = &[u64])
        let a_du_words: Vec<dashu_int::Word> = a_limbs
            .iter()
            .copied()
            .map(|w| w as dashu_int::Word)
            .collect();
        let b_du_words: Vec<dashu_int::Word> = b_limbs
            .iter()
            .copied()
            .map(|w| w as dashu_int::Word)
            .collect();
        let a_du = UBig::from_words(&a_du_words);
        let b_du = UBig::from_words(&b_du_words);

        // num-bigint via little-endian bytes
        let a_bytes: Vec<u8> = a_limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
        let b_bytes: Vec<u8> = b_limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
        let a_nb = NumBigUint::from_bytes_le(&a_bytes);
        let b_nb = NumBigUint::from_bytes_le(&b_bytes);

        group.bench_with_input(
            criterion::BenchmarkId::new("oxinum", nlimbs),
            &nlimbs,
            |bch, _| bch.iter(|| black_box(&a_oxi) * black_box(&b_oxi)),
        );
        group.bench_with_input(
            criterion::BenchmarkId::new("dashu", nlimbs),
            &nlimbs,
            |bch, _| bch.iter(|| black_box(&a_du) * black_box(&b_du)),
        );
        group.bench_with_input(
            criterion::BenchmarkId::new("num_bigint", nlimbs),
            &nlimbs,
            |bch, _| bch.iter(|| black_box(&a_nb) * black_box(&b_nb)),
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_mul_schoolbook,
    bench_mul_karatsuba,
    bench_mul_toom3,
    bench_mul_vs_baselines
);
criterion_main!(benches);
