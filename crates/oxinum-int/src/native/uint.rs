//! Native `BigUint` — little-endian `Vec<u64>`-limb arbitrary-precision
//! unsigned integer.
//!
//! Invariants (enforced by every public constructor and after every operation
//! that mutates `limbs` in place):
//!
//! - `limbs` is little-endian: `limbs[0]` is the least-significant 64-bit limb.
//! - No trailing-zero limbs: `limbs.last() != Some(&0)`.
//! - Canonical zero: `limbs.is_empty()`.

use core::cmp::Ordering;
use core::ops::{BitAnd, BitOr, BitXor, Shl, ShlAssign, Shr, ShrAssign};

/// Threshold (in limbs) above which Karatsuba multiplication is used.
///
/// Below this threshold, schoolbook multiplication is used. Empirically
/// ~32 limbs is a good crossover on 64-bit hardware for OxiNum's working set.
pub const KARATSUBA_THRESHOLD: usize = 32;

/// Threshold (in limbs) at or above which Toom-Cook-3 multiplication is used.
///
/// Between [`KARATSUBA_THRESHOLD`] and this value, Karatsuba is used. At or
/// above this value the asymptotically faster Toom-3 (O(n^1.465)) takes over.
/// The dispatch gates on `min(a.len, b.len)` so both operands must be large.
/// Empirically ~100 limbs is a reasonable crossover on 64-bit hardware.
pub(crate) const TOOM3_THRESHOLD: usize = 100;

/// Native arbitrary-precision unsigned integer.
///
/// Stored as a little-endian `Vec<u64>` with no trailing-zero limbs.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::BigUint;
///
/// let a = BigUint::from_u64(42);
/// let b = BigUint::from_u64(58);
/// let sum = &a + &b;
/// assert_eq!(sum, BigUint::from_u64(100));
/// ```
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BigUint {
    /// Little-endian limbs. No trailing zeros. Empty `Vec` = zero.
    pub(crate) limbs: Vec<u64>,
}

impl BigUint {
    /// The canonical zero value.
    pub const ZERO: BigUint = BigUint { limbs: Vec::new() };

    /// Construct a zero `BigUint`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert!(BigUint::zero().is_zero());
    /// ```
    #[inline]
    pub fn zero() -> Self {
        Self { limbs: Vec::new() }
    }

    /// Construct a `BigUint` equal to `1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert!(BigUint::one().is_one());
    /// ```
    #[inline]
    pub fn one() -> Self {
        Self { limbs: vec![1] }
    }

    /// Construct from a primitive `u64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_u64(42);
    /// assert_eq!(n.to_u64(), Some(42));
    /// ```
    #[inline]
    pub fn from_u64(value: u64) -> Self {
        if value == 0 {
            Self::zero()
        } else {
            Self { limbs: vec![value] }
        }
    }

    /// Construct from a primitive `u128`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let huge = BigUint::from_u128(u128::MAX);
    /// assert_eq!(huge.bit_length(), 128);
    /// ```
    #[inline]
    pub fn from_u128(value: u128) -> Self {
        let lo = value as u64;
        let hi = (value >> 64) as u64;
        if hi == 0 {
            Self::from_u64(lo)
        } else {
            Self {
                limbs: vec![lo, hi],
            }
        }
    }

    /// Construct from a slice of little-endian limbs (normalizing input).
    ///
    /// The input is little-endian: `limbs[0]` is the least-significant limb.
    /// Trailing zeros are stripped.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_le_limbs(&[5, 0, 0]);
    /// assert_eq!(n, BigUint::from_u64(5));
    /// ```
    pub fn from_le_limbs(limbs: &[u64]) -> Self {
        let mut v = limbs.to_vec();
        normalize(&mut v);
        // Shrink excess capacity created by trailing-zero removal so that the
        // returned value has minimal heap footprint (cache-line friendly).
        v.shrink_to_fit();
        Self { limbs: v }
    }

