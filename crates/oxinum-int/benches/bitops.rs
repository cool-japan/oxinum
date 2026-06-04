//! Throughput benchmarks for `native::BigUint` bitwise and shift operations.
//!
//! # SIMD vs scalar
//!
//! When this crate is compiled with `--features simd` on a nightly compiler,
//! `build.rs` emits `cfg(oxinum_simd)` and the `portable_simd` inner kernel is
//! active.  On stable (or without the feature) the scalar fallback is used.
//! Both paths produce bit-identical results; only throughput differs.
//!
//! Run with:
//! ```text
//! # Stable (scalar path):
//! cargo bench -p oxinum-int --bench bitops
//!
//! # Nightly with SIMD:
//! cargo +nightly bench -p oxinum-int --bench bitops --features simd
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_int::native::BigUint;
use std::hint::black_box;

/// Limb counts to benchmark at.
const LIMB_COUNTS: &[usize] = &[16, 64, 256, 1024];

fn make_biguint(seed: u64, n: usize) -> BigUint {
    // Ensure the MSB is non-zero so normalization doesn't shorten the vector.
    let limbs: Vec<u64> = (0..n)
        .map(|i| {
            let v = seed
                .wrapping_add(i as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15);
            // Force MSB non-zero for the final limb so the vector stays n long.
            if i == n - 1 {
                v | 1
            } else {
                v
            }
        })
        .collect();
    BigUint::from_le_limbs(&limbs)
}

// ---------------------------------------------------------------------------
// AND
// ---------------------------------------------------------------------------

fn bench_bitand(c: &mut Criterion) {
    let mut group = c.benchmark_group("biguint_bitand");
    for &n in LIMB_COUNTS {
        let a = make_biguint(0xDEAD_BEEF, n);
        let b = make_biguint(0xCAFE_BABE, n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &(&a, &b), |bch, (a, b)| {
            bch.iter(|| black_box(*a) & black_box(*b))
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// OR
// ---------------------------------------------------------------------------

fn bench_bitor(c: &mut Criterion) {
    let mut group = c.benchmark_group("biguint_bitor");
    for &n in LIMB_COUNTS {
        let a = make_biguint(0xDEAD_BEEF, n);
        let b = make_biguint(0xCAFE_BABE, n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &(&a, &b), |bch, (a, b)| {
            bch.iter(|| black_box(*a) | black_box(*b))
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// XOR
// ---------------------------------------------------------------------------

fn bench_bitxor(c: &mut Criterion) {
    let mut group = c.benchmark_group("biguint_bitxor");
    for &n in LIMB_COUNTS {
        let a = make_biguint(0xDEAD_BEEF, n);
        let b = make_biguint(0xCAFE_BABE, n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &(&a, &b), |bch, (a, b)| {
            bch.iter(|| black_box(*a) ^ black_box(*b))
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// SHL
// ---------------------------------------------------------------------------

fn bench_shl_bits(c: &mut Criterion) {
    let mut group = c.benchmark_group("biguint_shl_bits");
    // Use a non-trivial bit offset so the within-limb path is exercised.
    let shift = 37u64;
    for &n in LIMB_COUNTS {
        let a = make_biguint(0xDEAD_BEEF, n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &a, |bch, a| {
            bch.iter(|| black_box(a).shl_bits(black_box(shift)))
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// SHR
// ---------------------------------------------------------------------------

fn bench_shr_bits(c: &mut Criterion) {
    let mut group = c.benchmark_group("biguint_shr_bits");
    let shift = 37u64;
    for &n in LIMB_COUNTS {
        let a = make_biguint(0xDEAD_BEEF, n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &a, |bch, a| {
            bch.iter(|| black_box(a).shr_bits(black_box(shift)))
        });
    }
    group.finish();
}

criterion_group!(
    bitops_benches,
    bench_bitand,
    bench_bitor,
    bench_bitxor,
    bench_shl_bits,
    bench_shr_bits,
);
criterion_main!(bitops_benches);
