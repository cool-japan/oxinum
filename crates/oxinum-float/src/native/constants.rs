//! Arbitrary-precision mathematical constants for native `BigFloat`.
//!
//! Provides [`pi`], [`e_const`], and [`ln2`] computed to arbitrary precision
//! via integer-arithmetic algorithms (binary splitting / iterative atanh sums).
//!
//! # Algorithms
//!
//! ## π — Chudnovsky formula + binary splitting
//!
//! The Chudnovsky series gives ≈14.18 decimal digits per term:
//!
//! ```text
//! 1/π = (12 / C^(3/2)) · Σ_{k=0}^∞ (−1)^k · (6k)! · (Ak+B) / ((3k)! · (k!)³ · C^(3k))
//! ```
//!
//! where `A = 545140134`, `B = 13591409`, `C = 640320`.
//!
//! ## e — 1/k! binary splitting
//!
//! `e = Σ_{k=0}^∞ 1/k!`  with `p(k)=1`, `q(k)=k` (k>0), `a(k)=1`.
//!
//! ## ln 2 — Hwang Machin-like identity
//!
//! `ln 2 = 14·atanh(1/31) + 10·atanh(1/49) + 6·atanh(1/161)`
//!
//! Each `atanh(1/x)` is evaluated by iterative summation with an analytically
//! determined term count.
//!
//! # Caching
//!
//! Results are cached in thread-safe `OnceLock<RwLock<Option<BigFloat>>>`.
//! If the cached value has at least `prec + 16` bits the cache is reused;
//! otherwise the constant is recomputed at `prec + 32` guard bits.

use std::sync::{OnceLock, RwLock};

use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_int::native::BigInt;

use super::binary_splitting::{binary_split, BSSeries, BSSplit};
use super::float::{BigFloat, RoundingMode};

// ---------------------------------------------------------------------------
// Helper: convert a `BigInt` to a `BigFloat` at the requested precision.
// ---------------------------------------------------------------------------

/// Convert a `BigInt` to `BigFloat` at `prec` bits.
///
/// Uses `BigFloat::from_parts` which performs full normalization + rounding.
fn bigfloat_from_bigint(n: &BigInt, prec: u32, mode: RoundingMode) -> BigFloat {
    if n.is_zero() {
        return BigFloat::zero(prec);
    }
    // from_parts with exponent=0 means the integer is interpreted as
    //   (-1)^sign * magnitude * 2^0   = the integer itself (before normalisation).
    // from_parts strips trailing-zero bits (migrates them into the exponent)
    // and then rounds to prec bits.
    BigFloat::from_parts(n.sign(), n.magnitude().clone(), 0, prec, mode)
}

// ---------------------------------------------------------------------------
// π via Chudnovsky + binary splitting
// ---------------------------------------------------------------------------

/// Chudnovsky binary-splitting series.
///
/// Terms defined by:
/// - `k=0`: `p = 1`, `q = 1`, `b = 1`, `a = B = 13591409`
/// - `k>0`: `p = −(6k−5)(6k−4)(6k−3)(6k−2)(6k−1)(6k)`,
///   `q = (3k)(3k−1)(3k−2) · k³ · C³` where `C = 640320`,
///   `b = 1`, `a = A·k + B` where `A = 545140134`
///
/// After binary splitting over N terms, the result `t/(q·b)` equals
/// `Σ_k c_k` where `1/π = 12 · Σ_k c_k / sqrt(C³)`.
struct Chudnovsky;

