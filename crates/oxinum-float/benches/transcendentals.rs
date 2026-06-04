use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_float::native::{BigFloat, RoundingMode};
use oxinum_float::DBig;

fn bench_exp(c: &mut Criterion) {
    let mut group = c.benchmark_group("exp");
    for prec in [100u32, 500] {
        let x = BigFloat::from_i64(2, prec, RoundingMode::HalfEven);
        group.bench_with_input(
            BenchmarkId::from_parameter(prec),
            &(x, prec),
            |bench, (x, prec)| {
                let prec = *prec;
                bench.iter(|| x.exp(prec, RoundingMode::HalfEven).expect("exp"))
            },
        );
    }
    group.finish();
}

fn bench_ln(c: &mut Criterion) {
    let mut group = c.benchmark_group("ln");
    for prec in [100u32, 500] {
        let x = BigFloat::from_i64(7, prec, RoundingMode::HalfEven);
        group.bench_with_input(
            BenchmarkId::from_parameter(prec),
            &(x, prec),
            |bench, (x, prec)| {
                let prec = *prec;
                bench.iter(|| x.ln(prec, RoundingMode::HalfEven).expect("ln"))
            },
        );
    }
    group.finish();
}

fn bench_ln_agm(c: &mut Criterion) {
    let mut group = c.benchmark_group("ln_agm_vs_newton");
    for prec in [100u32, 500, 1000] {
        let x = BigFloat::from_i64(7, prec, RoundingMode::HalfEven);
        group.bench_with_input(
            BenchmarkId::new("agm", prec),
            &(x.clone(), prec),
            |bench, (x, prec)| {
                let prec = *prec;
                bench.iter(|| x.ln_agm(prec, RoundingMode::HalfEven).expect("ln_agm"))
            },
        );
        group.bench_with_input(
            BenchmarkId::new("newton", prec),
            &(x, prec),
            |bench, (x, prec)| {
                let prec = *prec;
                bench.iter(|| x.ln(prec, RoundingMode::HalfEven).expect("ln_newton"))
            },
        );
    }
    group.finish();
}

fn bench_transcendental_vs_dashu(c: &mut Criterion) {
    let mut group = c.benchmark_group("transcendental_vs_dashu");

    // NOTE: dashu-float DBig (decimal base-10) exposes exp() and ln() but NOT
    // sin/cos/tan.  This baseline covers exp and ln only.
    // Native sin/cos have no dashu equivalent and are therefore not compared here.
    for prec in [100u32, 500] {
        // --- exp(2) ---
        let x_native = BigFloat::from_i64(2, prec, RoundingMode::HalfEven);
        // Construct DBig "2" with the same decimal-digit precision.
        // DBig::with_precision returns Rounded<DBig> — call .value() for the result.
        let x_dashu = {
            use std::str::FromStr;
            DBig::from_str("2")
                .expect("DBig from 2")
                .with_precision(prec as usize)
                .value()
        };

        group.bench_with_input(BenchmarkId::new("oxinum_exp", prec), &prec, |b, &p| {
            b.iter(|| x_native.exp(p, RoundingMode::HalfEven).expect("exp"))
        });
        group.bench_with_input(BenchmarkId::new("dashu_exp", prec), &prec, |b, _| {
            b.iter(|| x_dashu.exp())
        });

        // --- ln(7) ---
        let x7_native = BigFloat::from_i64(7, prec, RoundingMode::HalfEven);
        let x7_dashu = {
            use std::str::FromStr;
            DBig::from_str("7")
                .expect("DBig from 7")
                .with_precision(prec as usize)
                .value()
        };

        group.bench_with_input(BenchmarkId::new("oxinum_ln", prec), &prec, |b, &p| {
            b.iter(|| x7_native.ln(p, RoundingMode::HalfEven).expect("ln"))
        });
        group.bench_with_input(BenchmarkId::new("dashu_ln", prec), &prec, |b, _| {
            b.iter(|| x7_dashu.ln())
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_exp,
    bench_ln,
    bench_ln_agm,
    bench_transcendental_vs_dashu
);
criterion_main!(benches);