    /// Construct from little-endian limbs while reserving `extra_capacity`
    /// additional limb slots.
    ///
    /// Use this when the caller knows the value will grow (e.g. during an
    /// add loop) to avoid repeated reallocations while keeping contiguous
    /// memory access.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// // Construct a 1-limb value with room for 4 more limbs.
    /// let n = BigUint::from_le_limbs_with_capacity(&[42], 4);
    /// assert_eq!(n, BigUint::from_u64(42));
    /// ```
    pub fn from_le_limbs_with_capacity(limbs: &[u64], extra_capacity: usize) -> Self {
        // Compute the normalized length first so we can reserve exactly right.
        let sig_len = limbs
            .iter()
            .rposition(|&x| x != 0)
            .map(|p| p + 1)
            .unwrap_or(0);
        let mut v = Vec::with_capacity(sig_len + extra_capacity);
        v.extend_from_slice(&limbs[..sig_len]);
        Self { limbs: v }
    }

    /// Release excess heap capacity, minimizing the memory footprint of this
    /// value.
    ///
    /// After a long chain of arithmetic operations the internal `Vec` may hold
    /// significantly more capacity than its current length.  `compact()` shrinks
    /// the allocation to the exact size needed.  This is a cache-friendly hint:
    /// a compacted `BigUint` fits into fewer cache lines during iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let mut n = BigUint::from_u64(42);
    /// n.compact();
    /// assert_eq!(n, BigUint::from_u64(42));
    /// ```
    #[inline]
    pub fn compact(&mut self) {
        self.limbs.shrink_to_fit();
    }

    /// Returns the raw little-endian limbs (no trailing zeros).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_u64(0xDEAD_BEEF);
    /// assert_eq!(n.as_limbs(), &[0xDEAD_BEEFu64]);
    /// ```
    #[inline]
    pub fn as_limbs(&self) -> &[u64] {
        &self.limbs
    }

    /// Returns `true` if this value is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert!(BigUint::zero().is_zero());
    /// assert!(!BigUint::one().is_zero());
    /// ```
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Returns `true` if this value is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert!(BigUint::one().is_one());
    /// ```
    #[inline]
    pub fn is_one(&self) -> bool {
        self.limbs.as_slice() == [1u64]
    }

    /// Try to convert to a `u64`, returning `None` on overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(42).to_u64(), Some(42));
    /// ```
    #[inline]
    pub fn to_u64(&self) -> Option<u64> {
        match self.limbs.len() {
            0 => Some(0),
            1 => Some(self.limbs[0]),
            _ => None,
        }
    }

    /// Returns the number of significant bits (`0` for zero).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::zero().bit_length(), 0);
    /// assert_eq!(BigUint::from_u64(1).bit_length(), 1);
    /// assert_eq!(BigUint::from_u64(0xFF).bit_length(), 8);
    /// ```
    pub fn bit_length(&self) -> u64 {
        match self.limbs.last() {
            None => 0,
            Some(&top) => {
                let n_limbs = self.limbs.len() as u64;
                (n_limbs - 1) * 64 + (64 - top.leading_zeros() as u64)
            }
        }
    }

    /// Returns the number of trailing zero bits.
    ///
    /// By convention, returns `0` for zero (no bits to count).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(0b1000).trailing_zeros(), 3);
    /// assert_eq!(BigUint::zero().trailing_zeros(), 0);
    /// ```
    pub fn trailing_zeros(&self) -> u64 {
        for (i, &limb) in self.limbs.iter().enumerate() {
            if limb != 0 {
                return (i as u64) * 64 + (limb.trailing_zeros() as u64);
            }
        }
        0
    }

    /// Returns the total count of set bits (population count).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(0b1011).count_ones(), 3);
    /// ```
    pub fn count_ones(&self) -> u64 {
        self.limbs.iter().map(|l| l.count_ones() as u64).sum()
    }

    /// Returns the value of bit `index` (LSB-indexed).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_u64(0b1010);
    /// assert!(!n.test_bit(0));
    /// assert!(n.test_bit(1));
    /// assert!(!n.test_bit(2));
    /// assert!(n.test_bit(3));
    /// ```
    pub fn test_bit(&self, index: u64) -> bool {
        let limb_idx = (index / 64) as usize;
        let bit_idx = index % 64;
        if limb_idx >= self.limbs.len() {
            return false;
        }
        (self.limbs[limb_idx] >> bit_idx) & 1 == 1
    }

