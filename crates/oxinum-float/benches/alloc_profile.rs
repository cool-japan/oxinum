// Allocation-profiling bench for native BigFloat::exp / sin / cos.
//
// This is a `harness = false` binary, so it has its own `main()` and may
// contain `unsafe` code (bench binaries do NOT inherit the library's
// `#![forbid(unsafe_code)]`).
//
// A custom `GlobalAlloc` wraps the system allocator and counts allocation
// calls, bytes, and live/peak bytes.  The `measure(||…)` helper resets
// counters before the closure and snapshots them after.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

// ---------------------------------------------------------------------------
// Counting allocator
// ---------------------------------------------------------------------------

struct CountingAlloc;

static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES_ALLOC: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Safety: forwarding directly to the system allocator.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOC_CALLS.fetch_add(1, Relaxed);
            BYTES_ALLOC.fetch_add(layout.size(), Relaxed);
            let live = LIVE_BYTES.fetch_add(layout.size(), Relaxed) + layout.size();
            // Track peak live bytes.
            let _ = PEAK_BYTES.fetch_max(live, Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Safety: forwarding to the same system allocator that performed alloc.
        unsafe { System.dealloc(ptr, layout) };
        DEALLOC_CALLS.fetch_add(1, Relaxed);
        LIVE_BYTES.fetch_sub(layout.size(), Relaxed);
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
    // Restart peak from the current live baseline so we measure the delta.
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

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    use oxinum_float::native::{BigFloat, RoundingMode};
    let mode = RoundingMode::HalfEven;

    println!(
        "{:<8} {:<16} {:>12} {:>14} {:>12} {:>12} {:>12}",
        "prec", "op", "alloc_calls", "dealloc_calls", "bytes_alloc", "peak_bytes", "live_bytes"
    );
    println!("{}", "-".repeat(90));

    for prec in [64u32, 256, 1024] {
        let x = BigFloat::from_i64(2, prec, mode);

        // exp(2)
        let (_, s) = measure(|| black_box(&x).exp(prec, mode).expect("exp"));
        println!(
            "{:<8} {:<16} {:>12} {:>14} {:>12} {:>12} {:>12}",
            prec, "exp", s.alloc_calls, s.dealloc_calls, s.bytes_alloc, s.peak_bytes, s.live_bytes
        );

        // sin(2)
        let (_, s) = measure(|| black_box(&x).sin(prec, mode).expect("sin"));
        println!(
            "{:<8} {:<16} {:>12} {:>14} {:>12} {:>12} {:>12}",
            prec, "sin", s.alloc_calls, s.dealloc_calls, s.bytes_alloc, s.peak_bytes, s.live_bytes
        );

        // cos(2)
        let (_, s) = measure(|| black_box(&x).cos(prec, mode).expect("cos"));
        println!(
            "{:<8} {:<16} {:>12} {:>14} {:>12} {:>12} {:>12}",
            prec, "cos", s.alloc_calls, s.dealloc_calls, s.bytes_alloc, s.peak_bytes, s.live_bytes
        );

        // exp(sin(cos(x)))
        let (_, s) = measure(|| {
            let a = black_box(&x).cos(prec, mode).expect("cos");
            let b = a.sin(prec, mode).expect("sin");
            b.exp(prec, mode).expect("exp")
        });
        println!(
            "{:<8} {:<16} {:>12} {:>14} {:>12} {:>12} {:>12}",
            prec,
            "exp(sin(cos))",
            s.alloc_calls,
            s.dealloc_calls,
            s.bytes_alloc,
            s.peak_bytes,
            s.live_bytes
        );
    }
}