impl BSSeries for Chudnovsky {
    fn term(&self, k: u64) -> (BigInt, BigInt, BigInt, BigInt) {
        const A: i64 = 545_140_134;
        const B: i64 = 13_591_409;
        const C3: i64 = 640_320_i64 * 640_320 * 640_320; // 262_537_412_640_768_000

        let a_val = BigInt::from(A * k as i64 + B);

        if k == 0 {
            return (BigInt::one(), BigInt::one(), BigInt::one(), a_val);
        }

        // p_k = −(6k−5)(6k−4)(6k−3)(6k−2)(6k−1)(6k)
        // This is the signed numerator factor encoding the (6k)! ratio.
        // Compute each factor as BigInt to avoid overflow for large k.
        let k6 = BigInt::from((k * 6) as i64);
        let p_val = {
            let f1 = &k6 - BigInt::from(5i64);
            let f2 = &k6 - BigInt::from(4i64);
            let f3 = &k6 - BigInt::from(3i64);
            let f4 = &k6 - BigInt::from(2i64);
            let f5 = &k6 - BigInt::from(1i64);
            let f6 = k6;
            let prod = &f1 * &f2 * &f3 * &f4 * &f5 * &f6;
            -prod
        };

        // q_k = (3k)(3k−1)(3k−2) · k³ · C³
        // The (3k)(3k−1)(3k−2) factor encodes the (3k)! denominator ratio.
        let k3 = BigInt::from((k * 3) as i64);
        let kb = BigInt::from(k as i64);
        let trinom = &k3 * (&k3 - BigInt::from(1i64)) * (&k3 - BigInt::from(2i64));
        let q_val = &trinom * &kb * &kb * &kb * BigInt::from(C3);

        let b_val = BigInt::one();

        (p_val, q_val, b_val, a_val)
    }
}

/// Compute the number of Chudnovsky terms needed for `prec` bits.
///
/// Each term contributes ≈ 14.181 decimal digits ≈ 47.11 bits.
fn chudnovsky_terms(prec: u32) -> u64 {
    // bits_per_term ≈ 14.181 * log2(10) ≈ 14.181 * 3.32193 ≈ 47.11
    let n = ((prec as f64) / 47.11) as u64 + 8;
    n.max(2)
}

/// Compute π using Chudnovsky binary splitting at `work_prec` bits.
fn compute_pi_at(work_prec: u32) -> OxiNumResult<BigFloat> {
    let mode = RoundingMode::HalfEven;
    let n = chudnovsky_terms(work_prec);

    let split: BSSplit = binary_split(&Chudnovsky, 0, n);

    // After binary splitting:
    //   split.t / (split.q * split.b)  =  Σ_k c_k
    //   1/π = 12 · Σ_k c_k / sqrt(C³)
    //   π   = sqrt(C³) · split.q · split.b / (12 · split.t)
    //
    // Note: split.t can be negative (Chudnovsky alternates).
    // We compute |π| and track sign separately.

    // C³ = 640320³ = 262_537_412_640_768_000  < i64::MAX (9.2e18). Use i64.
    const C3: i64 = 640_320_i64 * 640_320 * 640_320;
    let c3_float = BigFloat::from_i64(C3, work_prec, mode);
    let sqrt_c3 = c3_float.sqrt(work_prec, mode)?;

    // Denominator: 12 · split.t
    let twelve = BigInt::from(12i64);
    let denom_int: BigInt = &twelve * &split.t;

    // Numerator integer: split.q · split.b
    let numer_int: BigInt = &split.q * &split.b;

    // Convert to BigFloat.
    let numer_f = bigfloat_from_bigint(&numer_int, work_prec, mode);
    let denom_f = bigfloat_from_bigint(&denom_int, work_prec, mode);

    // Numerator in floating-point: sqrt_c3 * numer_f
    let full_numer = &sqrt_c3 * &numer_f;

    // π = full_numer / denom_f.
    // Both numer_int = Q*B and denom_int = 12*T must be positive for Chudnovsky.
    // A sign mismatch here would indicate a bug in the term encoding.
    let pi_raw = full_numer.div_ref(&denom_f)?;

    if pi_raw.sign() == oxinum_core::Sign::Negative {
        return Err(OxiNumError::Precision(
            "pi computation yielded negative result — Chudnovsky term encoding bug".into(),
        ));
    }

    Ok(pi_raw)
}

