//! Criterion benchmarks for [`oxinum_complex::CBig`] transcendental functions.
//!
//! A fixed off-axis operand `z = 1.25 − 0.75i` is exercised through `mul`,
//! `exp`, `ln`, `sqrt`, and `pow` at three working precisions
//! ({50, 200, 1000} significant decimal digits). Each routine is benchmarked
//! per (operation, precision) pair via `black_box` so the optimiser cannot fold
//! the work away. The `pow` benchmark raises `z` to a fixed off-axis exponent.
//!
//! This target is declared `harness = false` in `Cargo.toml`, so it provides
//! its own `criterion_main!`. Building it (`cargo build --benches`) is part of
//! the crate's test surface; the long runs themselves are optional.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_complex::CBig;

/// Working precisions (significant decimal digits) swept by every benchmark.
const PRECISIONS: [usize; 3] = [50, 200, 1000];

/// The fixed off-axis operand `1.25 − 0.75i`.
fn operand() -> CBig {
    CBig::from_f64(1.25, -0.75).expect("finite parts")
}

/// The fixed off-axis exponent `0.5 + 0.25i` used by the `pow` benchmark.
fn exponent() -> CBig {
    CBig::from_f64(0.5, 0.25).expect("finite parts")
}

fn bench_mul(c: &mut Criterion) {
    let mut group = c.benchmark_group("cbig_mul");
    let z = operand();
    for &p in &PRECISIONS {
        // `mul` itself is precision-free; the operand carries its precision via
        // `from_f64`. We still parametrise by `p` to keep the report uniform and
        // to reflect that downstream products feed precision-`p` transcendentals.
        group.bench_with_input(BenchmarkId::from_parameter(p), &p, |b, _| {
            b.iter(|| black_box(&z) * black_box(&z));
        });
    }
    group.finish();
}

fn bench_exp(c: &mut Criterion) {
    let mut group = c.benchmark_group("cbig_exp");
    let z = operand();
    for &p in &PRECISIONS {
        group.bench_with_input(BenchmarkId::from_parameter(p), &p, |b, &p| {
            b.iter(|| black_box(&z).exp(black_box(p)).expect("exp"));
        });
    }
    group.finish();
}

fn bench_ln(c: &mut Criterion) {
    let mut group = c.benchmark_group("cbig_ln");
    let z = operand();
    for &p in &PRECISIONS {
        group.bench_with_input(BenchmarkId::from_parameter(p), &p, |b, &p| {
            b.iter(|| black_box(&z).ln(black_box(p)).expect("ln"));
        });
    }
    group.finish();
}

fn bench_sqrt(c: &mut Criterion) {
    let mut group = c.benchmark_group("cbig_sqrt");
    let z = operand();
    for &p in &PRECISIONS {
        group.bench_with_input(BenchmarkId::from_parameter(p), &p, |b, &p| {
            b.iter(|| black_box(&z).sqrt(black_box(p)).expect("sqrt"));
        });
    }
    group.finish();
}

fn bench_pow(c: &mut Criterion) {
    let mut group = c.benchmark_group("cbig_pow");
    let z = operand();
    let w = exponent();
    for &p in &PRECISIONS {
        group.bench_with_input(BenchmarkId::from_parameter(p), &p, |b, &p| {
            b.iter(|| black_box(&z).pow(black_box(&w), black_box(p)).expect("pow"));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_mul, bench_exp, bench_ln, bench_sqrt, bench_pow);
criterion_main!(benches);
