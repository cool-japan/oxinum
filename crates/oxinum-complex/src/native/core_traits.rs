//! Core trait implementations for native [`BigComplex`]: [`Display`] and
//! [`Debug`].
//!
//! [`PartialEq`] lives next to the arithmetic operators in
//! [`super::complex_ops`]; `Eq`, `Ord`, and `Hash` are intentionally absent
//! (see the [`super`] module docs — `BigFloat` is neither `Eq` nor `Hash`
//! because of NaN, and the complex field has no ring-compatible order).
//!
//! # `Display`
//!
//! Renders as `<re> + <|im|>i` or `<re> - <|im|>i`, choosing the sign from
//! [`BigFloat::is_sign_negative`] on the imaginary part and printing each
//! component through `BigFloat`'s own `Display` (the exact `0xb…p…`
//! hex-float form). The imaginary magnitude uses `im.abs()` so the chosen
//! `+`/`-` separator is never duplicated by the component's own sign.

use core::fmt;

use super::BigComplex;

impl fmt::Display for BigComplex {
    /// Format as `a + bi` (or `a - bi` when the imaginary part is negative).
    ///
    /// Each component is delegated to [`BigFloat`](oxinum_float::native::BigFloat)'s
    /// `Display`. The real part keeps its own sign; the imaginary part is shown
    /// as a magnitude with an explicit `+`/`-` separator chosen from its sign.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_complex::native::BigComplex;
    /// let z = BigComplex::from_f64(1.0, -2.0, 53).expect("finite");
    /// let s = format!("{z}");
    /// assert!(s.contains('-'));
    /// assert!(s.ends_with('i'));
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.im.is_sign_negative() {
            // a - |b|i
            write!(f, "{} - {}i", self.re, self.im.abs())
        } else {
            // a + bi (covers non-negative im, including the canonical zero)
            write!(f, "{} + {}i", self.re, self.im)
        }
    }
}

impl fmt::Debug for BigComplex {
    /// Structural debug view exposing the real and imaginary [`BigFloat`](oxinum_float::native::BigFloat)s.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BigComplex")
            .field("re", &self.re)
            .field("im", &self.im)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_positive_imag() {
        let z = BigComplex::from_f64(1.0, 2.0, 53).expect("finite");
        let s = format!("{z}");
        assert!(s.contains(" + "), "expected ' + ' in {s:?}");
        assert!(s.ends_with('i'), "expected trailing 'i' in {s:?}");
    }

    #[test]
    fn display_negative_imag() {
        let z = BigComplex::from_f64(1.0, -2.0, 53).expect("finite");
        let s = format!("{z}");
        assert!(s.contains(" - "), "expected ' - ' in {s:?}");
        // The imaginary magnitude must not carry its own leading '-'.
        assert!(!s.contains("- -"), "double sign in {s:?}");
        assert!(s.ends_with('i'), "expected trailing 'i' in {s:?}");
    }

    #[test]
    fn debug_mentions_fields() {
        let z = BigComplex::from_f64(1.0, 2.0, 53).expect("finite");
        let s = format!("{z:?}");
        assert!(s.contains("BigComplex"), "{s:?}");
        assert!(s.contains("re"), "{s:?}");
        assert!(s.contains("im"), "{s:?}");
    }
}
