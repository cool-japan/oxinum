use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;
use oxinum_int::native::{BigInt, BigUint};
use oxinum_rational::native::BigRational;

fn make_rat(n: i64, d: u64) -> BigRational {
    BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d)).expect("valid rational")
}

fn bench_rational_gcd_reduction(c: &mut Criterion) {
    let mut group = c.benchmark_group("rational_gcd_reduction");
    group.bench_function("from_parts_large_gcd", |bench| {
        bench.iter(|| {
            // 720720/360360 → 2/1, requires GCD reduction
            make_rat(720720, 360360)
        })
    });
    group.bench_function("from_parts_coprime", |bench| {
        bench.iter(|| make_rat(355, 113))
    });
    group.finish();
}

fn bench_rational_arithmetic_chain(c: &mut Criterion) {
    // Benchmark: sum of harmonic series 1/1 + 1/2 + ... + 1/N (exact)
    let mut group = c.benchmark_group("harmonic_sum");
    for n in [10u64, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bench, &n| {
            bench.iter(|| {
                let mut sum = make_rat(0, 1);
                for k in 1..=n {
                    let term = make_rat(1, k);
                    sum = &sum + &term;
                }
                sum
            })
        });
    }
    group.finish();
}

fn bench_rational_3x3_determinant(c: &mut Criterion) {
    // Benchmark: 3x3 determinant via Sarrus' rule over exact rationals
    let mut group = c.benchmark_group("determinant_3x3");
    group.bench_function("exact_rationals", |bench| {
        bench.iter(|| {
            let a00 = make_rat(1, 2);
            let a01 = make_rat(3, 4);
            let a02 = make_rat(5, 6);
            let a10 = make_rat(7, 8);
            let a11 = make_rat(9, 10);
            let a12 = make_rat(11, 12);
            let a20 = make_rat(13, 14);
            let a21 = make_rat(15, 16);
            let a22 = make_rat(17, 18);
            // Sarrus: det = a00*(a11*a22 - a12*a21) - a01*(a10*a22 - a12*a20) + a02*(a10*a21 - a11*a20)
            let t0 = &(&a11 * &a22) - &(&a12 * &a21);
            let t1 = &(&a10 * &a22) - &(&a12 * &a20);
            let t2 = &(&a10 * &a21) - &(&a11 * &a20);
            &(&a00 * &t0) - &(&a01 * &t1) + &(&a02 * &t2)
        })
    });
    group.finish();
}

// NOTE: dashu-ratio (RBig) does NOT expose a continued_fraction() method.
// This baseline covers only arithmetic operations (add, mul) where a
// comparison with the oxinum native BigRational is meaningful.
// CF allocation profiling is in benches/alloc_profile.rs (native only).
fn bench_rational_vs_dashu(c: &mut Criterion) {
    let mut group = c.benchmark_group("rational_vs_dashu");

    // Pairs used for both addition and multiplication chains.
    let pairs: &[(i64, u64)] = &[(355, 113), (22, 7), (1457, 991), (720720, 360360)];

    // --- Addition ---
    group.bench_function("oxinum_add_chain", |b| {
        b.iter(|| {
            let mut acc = make_rat(0, 1);
            for &(n, d) in pairs {
                acc = &acc + &make_rat(n, d);
            }
            acc
        })
    });
    group.bench_function("dashu_add_chain", |b| {
        b.iter(|| {
            let mut acc = RBig::ZERO;
            for &(n, d) in pairs {
                acc = &acc + &RBig::from_parts(IBig::from(n), UBig::from(d));
            }
            acc
        })
    });

    // --- Multiplication ---
    group.bench_function("oxinum_mul_chain", |b| {
        b.iter(|| {
            let mut acc = make_rat(1, 1);
            for &(n, d) in pairs {
                acc = &acc * &make_rat(n, d);
            }
            acc
        })
    });
    group.bench_function("dashu_mul_chain", |b| {
        b.iter(|| {
            let mut acc = RBig::ONE;
            for &(n, d) in pairs {
                acc = &acc * &RBig::from_parts(IBig::from(n), UBig::from(d));
            }
            acc
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_rational_gcd_reduction,
    bench_rational_arithmetic_chain,
    bench_rational_3x3_determinant,
    bench_rational_vs_dashu
);
criterion_main!(benches);
