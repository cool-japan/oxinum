//! Allocation profiling for oxinum-rational native big-rational operations.
//! Run with: cargo bench -p oxinum-rational --bench alloc_profile
//!
//! NOTE: This file uses unsafe for the GlobalAlloc impl.
//! This is a bench binary — bench binaries do NOT inherit the library's
//! #![forbid(unsafe_code)].

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

struct CountingAlloc;

static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES_ALLOC: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOC_CALLS.fetch_add(1, Relaxed);
            BYTES_ALLOC.fetch_add(layout.size(), Relaxed);
            let live = LIVE_BYTES.fetch_add(layout.size(), Relaxed) + layout.size();
            let _ = PEAK_BYTES.fetch_max(live, Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        DEALLOC_CALLS.fetch_add(1, Relaxed);
        LIVE_BYTES.fetch_sub(layout.size(), Relaxed);
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

#[derive(Clone, Copy, Debug, Default)]
struct AllocStats {
    alloc_calls: usize,
    dealloc_calls: usize,
    bytes_alloc: usize,
    live_bytes: usize,
    peak_bytes: usize,
}

fn reset() {
    ALLOC_CALLS.store(0, Relaxed);
    DEALLOC_CALLS.store(0, Relaxed);
    BYTES_ALLOC.store(0, Relaxed);
    PEAK_BYTES.store(LIVE_BYTES.load(Relaxed), Relaxed);
}

fn snapshot() -> AllocStats {
    AllocStats {
        alloc_calls: ALLOC_CALLS.load(Relaxed),
        dealloc_calls: DEALLOC_CALLS.load(Relaxed),
        bytes_alloc: BYTES_ALLOC.load(Relaxed),
        live_bytes: LIVE_BYTES.load(Relaxed),
        peak_bytes: PEAK_BYTES.load(Relaxed),
    }
}

fn measure<R>(f: impl FnOnce() -> R) -> (R, AllocStats) {
    reset();
    let r = f();
    (r, snapshot())
}

use oxinum_int::native::{BigInt, BigUint};
use oxinum_rational::native::BigRational;

/// Build a `BigRational` from i64/u64; denominator must be nonzero.
/// `.expect()` is allowed in bench code for known-infallible paths.
fn rat(n: i64, d: u64) -> BigRational {
    BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d)).expect("nonzero denominator")
}

fn main() {
    // Fibonacci-based rational for a long all-1s continued fraction.
    // F(20)/F(19) = 6765/4181 — gives a CF of length ~20 with all coefficients = 1.
    fn fib_rat() -> BigRational {
        rat(6765, 4181)
    }

    // Large-coefficient rational: numerator/denominator with multi-digit values.
    // This exercises the BigInt arithmetic in the CF expansion.
    fn large_rat() -> BigRational {
        let n = BigInt::from(123456789012345i64);
        let d = BigUint::from_u64(98765432109876);
        BigRational::from_parts(n, d).expect("valid")
    }

    type BoxedFn = Box<dyn Fn() -> BigRational>;
    let samples: &[(&str, BoxedFn)] = &[
        ("355/113", Box::new(|| rat(355, 113))),
        ("1457/991", Box::new(|| rat(1457, 991))),
        ("fib(6765/4181)", Box::new(fib_rat)),
        ("large", Box::new(large_rat)),
    ];

    println!(
        "{:<22} {:<30} {:>8} {:>12} {:>12} {:>14} {:>12} {:>12}",
        "rational",
        "op",
        "cf_len",
        "alloc_calls",
        "dealloc_calls",
        "bytes_alloc",
        "peak_bytes",
        "live_after"
    );
    println!("{}", "-".repeat(122));

    for (name, make) in samples {
        let r = make();

        // continued_fraction
        let (cf, s) = measure(|| black_box(&r).continued_fraction());
        let cf_len = cf.len();
        black_box(&cf);
        println!(
            "{:<22} {:<30} {:>8} {:>12} {:>12} {:>14} {:>12} {:>12}",
            name,
            "continued_fraction",
            cf_len,
            s.alloc_calls,
            s.dealloc_calls,
            s.bytes_alloc,
            s.peak_bytes,
            s.live_bytes
        );

        // convergents
        let (cvg, s) = measure(|| black_box(&r).convergents());
        black_box(&cvg);
        println!(
            "{:<22} {:<30} {:>8} {:>12} {:>12} {:>14} {:>12} {:>12}",
            name,
            "convergents",
            cvg.len(),
            s.alloc_calls,
            s.dealloc_calls,
            s.bytes_alloc,
            s.peak_bytes,
            s.live_bytes
        );

        // best_rational_approximation
        let max_den = BigUint::from_u64(1000);
        let (ba, s) = measure(|| black_box(&r).best_rational_approximation(black_box(&max_den)));
        black_box(&ba);
        println!(
            "{:<22} {:<30} {:>8} {:>12} {:>12} {:>14} {:>12} {:>12}",
            name,
            "best_rational_approx(<1000)",
            0usize,
            s.alloc_calls,
            s.dealloc_calls,
            s.bytes_alloc,
            s.peak_bytes,
            s.live_bytes
        );

        println!();
    }
}