    /// Sets bit `index` (LSB-indexed) to 1.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let mut n = BigUint::zero();
    /// n.set_bit(5);
    /// assert_eq!(n, BigUint::from_u64(32));
    /// ```
    pub fn set_bit(&mut self, index: u64) {
        let limb_idx = (index / 64) as usize;
        let bit_idx = index % 64;
        if limb_idx >= self.limbs.len() {
            self.limbs.resize(limb_idx + 1, 0);
        }
        self.limbs[limb_idx] |= 1u64 << bit_idx;
        // Setting a bit can never create a trailing zero, but resize might have
        // introduced trailing zeros if the new limb itself ended up zero (which
        // is impossible here because we just OR'd a non-zero bit into it).
        normalize(&mut self.limbs);
    }

    /// Clears bit `index` (LSB-indexed).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let mut n = BigUint::from_u64(0b1010);
    /// n.clear_bit(1);
    /// assert_eq!(n, BigUint::from_u64(0b1000));
    /// ```
    pub fn clear_bit(&mut self, index: u64) {
        let limb_idx = (index / 64) as usize;
        let bit_idx = index % 64;
        if limb_idx >= self.limbs.len() {
            return;
        }
        self.limbs[limb_idx] &= !(1u64 << bit_idx);
        normalize(&mut self.limbs);
    }

    /// Construct from a big-endian byte sequence (most-significant byte first).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_bytes_be(&[1, 2, 3]);
    /// assert_eq!(n, BigUint::from_u64(0x010203));
    /// ```
    pub fn from_bytes_be(bytes: &[u8]) -> Self {
        // Reverse and delegate
        let mut le = bytes.to_vec();
        le.reverse();
        Self::from_bytes_le(&le)
    }

    /// Construct from a little-endian byte sequence (least-significant byte first).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_bytes_le(&[3, 2, 1]);
    /// assert_eq!(n, BigUint::from_u64(0x010203));
    /// ```
    pub fn from_bytes_le(bytes: &[u8]) -> Self {
        let n_limbs = bytes.len().div_ceil(8);
        let mut limbs = Vec::with_capacity(n_limbs);
        for chunk in bytes.chunks(8) {
            let mut buf = [0u8; 8];
            buf[..chunk.len()].copy_from_slice(chunk);
            limbs.push(u64::from_le_bytes(buf));
        }
        normalize(&mut limbs);
        Self { limbs }
    }

    /// Convert to a big-endian byte sequence. Returns an empty vec for zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(0x010203).to_bytes_be(), vec![1u8, 2, 3]);
    /// assert!(BigUint::zero().to_bytes_be().is_empty());
    /// ```
    pub fn to_bytes_be(&self) -> Vec<u8> {
        let mut le = self.to_bytes_le();
        le.reverse();
        le
    }

    /// Convert to a little-endian byte sequence. Returns an empty vec for zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(0x010203).to_bytes_le(), vec![3u8, 2, 1]);
    /// ```
    pub fn to_bytes_le(&self) -> Vec<u8> {
        if self.limbs.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<u8> = Vec::with_capacity(self.limbs.len() * 8);
        for &limb in &self.limbs {
            out.extend_from_slice(&limb.to_le_bytes());
        }
        // Strip trailing zero bytes from the top.
        while out.last() == Some(&0) {
            out.pop();
        }
        out
    }

    // -----------------------------------------------------------------------
    // Internal helpers (crate-visible) — addition, subtraction, shifts
    // -----------------------------------------------------------------------

    /// Add two `BigUint`s. Always succeeds.
    ///
    /// Structured as two sequential passes for cache-friendly linear access:
    ///
    /// 1. Overlap region: add limbs from both operands.
    /// 2. Tail of the longer operand: propagate carry only.
    ///
    /// This avoids the per-iteration branch `if i < short.limbs.len()`.
    pub(crate) fn add_ref(a: &BigUint, b: &BigUint) -> BigUint {
        // Ensure `long` is the longer of the two for sequential tail processing.
        let (long, short) = if a.limbs.len() >= b.limbs.len() {
            (&a.limbs, &b.limbs)
        } else {
            (&b.limbs, &a.limbs)
        };
        // Exact capacity: at most one carry limb beyond the longer operand.
        let mut out: Vec<u64> = Vec::with_capacity(long.len() + 1);

        // Pass 1: overlap region — add corresponding limbs.
        let mut carry: u64 = 0;
        for (&lv, &sv) in long[..short.len()].iter().zip(short.iter()) {
            let (s1, c1) = lv.overflowing_add(sv);
            let (s2, c2) = s1.overflowing_add(carry);
            out.push(s2);
            carry = (c1 as u64) | (c2 as u64);
        }

        // Pass 2: tail of the longer operand — propagate carry.
        for &lv in &long[short.len()..] {
            let (s, c) = lv.overflowing_add(carry);
            out.push(s);
            carry = c as u64;
            // Fast-path: once carry is zero, copy remaining limbs verbatim.
            if carry == 0 {
                out.extend_from_slice(&long[out.len()..]);
                // `normalize` is a no-op here (inputs were normalized), but keep
                // it for safety in case a future caller passes denormalized input.
                normalize(&mut out);
                return BigUint { limbs: out };
            }
        }
        if carry != 0 {
            out.push(carry);
        }
        normalize(&mut out);
        BigUint { limbs: out }
    }

