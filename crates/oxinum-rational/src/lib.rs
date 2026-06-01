#![forbid(unsafe_code)]
//! Exact rational arithmetic for the OxiNum ecosystem.
//!
//! Provides `RBig` and `Relaxed` re-exports from `dashu-ratio`, plus
//! additional functions: continued fraction expansion, best rational
//! approximation, decimal string conversion, mediant, mixed number
//! representation, and floor/ceil/round/truncate operations.

pub use dashu_ratio::{RBig, Relaxed};
pub use oxinum_core::{OxiNumError, OxiNumResult};

// Re-export IBig/UBig for convenience (needed to construct RBig)
pub use dashu_int::{IBig, UBig};

/// Type alias for clarity.
pub type BigRational = RBig;

mod convert;
mod enumerate;
mod ops;

/// Native arbitrary-precision rational implementation built on
/// [`oxinum_int::native::BigInt`] / [`oxinum_int::native::BigUint`].
///
/// Access via `oxinum_rational::native::BigRational` — intentionally NOT
/// re-exported at the crate root to avoid clashing with the `BigRational`
/// type alias for `dashu_ratio::RBig`.
pub mod native;

pub use convert::{from_f32, from_f64, parse_mixed, to_f64, to_f64_exact, MixedNumber};
pub use enumerate::{farey_sequence, from_stern_brocot_path, stern_brocot_path};
pub use ops::{
    best_rational_approximation, continued_fraction, from_continued_fraction, mediant,
    mixed_number, rational_abs, rational_ceil, rational_floor, rational_from_integer,
    rational_is_integer, rational_pow, rational_reciprocal, rational_round, rational_signum,
    rational_to_integer, rational_truncate, to_decimal_string,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rbig_from_u32() {
        let n = RBig::from(42u32);
        assert_eq!(n.to_string(), "42");
    }

    #[test]
    fn relaxed_from_u32() {
        let n: Relaxed = Relaxed::from(42u32);
        assert_eq!(n.to_string(), "42");
    }

    #[test]
    fn rbig_from_parts_pi_approx() {
        let r = RBig::from_parts(IBig::from(355), UBig::from(113u32));
        assert_eq!(r.numerator(), &IBig::from(355));
        assert_eq!(r.denominator(), &UBig::from(113u32));
        assert_eq!(r.to_string(), "355/113");
    }

    #[test]
    fn rbig_add_fractions() {
        // 1/2 + 1/3 = 5/6
        let half = RBig::from_parts(IBig::from(1), UBig::from(2u32));
        let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        let sum = half + third;
        assert_eq!(sum.numerator(), &IBig::from(5));
        assert_eq!(sum.denominator(), &UBig::from(6u32));
    }

    #[test]
    fn rbig_sub_fractions() {
        // 3/4 - 1/4 = 1/2
        let three_quarters = RBig::from_parts(IBig::from(3), UBig::from(4u32));
        let one_quarter = RBig::from_parts(IBig::from(1), UBig::from(4u32));
        let diff = three_quarters - one_quarter;
        assert_eq!(diff.numerator(), &IBig::from(1));
        assert_eq!(diff.denominator(), &UBig::from(2u32));
    }

    #[test]
    fn rbig_mul() {
        // 2/3 * 3/4 = 1/2
        let a = RBig::from_parts(IBig::from(2), UBig::from(3u32));
        let b = RBig::from_parts(IBig::from(3), UBig::from(4u32));
        let product = a * b;
        assert_eq!(product.numerator(), &IBig::from(1));
        assert_eq!(product.denominator(), &UBig::from(2u32));
    }

    #[test]
    fn rbig_div() {
        // (1/2) / (1/3) = 3/2
        let half = RBig::from_parts(IBig::from(1), UBig::from(2u32));
        let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        let quotient = half / third;
        assert_eq!(quotient.numerator(), &IBig::from(3));
        assert_eq!(quotient.denominator(), &UBig::from(2u32));
    }

    #[test]
    fn relaxed_canonicalize() {
        let r = Relaxed::from_parts(IBig::from(-15), UBig::from(6u32));
        assert_eq!(r.numerator(), &IBig::from(-15));
        assert_eq!(r.denominator(), &UBig::from(6u32));
        let canonical = r.canonicalize();
        assert_eq!(canonical.numerator(), &IBig::from(-5));
        assert_eq!(canonical.denominator(), &UBig::from(2u32));
    }

    #[test]
    fn rbig_automatic_simplification() {
        // 6/4 should auto-reduce to 3/2
        let r = RBig::from_parts(IBig::from(6), UBig::from(4u32));
        assert_eq!(r.numerator(), &IBig::from(3));
        assert_eq!(r.denominator(), &UBig::from(2u32));
    }

    #[test]
    fn rbig_is_integer() {
        let whole = RBig::from_parts(IBig::from(10), UBig::from(5u32));
        assert_eq!(*whole.denominator(), UBig::ONE);

        let frac = RBig::from_parts(IBig::from(1), UBig::from(3u32));
        assert_ne!(*frac.denominator(), UBig::ONE);
    }

    #[test]
    fn rbig_from_integer() {
        let r = RBig::from(42u32);
        assert_eq!(r.numerator(), &IBig::from(42));
        assert_eq!(r.denominator(), &UBig::ONE);
    }

    #[test]
    fn rbig_negation() {
        let r = RBig::from_parts(IBig::from(3), UBig::from(4u32));
        let neg = -r;
        assert_eq!(neg.numerator(), &IBig::from(-3));
        assert_eq!(neg.denominator(), &UBig::from(4u32));
    }
}
