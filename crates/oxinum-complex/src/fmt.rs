//! Human-readable formatting for [`CBig`].
//!
//! [`Display`](fmt::Display) renders a complex number in the conventional
//! `re ± im·i` form, picking the sign so the imaginary magnitude is always
//! printed without a leading `-`. This mirrors the rendering expected by the
//! SciRS2 `ArbitraryComplex` consumer.

use crate::CBig;
use core::fmt;
use oxinum_float::DBig;

/// Render as `"<re> + <im>i"` when the imaginary part is non-negative, and
/// `"<re> - <|im|>i"` when it is negative.
///
/// # Examples
///
/// ```
/// use oxinum_complex::CBig;
/// let z = CBig::from_f64(2.0, 3.0).expect("finite parts");
/// assert_eq!(z.to_string(), "2 + 3i");
/// let w = CBig::from_f64(2.0, -3.0).expect("finite parts");
/// assert_eq!(w.to_string(), "2 - 3i");
/// ```
impl fmt::Display for CBig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.im < DBig::from(0u32) {
            // Print the magnitude after an explicit minus sign.
            let mag = -self.im.clone();
            write!(f, "{} - {}i", self.re, mag)
        } else {
            write!(f, "{} + {}i", self.re, self.im)
        }
    }
}

/// Developer-facing representation that names both components explicitly.
///
/// # Examples
///
/// ```
/// use oxinum_complex::CBig;
/// let z = CBig::from_f64(2.0, -3.0).expect("finite parts");
/// assert_eq!(format!("{z:?}"), "CBig { re: 2, im: -3 }");
/// ```
impl fmt::Debug for CBig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CBig {{ re: {}, im: {} }}", self.re, self.im)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_positive_imag() {
        let z = CBig::from_f64(2.0, 3.0).expect("finite parts");
        assert_eq!(z.to_string(), "2 + 3i");
    }

    #[test]
    fn display_negative_imag() {
        let z = CBig::from_f64(2.0, -3.0).expect("finite parts");
        assert_eq!(z.to_string(), "2 - 3i");
    }

    #[test]
    fn display_real_shows_zero_imag() {
        let z = CBig::from_real(DBig::from(5));
        assert_eq!(z.to_string(), "5 + 0i");
    }

    #[test]
    fn display_fractional_components() {
        let z = CBig::from_f64(1.5, -2.25).expect("finite parts");
        assert_eq!(z.to_string(), "1.5 - 2.25i");
    }

    #[test]
    fn debug_is_non_empty_and_readable() {
        let z = CBig::from_f64(2.0, -3.0).expect("finite parts");
        let s = format!("{z:?}");
        assert!(!s.is_empty());
        assert_eq!(s, "CBig { re: 2, im: -3 }");
    }
}
