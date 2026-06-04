//! SIMD-accelerated (and scalar-fallback) inner kernels for bitwise and shift
//! operations on `BigUint` limb slices.
//!
//! # SIMD vs scalar selection
//!
//! All functions in this module have two exclusive implementations controlled
//! by `#[cfg(oxinum_simd)]`.  The `oxinum_simd` cfg is emitted by `build.rs`
//! only when **both** the `simd` Cargo feature is enabled **and** the compiler
//! is a nightly build.  On stable the `simd` feature may be present, but
//! `build.rs` will not emit the cfg, so the scalar fallback activates.  This
//! guarantees that `--all-features` on stable CI is always green.
//!
//! # Bitwise kernels
//!
//! - `and_limbs(a, b)` → min-length AND (higher limbs of the longer operand
//!   disappear — correct because `0 & x == 0`).
//! - `or_limbs(a, b)` → max-length OR (higher limbs of the longer operand are
//!   kept — correct because `0 | x == x`).
//! - `xor_limbs(a, b)` → max-length XOR, then normalized.
//!
//! # Shift kernels
//!
//! These handle only the *within-limb* (`bit_offset ∈ 1..=63`) portion of a
//! shift; the caller handles whole-limb offsets.
//!
//! - `shl_within(a, s)` — shift `a` left by `s` bits (1 ≤ s ≤ 63).
//! - `shr_within(a, s)` — shift `a` right by `s` bits (1 ≤ s ≤ 63).

// ---------------------------------------------------------------------------
// SIMD path (nightly only, activated by oxinum_simd cfg from build.rs)
// ---------------------------------------------------------------------------

#[cfg(oxinum_simd)]
mod inner {
    use core::simd::Simd;

    /// Number of u64 lanes per SIMD vector.
    const LANES: usize = 4;

    /// AND two limb slices.  Result length = `min(a.len(), b.len())`.
    pub(crate) fn and_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
        let len = a.len().min(b.len());
        let mut out: Vec<u64> = Vec::with_capacity(len);
        let mut i = 0usize;
        while i + LANES <= len {
            let av = Simd::<u64, LANES>::from_slice(&a[i..]);
            let bv = Simd::<u64, LANES>::from_slice(&b[i..]);
            out.extend_from_slice(&(av & bv).to_array());
            i += LANES;
        }
        while i < len {
            out.push(a[i] & b[i]);
            i += 1;
        }
        super::super::uint::normalize(&mut out);
        out
    }

    /// OR two limb slices.  Result length = `max(a.len(), b.len())`.
    pub(crate) fn or_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
        let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
        let short_len = short.len();
        let long_len = long.len();
        let mut out: Vec<u64> = Vec::with_capacity(long_len);
        let mut i = 0usize;
        while i + LANES <= short_len {
            let sv = Simd::<u64, LANES>::from_slice(&short[i..]);
            let lv = Simd::<u64, LANES>::from_slice(&long[i..]);
            out.extend_from_slice(&(sv | lv).to_array());
            i += LANES;
        }
        while i < short_len {
            out.push(short[i] | long[i]);
            i += 1;
        }
        // Append the remaining limbs from the longer slice unchanged.
        out.extend_from_slice(&long[short_len..]);
        // No normalize needed: OR of non-zero MSB stays non-zero.
        out
    }

    /// XOR two limb slices.  Result length = `max(a.len(), b.len())`, then
    /// normalized (XOR of equal values yields 0, so leading zeros are possible).
    pub(crate) fn xor_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
        let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
        let short_len = short.len();
        let long_len = long.len();
        let mut out: Vec<u64> = Vec::with_capacity(long_len);
        let mut i = 0usize;
        while i + LANES <= short_len {
            let sv = Simd::<u64, LANES>::from_slice(&short[i..]);
            let lv = Simd::<u64, LANES>::from_slice(&long[i..]);
            out.extend_from_slice(&(sv ^ lv).to_array());
            i += LANES;
        }
        while i < short_len {
            out.push(short[i] ^ long[i]);
            i += 1;
        }
        out.extend_from_slice(&long[short_len..]);
        super::super::uint::normalize(&mut out);
        out
    }

    /// Shift a limb slice left by `s` bits (1 ≤ s ≤ 63).
    ///
    /// Handles the *within-limb* portion only.  Returns a new `Vec<u64>` with
    /// the same length as `a` (plus an extra carry limb if needed) and no
    /// trailing zeros.
    pub(crate) fn shl_within(a: &[u64], s: u32) -> Vec<u64> {
        debug_assert!((1..=63).contains(&s), "shl_within: s must be 1..=63");
        let len = a.len();
        let anti = 64u32 - s;
        let mut out: Vec<u64> = Vec::with_capacity(len + 1);

        // The least-significant limb has no predecessor to OR into.
        out.push(a[0] << s);

        // SIMD inner loop: process 4 limbs at once where both cur[i..i+4] and
        // prev[i-1..i+3] are fully in-bounds.  Start at i=1 and require
        // i+LANES <= len (i.e., we need a[i+LANES-1] which is at most a[len-1]).
        let mut i = 1usize;
        while i + LANES <= len {
            // cur = a[i .. i+LANES]
            let cur = Simd::<u64, LANES>::from_slice(&a[i..]);
            // prev = a[i-1 .. i+LANES-1]
            let prev = Simd::<u64, LANES>::from_slice(&a[i - 1..]);
            let shifted = (cur << Simd::splat(s as u64)) | (prev >> Simd::splat(anti as u64));
            out.extend_from_slice(&shifted.to_array());
            i += LANES;
        }
        // Scalar remainder for limbs i..len.
        while i < len {
            out.push((a[i] << s) | (a[i - 1] >> anti));
            i += 1;
        }
        // Carry limb if the MSB had bits shifted out.
        let carry = a[len - 1] >> anti;
        if carry != 0 {
            out.push(carry);
        }
        super::super::uint::normalize(&mut out);
        out
    }

    /// Shift a limb slice right by `s` bits (1 ≤ s ≤ 63).
    ///
    /// Handles the *within-limb* portion only.  Returns a normalized `Vec<u64>`.
    pub(crate) fn shr_within(a: &[u64], s: u32) -> Vec<u64> {
        debug_assert!((1..=63).contains(&s), "shr_within: s must be 1..=63");
        let len = a.len();
        let anti = 64u32 - s;
        let mut out: Vec<u64> = Vec::with_capacity(len);

        // SIMD inner loop: cur=a[i..i+4], next=a[i+1..i+5].
        // We need a[i+LANES] to be valid, so condition is i + LANES + 1 <= len,
        // i.e., len >= i + LANES + 1.  Start at i=0.
        let mut i = 0usize;
        while i + LANES < len {
            let cur = Simd::<u64, LANES>::from_slice(&a[i..]);
            let next = Simd::<u64, LANES>::from_slice(&a[i + 1..]);
            let shifted = (cur >> Simd::splat(s as u64)) | (next << Simd::splat(anti as u64));
            out.extend_from_slice(&shifted.to_array());
            i += LANES;
        }
        // Scalar remainder for limbs i..len-1.
        while i < len - 1 {
            out.push((a[i] >> s) | (a[i + 1] << anti));
            i += 1;
        }
        // Final limb: no successor to OR in.
        out.push(a[len - 1] >> s);

        super::super::uint::normalize(&mut out);
        out
    }
}

