//! Two's-complement signed byte serialization for [`BigInt`].
//!
//! This module adds inherent methods on [`BigInt`] for converting to and from
//! big-endian / little-endian two's-complement byte sequences. The encoding
//! uses **minimal length**: the byte string is as short as possible while
//! still unambiguously representing the signed value under two's-complement
//! sign-extension semantics.
//!
//! # Encoding rules (big-endian)
//!
//! - **Zero** → `[0x00]` (single zero byte, NOT empty — distinguishes zero
//!   from a zero-length / sentinel value).
//! - **Positive** `n`: take the BE bytes of `|n|`; if the top byte's
//!   most-significant bit is `1`, prepend a `0x00` byte (else the encoding
//!   would sign-extend to a negative value).
//! - **Negative** `n`: compute the BE bytes of `|n| - 1`, bitwise-NOT each
//!   byte, then prepend `0xFF` if the top byte's MSB is `0` (else the
//!   encoding would sign-extend to a positive value).
//!
//! # Decoding rules (big-endian)
//!
//! - **Empty input** → `BigInt::zero()`.
//! - **Non-empty**: inspect the top bit of the first byte. If `0`, the value
//!   is non-negative; build a [`BigUint`] from the bytes directly. If `1`,
//!   the value is negative: bitwise-NOT all bytes, add `1`, and use the
//!   result as the magnitude with `Sign::Negative`.
//!
//! Little-endian variants apply the same logic with bytes reversed.
//!
//! # Examples
//!
//! ```
//! use oxinum_int::native::BigInt;
//!
//! // Round-trip identity.
//! let n = BigInt::from(-129i64);
//! let bytes = n.to_signed_bytes_be();
//! assert_eq!(bytes, vec![0xFFu8, 0x7F]);
//! assert_eq!(BigInt::from_signed_bytes_be(&bytes), n);
//!
//! // -128 fits in one byte (0x80 is exactly -128 in two's complement).
//! let neg128 = BigInt::from(-128i64);
//! assert_eq!(neg128.to_signed_bytes_be(), vec![0x80u8]);
//! ```

use super::int::BigInt;
use super::uint::BigUint;
use oxinum_core::Sign;

impl BigInt {
    /// Returns the two's-complement big-endian byte representation of this
    /// value, using the minimal number of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    ///
    /// assert_eq!(BigInt::from(0i64).to_signed_bytes_be(), vec![0u8]);
    /// assert_eq!(BigInt::from(1i64).to_signed_bytes_be(), vec![1u8]);
    /// assert_eq!(BigInt::from(-1i64).to_signed_bytes_be(), vec![0xFFu8]);
    /// assert_eq!(BigInt::from(127i64).to_signed_bytes_be(), vec![0x7Fu8]);
    /// assert_eq!(BigInt::from(-128i64).to_signed_bytes_be(), vec![0x80u8]);
    /// assert_eq!(BigInt::from(128i64).to_signed_bytes_be(), vec![0x00u8, 0x80]);
    /// assert_eq!(BigInt::from(129i64).to_signed_bytes_be(), vec![0x00u8, 0x81]);
    /// assert_eq!(BigInt::from(-129i64).to_signed_bytes_be(), vec![0xFFu8, 0x7F]);
    /// ```
    pub fn to_signed_bytes_be(&self) -> Vec<u8> {
        let mut bytes = self.to_signed_bytes_le();
        bytes.reverse();
        bytes
    }

    /// Returns the two's-complement little-endian byte representation of this
    /// value, using the minimal number of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    ///
    /// assert_eq!(BigInt::from(0i64).to_signed_bytes_le(), vec![0u8]);
    /// assert_eq!(BigInt::from(1i64).to_signed_bytes_le(), vec![1u8]);
    /// assert_eq!(BigInt::from(-1i64).to_signed_bytes_le(), vec![0xFFu8]);
    /// assert_eq!(BigInt::from(128i64).to_signed_bytes_le(), vec![0x80u8, 0x00]);
    /// assert_eq!(BigInt::from(-129i64).to_signed_bytes_le(), vec![0x7Fu8, 0xFF]);
    /// ```
    pub fn to_signed_bytes_le(&self) -> Vec<u8> {
        if self.is_zero() {
            // Zero is encoded as a single zero byte (NOT empty) to preserve
            // round-trip with `from_signed_bytes_le`.
            return vec![0u8];
        }
        if self.sign() == Sign::Positive {
            // Positive: take the unsigned LE bytes, then pad with one 0x00
            // byte at the high end if the top byte's MSB is 1 (otherwise the
            // encoding would sign-extend to a negative value).
            let mut bytes = self.magnitude().to_bytes_le();
            // `to_bytes_le` strips trailing-zero bytes, which equals the top
            // bytes in LE order — so the last byte is the most-significant.
            // Defensive: magnitude is non-zero here, so bytes is non-empty.
            let last_idx = bytes.len().saturating_sub(1);
            if bytes[last_idx] & 0x80 != 0 {
                bytes.push(0x00);
            }
            bytes
        } else {
            // Negative: bytes of |n| - 1 with each byte bitwise-NOT'd.
            // |n| - 1 is non-negative because |n| >= 1 (we're in the
            // negative-strict branch).
            let mag_minus_one = self
                .magnitude()
                .checked_sub(&BigUint::one())
                .unwrap_or_else(BigUint::zero);
            let mut bytes = mag_minus_one.to_bytes_le();
            // `to_bytes_le()` strips trailing zeros in LE-order (i.e. the
            // high bytes). For two's-complement encoding we need to invert
            // these implicit-zero high bytes into `0xFF`, but since the
            // representation is "minimal length", we only emit as many
            // bytes as needed — high zeros become high 0xFFs by sign
            // extension at decode time. So we just NOT what is present.
            for b in bytes.iter_mut() {
                *b = !*b;
            }
            // Now ensure the top byte (last in LE) has its MSB set so the
            // encoding decodes as negative. If not, push a 0xFF byte.
            // After NOT'ing, if |n|-1's top byte had MSB=1, NOT'd MSB=0 → push
            // 0xFF. If |n|-1's top byte had MSB=0 (or bytes is empty because
            // |n|-1 == 0, i.e. n == -1), we need the final encoding to start
            // with 0xFF.
            let need_pad = match bytes.last() {
                None => true,                    // |n| - 1 == 0 → encode as [0xFF]
                Some(&top) => (top & 0x80) == 0, // top MSB clear → sign-extend issue
            };
            if need_pad {
                bytes.push(0xFFu8);
            }
            bytes
        }
    }

