//! Radix conversion (`to_radix`, `from_str_radix`) + `Display`/`Debug`/
//! `LowerHex`/`UpperHex`/`Octal`/`Binary` for [`BigUint`] and [`BigInt`].
//!
//! For powers-of-2 radices (2, 4, 8, 16, 32) we use direct bit-walking via
//! shifts. For all other radices in 2..=36, we use repeated division by
//! `radix^k` where `k` is the largest power such that `radix^k` fits in `u64`.
//!
//! ## Alternate-form formatting
//!
//! All four traits (`LowerHex`, `UpperHex`, `Octal`, `Binary`) support the
//! `{:#}` alternate form: the prefix `0x`, `0o`, or `0b` is emitted via
//! `Formatter::pad_integral`, which also handles width, fill, and sign.

use super::int::BigInt;
use super::uint::BigUint;
use crate::OxiNumError;
use crate::OxiNumResult;
use std::fmt;

impl BigUint {
    /// Format this value as a string in the given `radix` (2..=36).
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::InvalidRadix`] if `radix < 2` or `radix > 36`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_u64(255);
    /// assert_eq!(n.to_radix(16).unwrap(), "ff");
    /// assert_eq!(n.to_radix(2).unwrap(), "11111111");
    /// ```
    pub fn to_radix(&self, radix: u32) -> OxiNumResult<String> {
        if !(2..=36).contains(&radix) {
            return Err(OxiNumError::InvalidRadix(radix));
        }
        if self.is_zero() {
            return Ok("0".to_string());
        }
        if radix.is_power_of_two() {
            return Ok(self.to_radix_pow2(radix));
        }
        Ok(self.to_radix_general(radix))
    }

    /// Parse a `BigUint` from `src` in the given `radix` (2..=36).
    ///
    /// Leading/trailing whitespace is rejected. Digits use lowercase or
    /// uppercase letters (case-insensitive) for radices > 10.
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::InvalidRadix`] if `radix < 2` or `radix > 36`,
    /// or [`OxiNumError::Parse`] on invalid digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// let n = BigUint::from_str_radix("ff", 16).unwrap();
    /// assert_eq!(n, BigUint::from_u64(255));
    /// ```
    pub fn from_str_radix(src: &str, radix: u32) -> OxiNumResult<BigUint> {
        if !(2..=36).contains(&radix) {
            return Err(OxiNumError::InvalidRadix(radix));
        }
        if src.is_empty() {
            return Err(OxiNumError::Parse("empty string".into()));
        }
        // Chunk by k = largest power so that radix^k fits in u64.
        // (For pow-2 radices we still use this approach for simplicity.)
        let (chunk_size, chunk_base) = chunk_for_radix(radix);
        let bytes = src.as_bytes();
        let total = bytes.len();
        let chunk_base_big = BigUint::from_u64(chunk_base);
        let mut idx = 0usize;
        let first_chunk_len = if total % chunk_size == 0 {
            chunk_size
        } else {
            total % chunk_size
        };
        // Process first (short) chunk:
        let first_str = std::str::from_utf8(&bytes[idx..idx + first_chunk_len])
            .map_err(|_| OxiNumError::Parse("invalid UTF-8".into()))?;
        let first_val: u64 = u64::from_str_radix(first_str, radix)
            .map_err(|e| OxiNumError::Parse(format!("{e}").into()))?;
        let mut acc = BigUint::from_u64(first_val);
        idx += first_chunk_len;
        // Compute the per-chunk multiplier `radix^chunk_size` once.
        while idx < total {
            let chunk_str = std::str::from_utf8(&bytes[idx..idx + chunk_size])
                .map_err(|_| OxiNumError::Parse("invalid UTF-8".into()))?;
            let val: u64 = u64::from_str_radix(chunk_str, radix)
                .map_err(|e| OxiNumError::Parse(format!("{e}").into()))?;
            acc = &acc * &chunk_base_big;
            acc = &acc + &BigUint::from_u64(val);
            idx += chunk_size;
        }
        Ok(acc)
    }

    // --------------------------------------------------------------------
    // Helpers
    // --------------------------------------------------------------------