// ---------------------------------------------------------------------------
// e via 1/k! binary splitting
// ---------------------------------------------------------------------------

/// Series for `e = Σ_{k=0}^∞ 1/k!`.
///
/// Per-term factors:
/// - `k=0`: `p = 1`, `q = 1`, `b = 1`, `a = 1`
/// - `k>0`: `p = 1`, `q = k`, `b = 1`, `a = 1`
///
/// After binary splitting: `t / (q · b) = e − epsilon` for N large enough.
struct ESeries;

impl BSSeries for ESeries {
    fn term(&self, k: u64) -> (BigInt, BigInt, BigInt, BigInt) {
        let one = BigInt::one();
        if k == 0 {
            (one.clone(), one.clone(), one.clone(), one)
        } else {
            (one.clone(), BigInt::from(k as i64), one.clone(), one)
        }
    }
}

/// Number of terms for `e` at `work_prec` bits.
///
/// We need `k` large enough so `log2(k!) > work_prec + 8`.
/// Uses the analytic estimate `log2(k!) ≈ k log2(k) - k/ln(2)`.
fn e_terms(work_prec: u32) -> u64 {
    let target = (work_prec as f64) + 8.0;
    let mut k: u64 = 4;
    let mut log2_kfact: f64 = (1..=4u64).map(|i| (i as f64).log2()).sum();
    while log2_kfact < target {
        k += 1;
        log2_kfact += (k as f64).log2();
    }
    k + 1
}

/// Compute e using binary splitting at `work_prec` bits.
fn compute_e_at(work_prec: u32) -> OxiNumResult<BigFloat> {
    let mode = RoundingMode::HalfEven;
    let n = e_terms(work_prec);

    let split: BSSplit = binary_split(&ESeries, 0, n);

    // sum = t / (q · b)
    let denom_int: BigInt = &split.q * &split.b;
    let numer_f = bigfloat_from_bigint(&split.t, work_prec, mode);
    let denom_f = bigfloat_from_bigint(&denom_int, work_prec, mode);

    numer_f.div_ref(&denom_f)
}

// ---------------------------------------------------------------------------
// ln 2 via Machin-like atanh identity
// ---------------------------------------------------------------------------

/// Compute `atanh(1/x)` at `work_prec` bits using the series
/// `atanh(1/x) = 1/x + 1/(3x³) + 1/(5x⁵) + …`
///
/// Based on Hwang's identity `ln 2 = 14·atanh(1/31) + 10·atanh(1/49) + 6·atanh(1/161)`.
///
/// Term k has magnitude `1/((2k+1)·x^(2k+1))`.
/// We need enough terms so `(2k+1)·log2(x) > work_prec + 8`.
fn atanh_inv_x(x: u64, work_prec: u32) -> OxiNumResult<BigFloat> {
    let mode = RoundingMode::HalfEven;

    // Number of terms: smallest k such that (2k+1)*log2(x) > work_prec + 8.
    let log2_x = (x as f64).log2();
    let k_needed = (((work_prec as f64) + 8.0) / log2_x / 2.0) as u64 + 4;

    // We compute the sum iteratively using BigFloat arithmetic.
    // atanh(1/x) = (1/x) * Σ_{k=0}^{N} (1/x²)^k / (2k+1)
    //
    // Iterate: term_0 = 1/x.  For each k>0: term_k = term_{k-1} / x².
    // Then add term_k / (2k+1) to accumulator.

    let x_f = BigFloat::from_i64(x as i64, work_prec, mode);
    let x2_f = &x_f * &x_f;

    // term = 1/x
    let one_f = BigFloat::from_i64(1, work_prec, mode);
    let mut term = one_f.div_ref_with_mode(&x_f, mode)?;

    // acc = term / (2*0+1) = term / 1 = term
    let mut acc = term.clone();

    for k in 1..=k_needed {
        // term /= x²
        term = term.div_ref_with_mode(&x2_f, mode)?;
        // scaled_term = term / (2k+1)
        let denom_f = BigFloat::from_i64((2 * k + 1) as i64, work_prec, mode);
        let scaled = term.div_ref_with_mode(&denom_f, mode)?;
        acc = &acc + &scaled;
    }

    Ok(acc)
}

