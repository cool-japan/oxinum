use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_float::native::{BigFloat, RoundingMode};

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

criterion_group!(benches, bench_exp, bench_ln, bench_ln_agm);
criterion_main!(benches);