    fn to_radix_pow2(&self, radix: u32) -> String {
        // bits_per_digit = log2(radix).
        let bits_per_digit = radix.trailing_zeros() as u64;
        // We need the full binary representation, then group bits from LSB.
        let total_bits = self.bit_length();
        // Number of digits in the output:
        let n_digits = total_bits.div_ceil(bits_per_digit) as usize;
        let mut digits: Vec<u8> = Vec::with_capacity(n_digits);
        let mask: u64 = (1u64 << bits_per_digit) - 1;
        for i in 0..n_digits {
            let bit_pos = (i as u64) * bits_per_digit;
            let limb_idx = (bit_pos / 64) as usize;
            let bit_in_limb = bit_pos % 64;
            let mut val: u64 = (self.limbs[limb_idx] >> bit_in_limb) & mask;
            // If the digit spans two limbs, fetch the bridging bits.
            let bits_in_first = (64u64).saturating_sub(bit_in_limb);
            if bits_in_first < bits_per_digit && limb_idx + 1 < self.limbs.len() {
                let needed = bits_per_digit - bits_in_first;
                let upper_mask = (1u64 << needed) - 1;
                let extra = self.limbs[limb_idx + 1] & upper_mask;
                val |= extra << bits_in_first;
            }
            digits.push(digit_to_char(val as u32));
        }
        // digits is LSB-first; reverse for MSB-first.
        digits.reverse();
        // Strip leading zero digits (we sized for ceil; the topmost might be 0).
        while digits.first() == Some(&b'0') && digits.len() > 1 {
            digits.remove(0);
        }
        String::from_utf8(digits).unwrap_or_else(|_| String::from("<radix-output-utf8-bug>"))
    }

    fn to_radix_general(&self, radix: u32) -> String {
        let (chunk_size, chunk_base) = chunk_for_radix(radix);
        let chunk_base_big = BigUint::from_u64(chunk_base);
        let mut acc = self.clone();
        // Accumulate raw chunk values (each < chunk_base).
        let mut chunks: Vec<u64> = Vec::new();
        while !acc.is_zero() {
            let (q, r) = super::div::divrem(&acc, &chunk_base_big);
            chunks.push(r.to_u64().unwrap_or(0));
            acc = q;
        }
        // Build the string: most-significant chunk has no leading zeros;
        // every subsequent chunk must be zero-padded to `chunk_size` digits.
        let mut out = String::with_capacity(chunks.len() * chunk_size);
        if let Some(&top) = chunks.last() {
            // Top: no padding.
            out.push_str(&format_in_radix(top, radix, 0));
        }
        for &c in chunks.iter().rev().skip(1) {
            out.push_str(&format_in_radix(c, radix, chunk_size));
        }
        out
    }
}

/// Largest `(k, radix^k)` such that `radix^k` fits in a `u64`.
fn chunk_for_radix(radix: u32) -> (usize, u64) {
    let r = radix as u64;
    let mut k: usize = 1;
    let mut acc: u64 = r;
    loop {
        match acc.checked_mul(r) {
            Some(v) => {
                acc = v;
                k += 1;
            }
            None => return (k, acc),
        }
    }
}

/// Format `value` in `radix`, padding with leading zeros to width `min_width`.
fn format_in_radix(value: u64, radix: u32, min_width: usize) -> String {
    if value == 0 {
        // Zero-padded zero of width `min_width` (could be 0 for top chunk).
        return "0".repeat(min_width.max(1));
    }
    let mut digits: Vec<u8> = Vec::new();
    let mut v = value;
    while v != 0 {
        digits.push(digit_to_char((v % radix as u64) as u32));
        v /= radix as u64;
    }
    while digits.len() < min_width {
        digits.push(b'0');
    }
    digits.reverse();
    String::from_utf8(digits).unwrap_or_else(|_| "0".to_string())
}

fn digit_to_char(d: u32) -> u8 {
    debug_assert!(d < 36);
    match d {
        0..=9 => b'0' + d as u8,
        10..=35 => b'a' + (d - 10) as u8,
        _ => b'?',
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_radix(10) {
            Ok(s) => f.write_str(&s),
            Err(_) => f.write_str("<BigUint-radix-error>"),
        }
    }
}

