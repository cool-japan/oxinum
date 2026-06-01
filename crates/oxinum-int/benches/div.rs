use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_int::native::{divrem, BigUint, NEWTON_DIV_THRESHOLD};

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

criterion_group!(benches, bench_div_knuth, bench_div_burnikel_ziegler);
criterion_main!(benches);