/// Compute ln 2 using Hwang's identity at `work_prec` bits.
fn compute_ln2_at(work_prec: u32) -> OxiNumResult<BigFloat> {
    let mode = RoundingMode::HalfEven;

    // ln 2 = 14·atanh(1/31) + 10·atanh(1/49) + 6·atanh(1/161)
    let a31 = atanh_inv_x(31, work_prec)?;
    let a49 = atanh_inv_x(49, work_prec)?;
    let a161 = atanh_inv_x(161, work_prec)?;

    let c14 = BigFloat::from_i64(14, work_prec, mode);
    let c10 = BigFloat::from_i64(10, work_prec, mode);
    let c6 = BigFloat::from_i64(6, work_prec, mode);

    let t1 = &c14 * &a31;
    let t2 = &c10 * &a49;
    let t3 = &c6 * &a161;

    Ok(&(&t1 + &t2) + &t3)
}

// ---------------------------------------------------------------------------
// Caching wrappers
// ---------------------------------------------------------------------------

/// Cache cell for a single constant.
struct ConstCache {
    inner: OnceLock<RwLock<Option<BigFloat>>>,
}

impl ConstCache {
    const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    fn lock(&self) -> &RwLock<Option<BigFloat>> {
        self.inner.get_or_init(|| RwLock::new(None))
    }

    /// Return the constant at `prec` bits, using or populating the cache.
    ///
    /// `compute` is called with `work_prec = prec + 32` when the cache is
    /// cold or the cached precision is insufficient.
    fn get_or_compute<F>(&self, prec: u32, compute: F) -> OxiNumResult<BigFloat>
    where
        F: FnOnce(u32) -> OxiNumResult<BigFloat>,
    {
        const MODE: RoundingMode = RoundingMode::HalfEven;

        // Fast path: read-lock check.
        {
            let guard = self
                .lock()
                .read()
                .map_err(|_| OxiNumError::Precision("ConstCache RwLock poisoned".into()))?;
            if let Some(ref cached) = *guard {
                if cached.precision() >= prec + 16 {
                    return Ok(cached.clone().with_precision(prec, MODE));
                }
            }
        }

        // Slow path: compute at higher precision and write.
        let work_prec = prec + 32;
        let result = compute(work_prec)?;

        {
            let mut guard = self
                .lock()
                .write()
                .map_err(|_| OxiNumError::Precision("ConstCache RwLock poisoned".into()))?;
            // Double-check: another thread may have updated the cache.
            if let Some(ref cached) = *guard {
                if cached.precision() >= prec + 16 {
                    return Ok(cached.clone().with_precision(prec, MODE));
                }
            }
            *guard = Some(result);
        }

        // Now re-read from the cache to avoid double-cloning.
        {
            let guard = self
                .lock()
                .read()
                .map_err(|_| OxiNumError::Precision("ConstCache RwLock poisoned".into()))?;
            if let Some(ref cached) = *guard {
                return Ok(cached.clone().with_precision(prec, MODE));
            }
        }

        Err(OxiNumError::Precision(
            "ConstCache: unexpected empty after write".into(),
        ))
    }
}