impl fmt::Debug for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // For debug, show decimal + limb count for diagnosability.
        write!(f, "BigUint({})", self)?;
        if !self.limbs.is_empty() {
            write!(f, " [{}lm]", self.limbs.len())?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// LowerHex / UpperHex / Octal / Binary for BigUint
// ---------------------------------------------------------------------------

impl fmt::LowerHex for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.to_radix(16).map_err(|_| fmt::Error)?;
        f.pad_integral(true, "0x", &digits)
    }
}

impl fmt::UpperHex for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.to_radix(16).map_err(|_| fmt::Error)?;
        // pad_integral with UpperHex prefix is still "0x" (std matches this).
        f.pad_integral(true, "0x", &digits.to_ascii_uppercase())
    }
}

impl fmt::Octal for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.to_radix(8).map_err(|_| fmt::Error)?;
        f.pad_integral(true, "0o", &digits)
    }
}

impl fmt::Binary for BigUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.to_radix(2).map_err(|_| fmt::Error)?;
        f.pad_integral(true, "0b", &digits)
    }
}

// ---------------------------------------------------------------------------
// LowerHex / UpperHex / Octal / Binary for BigInt
//
// BigInt emits `sign + magnitude-in-radix`. The `pad_integral` flag
// `is_nonnegative` drives the sign; the prefix is the same as for BigUint.
// ---------------------------------------------------------------------------

impl fmt::LowerHex for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.magnitude().to_radix(16).map_err(|_| fmt::Error)?;
        f.pad_integral(!self.is_negative(), "0x", &digits)
    }
}

impl fmt::UpperHex for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.magnitude().to_radix(16).map_err(|_| fmt::Error)?;
        f.pad_integral(!self.is_negative(), "0x", &digits.to_ascii_uppercase())
    }
}

impl fmt::Octal for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.magnitude().to_radix(8).map_err(|_| fmt::Error)?;
        f.pad_integral(!self.is_negative(), "0o", &digits)
    }
}

impl fmt::Binary for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.magnitude().to_radix(2).map_err(|_| fmt::Error)?;
        f.pad_integral(!self.is_negative(), "0b", &digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_radix_zero() {
        assert_eq!(BigUint::zero().to_radix(10).expect("radix"), "0");
        assert_eq!(BigUint::zero().to_radix(2).expect("radix"), "0");
        assert_eq!(BigUint::zero().to_radix(16).expect("radix"), "0");
    }

    #[test]
    fn to_radix_decimal_small() {
        let n = BigUint::from_u64(12345);
        assert_eq!(n.to_radix(10).expect("radix"), "12345");
    }

    #[test]
    fn to_radix_hex() {
        let n = BigUint::from_u64(0xDEAD_BEEF);
        assert_eq!(n.to_radix(16).expect("radix"), "deadbeef");
    }

    #[test]
    fn to_radix_binary() {
        let n = BigUint::from_u64(0b1010_1100);
        assert_eq!(n.to_radix(2).expect("radix"), "10101100");
    }

    #[test]
    fn radix_invalid() {
        assert!(BigUint::from_u64(10).to_radix(1).is_err());
        assert!(BigUint::from_u64(10).to_radix(37).is_err());
        assert!(BigUint::from_str_radix("123", 1).is_err());
    }

    #[test]
    fn radix_roundtrip_many() {
        let n = BigUint::from_le_limbs(&[0xDEAD_BEEF_CAFE_BABE, 0x1234_5678_9ABC_DEF0, 0x42]);
        for r in [2, 8, 10, 16, 36, 7, 13] {
            let s = n.to_radix(r).expect("to_radix");
            let m = BigUint::from_str_radix(&s, r).expect("from_radix");
            assert_eq!(m, n, "roundtrip failed at radix {r}");
        }
    }

    #[test]
    fn display_matches_decimal() {
        let n = BigUint::from_u64(12_345_678_901_234_567_890u64);
        assert_eq!(format!("{n}"), "12345678901234567890");
    }
}