    /// Compute `self - other` if `self >= other`; otherwise `None` (no underflow).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let a = BigUint::from_u64(100);
    /// let b = BigUint::from_u64(40);
    /// assert_eq!(a.checked_sub(&b), Some(BigUint::from_u64(60)));
    /// assert_eq!(b.checked_sub(&a), None);
    /// ```
    pub fn checked_sub(&self, other: &BigUint) -> Option<BigUint> {
        if self.cmp(other) == Ordering::Less {
            return None;
        }
        // self >= other => no underflow.
        // Two sequential passes for cache-friendly linear reads:
        // Pass 1: overlap region — subtract corresponding limbs.
        // Pass 2: tail of self — propagate borrow only.
        let mut out: Vec<u64> = Vec::with_capacity(self.limbs.len());
        let mut borrow: u64 = 0;

        // Pass 1: overlap region.
        for (&av, &bv) in self.limbs[..other.limbs.len()]
            .iter()
            .zip(other.limbs.iter())
        {
            let (d1, b1) = av.overflowing_sub(bv);
            let (d2, b2) = d1.overflowing_sub(borrow);
            out.push(d2);
            borrow = (b1 as u64) | (b2 as u64);
        }

        // Pass 2: tail of self — propagate borrow.
        for &av in &self.limbs[other.limbs.len()..] {
            let (d, b) = av.overflowing_sub(borrow);
            out.push(d);
            borrow = b as u64;
            if borrow == 0 {
                // No further borrow; copy the remaining limbs verbatim.
                out.extend_from_slice(&self.limbs[out.len()..]);
                normalize(&mut out);
                return Some(BigUint { limbs: out });
            }
        }
        debug_assert_eq!(borrow, 0, "checked_sub underflow despite cmp guard");
        normalize(&mut out);
        Some(BigUint { limbs: out })
    }

    /// Logical shift-left by `n` bits, returning a new `BigUint`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let a = BigUint::from_u64(1);
    /// let shifted = a.shl_bits(65);
    /// assert_eq!(shifted.bit_length(), 66);
    /// ```
    pub fn shl_bits(&self, n: u64) -> BigUint {
        if self.is_zero() || n == 0 {
            return self.clone();
        }
        let limb_offset = (n / 64) as usize;
        let bit_offset = (n % 64) as u32;
        // Pre-allocate exact capacity: zero-fill prefix + source limbs + possible
        // carry limb when bit_offset != 0.  This avoids any reallocation.
        let capacity = limb_offset + self.limbs.len() + if bit_offset != 0 { 1 } else { 0 };
        let mut out: Vec<u64> = Vec::with_capacity(capacity);
        // Extend with `limb_offset` zero limbs (the shifted-out region).
        out.resize(limb_offset, 0u64);
        if bit_offset == 0 {
            out.extend_from_slice(&self.limbs);
        } else {
            out.extend(super::simd_ops::shl_within(&self.limbs, bit_offset));
        }
        normalize(&mut out);
        BigUint { limbs: out }
    }