// ---------------------------------------------------------------------------
// Scalar fallback path (stable / no SIMD)
// ---------------------------------------------------------------------------

#[cfg(not(oxinum_simd))]
mod inner {
    /// AND two limb slices.  Result length = `min(a.len(), b.len())`.
    pub(crate) fn and_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
        let mut out: Vec<u64> = a.iter().zip(b.iter()).map(|(&x, &y)| x & y).collect();
        super::super::uint::normalize(&mut out);
        out
    }

    /// OR two limb slices.  Result length = `max(a.len(), b.len())`.
    pub(crate) fn or_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
        let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
        let mut out: Vec<u64> = short
            .iter()
            .zip(long.iter())
            .map(|(&x, &y)| x | y)
            .collect();
        out.extend_from_slice(&long[short.len()..]);
        // No normalize needed: OR of non-zero MSB stays non-zero.
        out
    }

    /// XOR two limb slices.  Result length = `max(a.len(), b.len())`, normalized.
    pub(crate) fn xor_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
        let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
        let mut out: Vec<u64> = short
            .iter()
            .zip(long.iter())
            .map(|(&x, &y)| x ^ y)
            .collect();
        out.extend_from_slice(&long[short.len()..]);
        super::super::uint::normalize(&mut out);
        out
    }

    /// Shift a limb slice left by `s` bits (1 ≤ s ≤ 63).
    pub(crate) fn shl_within(a: &[u64], s: u32) -> Vec<u64> {
        debug_assert!((1..=63).contains(&s), "shl_within: s must be 1..=63");
        let anti = 64u32 - s;
        let len = a.len();
        let mut out: Vec<u64> = Vec::with_capacity(len + 1);
        out.push(a[0] << s);
        for i in 1..len {
            out.push((a[i] << s) | (a[i - 1] >> anti));
        }
        let carry = a[len - 1] >> anti;
        if carry != 0 {
            out.push(carry);
        }
        super::super::uint::normalize(&mut out);
        out
    }

    /// Shift a limb slice right by `s` bits (1 ≤ s ≤ 63).
    pub(crate) fn shr_within(a: &[u64], s: u32) -> Vec<u64> {
        debug_assert!((1..=63).contains(&s), "shr_within: s must be 1..=63");
        let anti = 64u32 - s;
        let len = a.len();
        let mut out: Vec<u64> = Vec::with_capacity(len);
        for i in 0..len - 1 {
            out.push((a[i] >> s) | (a[i + 1] << anti));
        }
        out.push(a[len - 1] >> s);
        super::super::uint::normalize(&mut out);
        out
    }
}

// Re-export the selected inner impl at module level.
pub(crate) use inner::{and_limbs, or_limbs, shl_within, shr_within, xor_limbs};
