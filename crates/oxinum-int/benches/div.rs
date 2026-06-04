use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dashu_int::UBig;
use num_bigint::BigUint as NumBigUint;
use oxinum_int::native::{divrem, BigUint, NEWTON_DIV_THRESHOLD};
use std::hint::black_box;

fn bench_div_knuth(c: &mut Criterion) {
    let mut group = c.benchmark_group("div_knuth_d");
    // Below Burnikel-Ziegler threshold (NEWTON_DIV_THRESHOLD = 50 limbs)
    for vlen in [2usize, 10, 30, 49] {
        let ulen = vlen * 2;
        let u_limbs: Vec<u64> = (0..ulen)
            .map(|i| 0xDEAD_BEEF_u64.wrapping_add(i as u64))
            .collect();
        let mut v_limbs: Vec<u64> = (0..vlen)
            .map(|i| (0xCAFE_BABE_u64.wrapping_add(i as u64)) | 1)
            .collect();
        // Ensure top limb is non-zero (normalized)
        if v_limbs[vlen - 1] == 0 {
            v_limbs[vlen - 1] = 1;
        }
        let u = BigUint::from_le_limbs(&u_limbs);
        let v = BigUint::from_le_limbs(&v_limbs);
        group.bench_with_input(
            BenchmarkId::from_parameter(vlen),
            &(u, v),
            |bench, (u, v)| bench.iter(|| divrem(u, v)),
        );
    }
    group.finish();
}

fn bench_div_burnikel_ziegler(c: &mut Criterion) {
    let mut group = c.benchmark_group("div_burnikel_ziegler");
    // At or above Burnikel-Ziegler threshold
    for vlen in [
        NEWTON_DIV_THRESHOLD,
        NEWTON_DIV_THRESHOLD + 20,
        NEWTON_DIV_THRESHOLD + 60,
    ] {
        let ulen = vlen * 2;
        let mut u_limbs: Vec<u64> = (0..ulen)
            .map(|i| 0xDEAD_BEEF_u64.wrapping_add(i as u64))
            .collect();
        let mut v_limbs: Vec<u64> = (0..vlen)
            .map(|i| (0xCAFE_BABE_u64.wrapping_add(i as u64)) | 1)
            .collect();
        // Ensure top limbs are non-zero (normalized)
        if u_limbs[ulen - 1] == 0 {
            u_limbs[ulen - 1] = 1;
        }
        if v_limbs[vlen - 1] == 0 {
            v_limbs[vlen - 1] = 1;
        }
        let u = BigUint::from_le_limbs(&u_limbs);
        let v = BigUint::from_le_limbs(&v_limbs);
        group.bench_with_input(
            BenchmarkId::from_parameter(vlen),
            &(u, v),
            |bench, (u, v)| bench.iter(|| divrem(u, v)),
        );
    }
    group.finish();
}

fn bench_div_vs_baselines(c: &mut Criterion) {
    let mut group = c.benchmark_group("div_vs_baselines");
    // Test at sizes spanning the algorithm boundary
    for vlen in [10usize, 30, NEWTON_DIV_THRESHOLD, NEWTON_DIV_THRESHOLD + 60] {
        let ulen = vlen * 2;
        let mut u_limbs: Vec<u64> = (0..ulen)
            .map(|i| 0xDEAD_BEEF_u64.wrapping_add(i as u64))
            .collect();
        let mut v_limbs: Vec<u64> = (0..vlen)
            .map(|i| 0xCAFE_BABE_u64.wrapping_add(i as u64))
            .collect();
        if let Some(last) = u_limbs.last_mut() {
            if *last == 0 {
                *last = 1;
            }
        }
        if let Some(last) = v_limbs.last_mut() {
            if *last == 0 {
                *last = 1;
            }
        }
        // ensure v is odd (avoid algorithm edge case)
        v_limbs[vlen - 1] |= 1;

        let u_oxi = BigUint::from_le_limbs(&u_limbs);
        let v_oxi = BigUint::from_le_limbs(&v_limbs);

        let a_bytes: Vec<u8> = u_limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
        let b_bytes: Vec<u8> = v_limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
        let u_nb = NumBigUint::from_bytes_le(&a_bytes);
        let v_nb = NumBigUint::from_bytes_le(&b_bytes);

        let u_du_words: Vec<dashu_int::Word> = u_limbs
            .iter()
            .copied()
            .map(|w| w as dashu_int::Word)
            .collect();
        let v_du_words: Vec<dashu_int::Word> = v_limbs
            .iter()
            .copied()
            .map(|w| w as dashu_int::Word)
            .collect();
        let u_du = UBig::from_words(&u_du_words);
        let v_du = UBig::from_words(&v_du_words);

        group.bench_with_input(
            criterion::BenchmarkId::new("oxinum", vlen),
            &vlen,
            |bch, _| bch.iter(|| divrem(black_box(&u_oxi), black_box(&v_oxi))),
        );
        group.bench_with_input(
            criterion::BenchmarkId::new("dashu", vlen),
            &vlen,
            |bch, _| {
                bch.iter(|| {
                    (
                        black_box(&u_du) / black_box(&v_du),
                        black_box(&u_du) % black_box(&v_du),
                    )
                })
            },
        );
        group.bench_with_input(
            criterion::BenchmarkId::new("num_bigint", vlen),
            &vlen,
            |bch, _| {
                bch.iter(|| {
                    (
                        black_box(&u_nb) / black_box(&v_nb),
                        black_box(&u_nb) % black_box(&v_nb),
                    )
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_div_knuth,
    bench_div_burnikel_ziegler,
    bench_div_vs_baselines
);
criterion_main!(benches);