    /// Construct a `BigInt` from a two's-complement big-endian byte slice.
    ///
    /// An empty slice decodes as zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    ///
    /// assert_eq!(BigInt::from_signed_bytes_be(&[]), BigInt::zero());
    /// assert_eq!(BigInt::from_signed_bytes_be(&[0x00]), BigInt::zero());
    /// assert_eq!(BigInt::from_signed_bytes_be(&[0x01]), BigInt::from(1i64));
    /// assert_eq!(BigInt::from_signed_bytes_be(&[0xFF]), BigInt::from(-1i64));
    /// assert_eq!(BigInt::from_signed_bytes_be(&[0x80]), BigInt::from(-128i64));
    /// assert_eq!(BigInt::from_signed_bytes_be(&[0xFF, 0x7F]), BigInt::from(-129i64));
    /// ```
    pub fn from_signed_bytes_be(bytes: &[u8]) -> BigInt {
        if bytes.is_empty() {
            return BigInt::zero();
        }
        let top = bytes[0];
        if top & 0x80 == 0 {
            // Non-negative: build magnitude directly from the BE bytes.
            let mag = BigUint::from_bytes_be(bytes);
            BigInt::from_parts(Sign::Positive, mag)
        } else {
            // Negative: bitwise-NOT all bytes and add 1 to recover |n|.
            let mut inv: Vec<u8> = bytes.iter().map(|b| !*b).collect();
            // After NOT, build a BigUint and add 1.
            // Note: NOT on a length-N two's-complement encoding of a negative
            // value gives |n| - 1 in length-N unsigned form (possibly with
            // leading zeros).
            // Reverse to LE for our helper.
            inv.reverse();
            let inv_uint = BigUint::from_bytes_le(&inv);
            let mag = &inv_uint + &BigUint::one();
            BigInt::from_parts(Sign::Negative, mag)
        }
    }

    /// Construct a `BigInt` from a two's-complement little-endian byte slice.
    ///
    /// An empty slice decodes as zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    ///
    /// assert_eq!(BigInt::from_signed_bytes_le(&[]), BigInt::zero());
    /// assert_eq!(BigInt::from_signed_bytes_le(&[0xFF]), BigInt::from(-1i64));
    /// assert_eq!(BigInt::from_signed_bytes_le(&[0x7F, 0xFF]), BigInt::from(-129i64));
    /// ```
    pub fn from_signed_bytes_le(bytes: &[u8]) -> BigInt {
        if bytes.is_empty() {
            return BigInt::zero();
        }
        // Reverse to BE and delegate.
        let mut be: Vec<u8> = bytes.to_vec();
        be.reverse();
        Self::from_signed_bytes_be(&be)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_encodes_as_single_zero_byte() {
        assert_eq!(BigInt::zero().to_signed_bytes_be(), vec![0u8]);
        assert_eq!(BigInt::zero().to_signed_bytes_le(), vec![0u8]);
    }

    #[test]
    fn empty_decodes_as_zero() {
        assert_eq!(BigInt::from_signed_bytes_be(&[]), BigInt::zero());
        assert_eq!(BigInt::from_signed_bytes_le(&[]), BigInt::zero());
    }

    #[test]
    fn single_zero_byte_decodes_as_zero() {
        assert_eq!(BigInt::from_signed_bytes_be(&[0x00]), BigInt::zero());
        assert_eq!(BigInt::from_signed_bytes_le(&[0x00]), BigInt::zero());
    }

    #[test]
    fn small_positive_minimal_encoding() {
        assert_eq!(BigInt::from(1i64).to_signed_bytes_be(), vec![0x01u8]);
        assert_eq!(BigInt::from(127i64).to_signed_bytes_be(), vec![0x7Fu8]);
        // 128 needs a leading zero to avoid sign extension.
        assert_eq!(
            BigInt::from(128i64).to_signed_bytes_be(),
            vec![0x00u8, 0x80]
        );
        assert_eq!(
            BigInt::from(129i64).to_signed_bytes_be(),
            vec![0x00u8, 0x81]
        );
    }

    #[test]
    fn small_negative_minimal_encoding() {
        assert_eq!(BigInt::from(-1i64).to_signed_bytes_be(), vec![0xFFu8]);
        // -128 fits in a single byte: 0x80 == -128 in two's complement.
        assert_eq!(BigInt::from(-128i64).to_signed_bytes_be(), vec![0x80u8]);
        // -129 needs two bytes (0xFF, 0x7F).
        assert_eq!(
            BigInt::from(-129i64).to_signed_bytes_be(),
            vec![0xFFu8, 0x7F]
        );
    }
}