static PI_CACHE: ConstCache = ConstCache::new();
static E_CACHE: ConstCache = ConstCache::new();
static LN2_CACHE: ConstCache = ConstCache::new();

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return π at `prec` bits of precision.
///
/// # Errors
///
/// Propagates any arithmetic error from the internal Chudnovsky computation
/// (in practice, only a sqrt-of-negative error which should never occur).
///
/// # Examples
///
/// ```
/// use oxinum_float::native::{pi, BigFloat, RoundingMode};
/// let p = pi(64).expect("pi");
/// assert!((p.to_f64() - std::f64::consts::PI).abs() < 1e-14);
/// ```
pub fn pi(prec: u32) -> OxiNumResult<BigFloat> {
    PI_CACHE.get_or_compute(prec, compute_pi_at)
}

/// Return e (Euler's number) at `prec` bits of precision.
///
/// # Errors
///
/// Propagates any arithmetic error from the internal 1/k! computation.
///
/// # Examples
///
/// ```
/// use oxinum_float::native::e_const;
/// let e = e_const(64).expect("e");
/// assert!((e.to_f64() - std::f64::consts::E).abs() < 1e-14);
/// ```
pub fn e_const(prec: u32) -> OxiNumResult<BigFloat> {
    E_CACHE.get_or_compute(prec, compute_e_at)
}

/// Return ln 2 at `prec` bits of precision.
///
/// # Errors
///
/// Propagates any arithmetic error from the internal atanh series.
///
/// # Examples
///
/// ```
/// use oxinum_float::native::ln2;
/// let l = ln2(64).expect("ln2");
/// assert!((l.to_f64() - std::f64::consts::LN_2).abs() < 1e-14);
/// ```
pub fn ln2(prec: u32) -> OxiNumResult<BigFloat> {
    LN2_CACHE.get_or_compute(prec, compute_ln2_at)
}

// ---------------------------------------------------------------------------
// Internal unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that with 1 Chudnovsky term, the raw sum equals T/Q ≈ 1/(π·12/sqrt(C³))
    /// ≈ 0.02654 (1/π ≈ 0.3183, times 12/sqrt(C³) ≈ 1.2/5.12e8 = 2.34e-9 — wait,
    /// recalculate: 12/sqrt(C³) ≈ 12/(5.124e8) ≈ 2.34e-8, so sum term_0 ≈ 1/(π·12/sqrt) —
    /// just verify the sign structure instead.)
    #[test]
    fn chudnovsky_term_zero() {
        let (p, q, b, a) = Chudnovsky.term(0);
        assert_eq!(p, BigInt::one());
        assert_eq!(q, BigInt::one());
        assert_eq!(b, BigInt::one());
        assert_eq!(a, BigInt::from(13_591_409i64));
    }

    #[test]
    fn chudnovsky_term_one_negative_p() {
        let (p, _q, _b, _a) = Chudnovsky.term(1);
        // p(1) = -(6*1-5)(6*1-4)(6*1-3)(6*1-2)(6*1-1)(6*1)
        //       = -(1)(2)(3)(4)(5)(6) = -720
        assert_eq!(p, BigInt::from(-720i64));
    }

    #[test]
    fn e_series_term() {
        let (p, q, b, a) = ESeries.term(0);
        assert_eq!(p, BigInt::one());
        assert_eq!(q, BigInt::one());
        assert_eq!(b, BigInt::one());
        assert_eq!(a, BigInt::one());

        let (p2, q2, _b2, a2) = ESeries.term(5);
        assert_eq!(p2, BigInt::one());
        assert_eq!(q2, BigInt::from(5i64));
        assert_eq!(a2, BigInt::one());
    }

    #[test]
    fn pi_f64_matches() {
        let p = pi(64).expect("pi(64)");
        assert!((p.to_f64() - std::f64::consts::PI).abs() < 1e-14);
    }

    #[test]
    fn e_f64_matches() {
        let e = e_const(64).expect("e_const(64)");
        assert!((e.to_f64() - std::f64::consts::E).abs() < 1e-14);
    }

    #[test]
    fn ln2_f64_matches() {
        let l = ln2(64).expect("ln2(64)");
        assert!((l.to_f64() - std::f64::consts::LN_2).abs() < 1e-14);
    }
}
