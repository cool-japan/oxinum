use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use oxinum_int::native::factorial;

fn bench_factorial(c: &mut Criterion) {
    let mut group = c.benchmark_group("factorial");
    for n in [100u64, 1000, 5000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bench, &n| {
            bench.iter(|| factorial(n))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_factorial);
criterion_main!(benches);
