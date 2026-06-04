//! Allocation profiling for oxinum-int native big-integer operations.
//! Run with: cargo bench -p oxinum-int --bench alloc_profile
//!
//! NOTE: This file uses unsafe for the GlobalAlloc impl.
//! This is a bench binary — it does NOT inherit the library's #![forbid(unsafe_code)].

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

fn main() {
    use oxinum_int::native::BigUint;

    fn make_limbs(seed: u64, n: usize) -> Vec<u64> {
        (0..n).map(|i| seed.wrapping_add(i as u64)).collect()
    }

    println!(
        "{:<8} {:<18} {:>12} {:>14} {:>12} {:>12} {:>12}",
        "limbs", "op", "alloc_calls", "dealloc_calls", "bytes_alloc", "peak_bytes", "live_after"
    );

    // Tier sizes: schoolbook (<32), karatsuba (32..~100), toom3 (~100+), large toom3
    for n in [8usize, 64, 200, 1000] {
        let a_limbs = make_limbs(0xDEAD_BEEF_CAFE_0000, n);
        let b_limbs = make_limbs(0xCAFE_BABE_0000_1234, n);
        let a = BigUint::from_le_limbs(&a_limbs);
        let b = BigUint::from_le_limbs(&b_limbs);

        // Single multiply
        let (prod, s) = measure(|| black_box(&a) * black_box(&b));
        black_box(&prod);
        println!(
            "{:<8} {:<18} {:>12} {:>14} {:>12} {:>12} {:>12}",
            n,
            "single_mul",
            s.alloc_calls,
            s.dealloc_calls,
            s.bytes_alloc,
            s.peak_bytes,
            s.live_bytes
        );

        // 16-step chained product (acc = acc * b, 16 times)
        let (acc, s) = measure(|| {
            let mut acc = a.clone();
            for _ in 0..16 {
                acc = black_box(&acc) * black_box(&b);
            }
            acc
        });
        black_box(&acc);
        println!(
            "{:<8} {:<18} {:>12} {:>14} {:>12} {:>12} {:>12}",
            n,
            "chain16_mul",
            s.alloc_calls,
            s.dealloc_calls,
            s.bytes_alloc,
            s.peak_bytes,
            s.live_bytes
        );
    }
}