    /// Raise this value to the `exp` power via binary (square-and-multiply)
    /// exponentiation.
    ///
    /// `self.pow(0) == 1` for every `self` (including zero, matching Rust's
    /// `u64::pow` and the mathematical convention used by `dashu_int::UBig`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(2).pow(10), BigUint::from_u64(1024));
    /// assert_eq!(BigUint::from_u64(3).pow(0), BigUint::from_u64(1));
    /// assert_eq!(BigUint::zero().pow(0), BigUint::from_u64(1));
    /// ```
    pub fn pow(&self, exp: u32) -> BigUint {
        if exp == 0 {
            return BigUint::one();
        }
        if self.is_zero() {
            return BigUint::zero();
        }
        if self.is_one() {
            return BigUint::one();
        }
        let mut base = self.clone();
        let mut result = BigUint::one();
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = &result * &base;
            }
            e >>= 1;
            if e > 0 {
                base = &base * &base;
            }
        }
        result
    }

    /// Logical shift-right by `n` bits, returning a new `BigUint`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let a = BigUint::from_u64(0b1100);
    /// let shifted = a.shr_bits(2);
    /// assert_eq!(shifted, BigUint::from_u64(0b11));
    /// ```
    pub fn shr_bits(&self, n: u64) -> BigUint {
        if self.is_zero() || n == 0 {
            return self.clone();
        }
        let limb_offset = (n / 64) as usize;
        let bit_offset = (n % 64) as u32;
        if limb_offset >= self.limbs.len() {
            return BigUint::zero();
        }
        let remaining = &self.limbs[limb_offset..];
        let mut out: Vec<u64> = Vec::with_capacity(remaining.len());
        if bit_offset == 0 {
            out.extend_from_slice(remaining);
        } else {
            out.extend(super::simd_ops::shr_within(remaining, bit_offset));
        }
        normalize(&mut out);
        BigUint { limbs: out }
    }
}

/// Strip trailing-zero limbs. The canonical zero is an empty `Vec`.
///
/// Uses `truncate` to remove a bulk suffix of zeros in a single call rather
/// than repeated `pop()` (both are O(k) for k zeros, but `truncate` avoids
/// the per-iteration bounds-check branch and is friendlier to the branch
/// predictor on long zero suffixes).
#[inline]
pub(crate) fn normalize(limbs: &mut Vec<u64>) {
    let new_len = limbs
        .iter()
        .rposition(|&x| x != 0)
        .map(|p| p + 1)
        .unwrap_or(0);
    limbs.truncate(new_len);
}

// ---------------------------------------------------------------------------
// Equality & ordering
// ---------------------------------------------------------------------------

impl PartialEq for BigUint {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.limbs == other.limbs
    }
}

impl Eq for BigUint {}

impl std::hash::Hash for BigUint {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.limbs.hash(state);
    }
}

impl Ord for BigUint {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by length first (normalized invariant): longer is greater.
        match self.limbs.len().cmp(&other.limbs.len()) {
            Ordering::Equal => {
                // Same length: compare MSB-first.
                for (a, b) in self.limbs.iter().rev().zip(other.limbs.iter().rev()) {
                    match a.cmp(b) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                Ordering::Equal
            }
            non_eq => non_eq,
        }
    }
}

impl PartialOrd for BigUint {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Shl/Shr operators (by u64 bit-count) — owned and borrowed
// ---------------------------------------------------------------------------

impl Shl<u64> for BigUint {
    type Output = BigUint;
    #[inline]
    fn shl(self, n: u64) -> BigUint {
        self.shl_bits(n)
    }
}

impl Shl<u64> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn shl(self, n: u64) -> BigUint {
        self.shl_bits(n)
    }
}

impl Shr<u64> for BigUint {
    type Output = BigUint;
    #[inline]
    fn shr(self, n: u64) -> BigUint {
        self.shr_bits(n)
    }
}

impl Shr<u64> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn shr(self, n: u64) -> BigUint {
        self.shr_bits(n)
    }
}

impl ShlAssign<u64> for BigUint {
    #[inline]
    fn shl_assign(&mut self, n: u64) {
        *self = self.shl_bits(n);
    }
}

impl ShrAssign<u64> for BigUint {
    #[inline]
    fn shr_assign(&mut self, n: u64) {
        *self = self.shr_bits(n);
    }
}

// ---------------------------------------------------------------------------
// Bitwise AND, OR, XOR — delegate to simd_ops kernels (scalar fallback on
// stable, SIMD path on nightly when `--features simd` is passed).
// Assign variants (BitAndAssign/BitOrAssign/BitXorAssign) live in bitwise.rs.
// ---------------------------------------------------------------------------

