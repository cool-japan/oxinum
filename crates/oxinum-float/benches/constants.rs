// Constant computation benchmarks for oxinum-float native BigFloat.
//
// Precision values are in bits. 3322 bits ≈ 1000 decimal digits (ceil(1000 × log₂(10))).
// Quick reference: 100 bits ≈ 30 decimal digits, 500 bits ≈ 150 decimal digits,
// 1000 bits ≈ 301 decimal digits, 3322 bits ≈ 1000 decimal digits.
//
// π algorithm: Chudnovsky series with binary splitting (T1 plan, ~14.18 digits/term).
//   This IS the pi() implementation — no separate Machin-like formula is currently
//   provided at this precision level. The Machin-like formula (π/4 = 4·atan(1/5) −
//   atan(1/239)) is exercised separately in the integration test
//   crates/oxinum/tests/machin_pi.rs and is lower-performance at high precision.
//
// e algorithm: 1/n! binary splitting.
// ln2 algorithm: Hwang identity 14·atanh(1/31)+10·atanh(1/49)+6·atanh(1/161).
//
// All three constants are benchmarked at the same precision ladder (100/500/1000/3322 bits)
// so relative cost can be compared directly in criterion output.
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_float::native::{e_const, ln2, pi};

fn bench_pi(c: &mut Criterion) {
    // Chudnovsky binary splitting — ~14.18 decimal digits per series term.
    // 3322 bits = ceil(1000 × log₂(10)) ≈ 1000 decimal digits.
    let mut group = c.benchmark_group("pi");
    for prec in [100u32, 500, 1000, 3322] {
        group.bench_with_input(BenchmarkId::from_parameter(prec), &prec, |bench, &prec| {
            bench.iter(|| pi(prec).expect("pi"))
        });
    }
    group.finish();
}

fn bench_e_const(c: &mut Criterion) {
    // 1/n! binary splitting — compare wall time vs pi at identical precisions.
    let mut group = c.benchmark_group("e_const");
    for prec in [100u32, 500, 1000, 3322] {
        group.bench_with_input(BenchmarkId::from_parameter(prec), &prec, |bench, &prec| {
            bench.iter(|| e_const(prec).expect("e_const"))
        });
    }
    group.finish();
}

fn bench_ln2(c: &mut Criterion) {
    // Hwang identity (atanh sums, binary splitting) — compare vs pi and e at the same
    // precision ladder.
    let mut group = c.benchmark_group("ln2");
    for prec in [100u32, 500, 1000, 3322] {
        group.bench_with_input(BenchmarkId::from_parameter(prec), &prec, |bench, &prec| {
            bench.iter(|| ln2(prec).expect("ln2"))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_pi, bench_e_const, bench_ln2);
criterion_main!(benches);
