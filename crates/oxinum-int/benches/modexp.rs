use criterion::{criterion_group, criterion_main, Criterion};
use oxinum_int::native::{mod_pow, BigUint};

fn bench_mod_pow(c: &mut Criterion) {
    let mut group = c.benchmark_group("mod_pow");

    // 256-bit base, 256-bit exponent, 256-bit modulus
    let base = BigUint::from_le_limbs(&[
        0xDEAD_BEEF_CAFE_BABE_u64,
        0x1234_5678_9ABC_DEF0_u64,
        0xFEDC_BA98_7654_3210_u64,
        0x0F1E_2D3C_4B5A_6978_u64,
    ]);
    let exp = BigUint::from_le_limbs(&[
        0xAABB_CCDD_EEFF_0011_u64,
        0x2233_4455_6677_8899_u64,
        0xAABB_CCDD_EEFF_0011_u64,
        0x1122_3344_5566_7788_u64,
    ]);
    // Modulus must be odd; use a prime-looking number
    let modulus = BigUint::from_le_limbs(&[
        0xFFFF_FFFF_FFFF_FFC5_u64,
        0xFFFF_FFFF_FFFF_FFFF_u64,
        0xFFFF_FFFF_FFFF_FFFF_u64,
        0x0000_0000_FFFF_FFFF_u64,
    ]);

    group.bench_function("256bit", |bench| {
        bench.iter(|| mod_pow(&base, &exp, &modulus).expect("mod_pow 256bit"))
    });

    // 512-bit
    let base_512 = BigUint::from_le_limbs(&[0xDEAD_BEEF_CAFE_BABE_u64; 8]);
    let exp_512 = BigUint::from_le_limbs(&[0xAABB_CCDD_EEFF_0011_u64; 8]);
    let mod_512 = {
        let mut v = [0xFFFF_FFFF_FFFF_FFFF_u64; 8];
        v[0] = 0xFFFF_FFFF_FFFF_FFC5; // ensure odd
        BigUint::from_le_limbs(&v)
    };

    group.bench_function("512bit", |bench| {
        bench.iter(|| mod_pow(&base_512, &exp_512, &mod_512).expect("mod_pow 512bit"))
    });

    group.finish();
}

criterion_group!(benches, bench_mod_pow);
criterion_main!(benches);