impl BitAnd<&BigUint> for &BigUint {
    type Output = BigUint;
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let a = BigUint::from_u64(0b1100);
    /// let b = BigUint::from_u64(0b1010);
    /// assert_eq!(&a & &b, BigUint::from_u64(0b1000));
    /// ```
    #[inline]
    fn bitand(self, rhs: &BigUint) -> BigUint {
        BigUint {
            limbs: super::simd_ops::and_limbs(&self.limbs, &rhs.limbs),
        }
    }
}

impl BitAnd<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn bitand(self, rhs: BigUint) -> BigUint {
        (&self).bitand(&rhs)
    }
}

impl BitAnd<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn bitand(self, rhs: &BigUint) -> BigUint {
        (&self).bitand(rhs)
    }
}

impl BitAnd<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn bitand(self, rhs: BigUint) -> BigUint {
        self.bitand(&rhs)
    }
}

impl BitOr<&BigUint> for &BigUint {
    type Output = BigUint;
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let a = BigUint::from_u64(0b1100);
    /// let b = BigUint::from_u64(0b1010);
    /// assert_eq!(&a | &b, BigUint::from_u64(0b1110));
    /// ```
    #[inline]
    fn bitor(self, rhs: &BigUint) -> BigUint {
        BigUint {
            limbs: super::simd_ops::or_limbs(&self.limbs, &rhs.limbs),
        }
    }
}

impl BitOr<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn bitor(self, rhs: BigUint) -> BigUint {
        (&self).bitor(&rhs)
    }
}

impl BitOr<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn bitor(self, rhs: &BigUint) -> BigUint {
        (&self).bitor(rhs)
    }
}

impl BitOr<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn bitor(self, rhs: BigUint) -> BigUint {
        self.bitor(&rhs)
    }
}

impl BitXor<&BigUint> for &BigUint {
    type Output = BigUint;
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let a = BigUint::from_u64(0b1100);
    /// let b = BigUint::from_u64(0b1010);
    /// assert_eq!(&a ^ &b, BigUint::from_u64(0b0110));
    /// ```
    #[inline]
    fn bitxor(self, rhs: &BigUint) -> BigUint {
        BigUint {
            limbs: super::simd_ops::xor_limbs(&self.limbs, &rhs.limbs),
        }
    }
}

impl BitXor<BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn bitxor(self, rhs: BigUint) -> BigUint {
        (&self).bitxor(&rhs)
    }
}

impl BitXor<&BigUint> for BigUint {
    type Output = BigUint;
    #[inline]
    fn bitxor(self, rhs: &BigUint) -> BigUint {
        (&self).bitxor(rhs)
    }
}

impl BitXor<BigUint> for &BigUint {
    type Output = BigUint;
    #[inline]
    fn bitxor(self, rhs: BigUint) -> BigUint {
        self.bitxor(&rhs)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_canonical() {
        let z = BigUint::zero();
        assert!(z.is_zero());
        assert_eq!(z.limbs.len(), 0);
        assert_eq!(z.bit_length(), 0);
    }

    #[test]
    fn normalize_strips_trailing_zeros() {
        let n = BigUint::from_le_limbs(&[5, 0, 0]);
        assert_eq!(n.as_limbs(), &[5u64]);
    }

    #[test]
    fn from_u128_high_limb() {
        let n = BigUint::from_u128(u128::MAX);
        assert_eq!(n.as_limbs(), &[u64::MAX, u64::MAX]);
        assert_eq!(n.bit_length(), 128);
    }

    #[test]
    fn add_with_carry_chain() {
        // 0xFF...F + 1 should produce a new limb.
        let a = BigUint::from_u64(u64::MAX);
        let b = BigUint::from_u64(1);
        let s = &a + &b;
        assert_eq!(s.as_limbs(), &[0u64, 1u64]);
    }

    #[test]
    fn checked_sub_underflow_returns_none() {
        let a = BigUint::from_u64(5);
        let b = BigUint::from_u64(10);
        assert!(a.checked_sub(&b).is_none());
    }

    #[test]
    fn checked_sub_basic() {
        let a = BigUint::from_u64(100);
        let b = BigUint::from_u64(40);
        assert_eq!(a.checked_sub(&b), Some(BigUint::from_u64(60)));
    }

    #[test]
    fn checked_sub_borrow_chain() {
        let a = BigUint::from_le_limbs(&[0, 1]); // 2^64
        let b = BigUint::from_u64(1);
        let d = a.checked_sub(&b).expect("non-underflow");
        assert_eq!(d.as_limbs(), &[u64::MAX]);
    }

    #[test]
    fn shl_within_limb() {
        let n = BigUint::from_u64(1);
        assert_eq!(n.shl_bits(5), BigUint::from_u64(32));
    }

    #[test]
    fn shl_crosses_limb_boundary() {
        let n = BigUint::from_u64(1);
        let s = n.shl_bits(64);
        assert_eq!(s.as_limbs(), &[0u64, 1u64]);
    }

    #[test]
    fn shl_by_zero_is_identity() {
        let n = BigUint::from_u64(42);
        assert_eq!(n.shl_bits(0), n);
    }

    #[test]
    fn shr_within_limb() {
        let n = BigUint::from_u64(0b1010_0000);
        assert_eq!(n.shr_bits(4), BigUint::from_u64(0b1010));
    }

    #[test]
    fn shr_crosses_limb_boundary() {
        let n = BigUint::from_le_limbs(&[0, 1]); // 2^64
        let s = n.shr_bits(64);
        assert_eq!(s, BigUint::from_u64(1));
    }

    #[test]
    fn shl_then_shr_is_identity() {
        let n = BigUint::from_u64(0xDEAD_BEEF_CAFE_BABE);
        for k in [0u64, 1, 32, 63, 64, 65, 100, 256] {
            let r = n.shl_bits(k).shr_bits(k);
            assert_eq!(r, n, "shl/shr identity failed for k={k}");
        }
    }

    #[test]
    fn cmp_by_length() {
        let small = BigUint::from_u64(u64::MAX);
        let big = BigUint::from_le_limbs(&[0, 1]); // 2^64 > 2^64 - 1
        assert!(big > small);
    }

    #[test]
    fn cmp_same_length_msb_first() {
        let a = BigUint::from_le_limbs(&[1, 2]);
        let b = BigUint::from_le_limbs(&[100, 1]);
        // [1, 2] has top limb 2 > 1 = top of [100, 1].
        assert!(a > b);
    }

    #[test]
    fn bit_length_basic() {
        assert_eq!(BigUint::zero().bit_length(), 0);
        assert_eq!(BigUint::from_u64(1).bit_length(), 1);
        assert_eq!(BigUint::from_u64(0xFF).bit_length(), 8);
        assert_eq!(BigUint::from_u64(u64::MAX).bit_length(), 64);
    }

    #[test]
    fn trailing_zeros_basic() {
        assert_eq!(BigUint::from_u64(0b1000).trailing_zeros(), 3);
        assert_eq!(BigUint::from_le_limbs(&[0, 1]).trailing_zeros(), 64);
        assert_eq!(BigUint::zero().trailing_zeros(), 0);
    }

    #[test]
    fn count_ones_basic() {
        assert_eq!(BigUint::zero().count_ones(), 0);
        assert_eq!(BigUint::from_u64(0b1011).count_ones(), 3);
        // Multi-limb: top limb has u64 = 0xF (4 bits), low limb has 1 bit set.
        let n = BigUint::from_le_limbs(&[1, 0xF]);
        assert_eq!(n.count_ones(), 5);
    }

    #[test]
    fn test_set_clear_bit() {
        let mut n = BigUint::zero();
        n.set_bit(100);
        assert!(n.test_bit(100));
        assert_eq!(n.bit_length(), 101);
        n.clear_bit(100);
        assert!(n.is_zero());
    }

    #[test]
    fn bytes_roundtrip_le() {
        let n = BigUint::from_u64(0xDEAD_BEEF_CAFE_BABE);
        let bytes = n.to_bytes_le();
        let m = BigUint::from_bytes_le(&bytes);
        assert_eq!(m, n);
    }

    #[test]
    fn bytes_roundtrip_be() {
        let n = BigUint::from_u64(0xDEAD_BEEF_CAFE_BABE);
        let bytes = n.to_bytes_be();
        let m = BigUint::from_bytes_be(&bytes);
        assert_eq!(m, n);
    }

    #[test]
    fn bitand_or_xor() {
        let a = BigUint::from_u64(0b1100);
        let b = BigUint::from_u64(0b1010);
        assert_eq!(&a & &b, BigUint::from_u64(0b1000));
        assert_eq!(&a | &b, BigUint::from_u64(0b1110));
        assert_eq!(&a ^ &b, BigUint::from_u64(0b0110));
    }

    // -----------------------------------------------------------------------
    // Cache-friendly API tests (normalize/compact/from_le_limbs_with_capacity)
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_bulk_truncate_correctness() {
        // Verify the rposition-based truncate matches old pop-based semantics.
        let mut v: Vec<u64> = vec![1, 2, 3, 0, 0, 0];
        normalize(&mut v);
        assert_eq!(v, vec![1u64, 2, 3]);

        let mut z: Vec<u64> = vec![0, 0, 0];
        normalize(&mut z);
        assert!(z.is_empty());

        let mut s: Vec<u64> = vec![42];
        normalize(&mut s);
        assert_eq!(s, vec![42u64]);
    }

    #[test]
    fn add_two_pass_carries_correctly() {
        // Multi-limb add that produces a carry into a new limb.
        let a = BigUint::from_le_limbs(&[u64::MAX, u64::MAX]);
        let b = BigUint::from_u64(1);
        let sum = &a + &b;
        assert_eq!(sum.as_limbs(), &[0u64, 0, 1]);
    }

    #[test]
    fn add_fast_path_tail_copy() {
        // Exercises the early-exit branch when carry becomes zero mid-tail.
        let a = BigUint::from_le_limbs(&[1, 2, 3, 4, 5]);
        let b = BigUint::from_le_limbs(&[u64::MAX - 1, 0]);
        // a + b: limb0 = 1+(2^64-2)=2^64-1 (no carry), then remaining limbs
        // of a are copied verbatim.
        let expected = BigUint::from_le_limbs(&[u64::MAX, 2, 3, 4, 5]);
        assert_eq!(&a + &b, expected);
    }

    #[test]
    fn checked_sub_two_pass_borrows_correctly() {
        // 2^128 - 1 (full multi-limb borrow propagation).
        let a = BigUint::from_le_limbs(&[0, 0, 1]); // 2^128
        let b = BigUint::from_u64(1);
        let diff = a.checked_sub(&b).expect("no underflow");
        assert_eq!(diff.as_limbs(), &[u64::MAX, u64::MAX]);
    }

    #[test]
    fn checked_sub_fast_path_tail_copy() {
        // Exercises the early-exit when borrow clears mid-tail.
        let a = BigUint::from_le_limbs(&[5, 100, 100, 100]);
        let b = BigUint::from_le_limbs(&[3, 100]);
        // limb0=5-3=2 (no borrow), limb1=100-100=0 (no borrow) → tail copied.
        let expected = BigUint::from_le_limbs(&[2, 0, 100, 100]);
        assert_eq!(a.checked_sub(&b).expect("no underflow"), expected);
    }

    #[test]
    fn from_le_limbs_with_capacity_value_correct() {
        let n = BigUint::from_le_limbs_with_capacity(&[42, 0, 0], 4);
        assert_eq!(n, BigUint::from_u64(42));
    }

    #[test]
    fn from_le_limbs_with_capacity_zero_input() {
        let n = BigUint::from_le_limbs_with_capacity(&[0, 0], 8);
        assert!(n.is_zero());
    }

    #[test]
    fn from_le_limbs_with_capacity_multidigit() {
        let n = BigUint::from_le_limbs_with_capacity(&[1, 2, 3], 2);
        assert_eq!(n.as_limbs(), &[1u64, 2, 3]);
    }

    #[test]
    fn compact_preserves_value() {
        let mut n = BigUint::from_le_limbs_with_capacity(&[0xDEAD, 0xBEEF], 100);
        let before = n.clone();
        n.compact();
        assert_eq!(n, before, "compact() must not change value");
    }

    #[test]
    fn shl_bits_pre_alloc_no_realloc() {
        // Verify correctness of the pre-allocated shl_bits path (functional check).
        let n = BigUint::from_u64(1);
        let s = n.shl_bits(127);
        // 2^127 should have bit 127 set.
        assert!(s.test_bit(127));
        assert_eq!(s.bit_length(), 128);
    }
}
