#![forbid(unsafe_code)]
//! # OxiNum
//!
//! OxiNum is the COOLJAPAN Pure-Rust arbitrary-precision math layer
//! (GMP/MPFR-free). It provides arbitrary-precision integers, floats, and
//! rationals, plus number-theory and elementary functions, built on the
//! Pure Rust `dashu` backend.
//!
//! ## Dual exposure: dashu-backed (default) and pure-native (`native::`)
//!
//! Two coexistent type families are intentionally exposed:
//!
//! * The crate-root re-exports — `Int`, `Natural`, `Float`, `Rational`,
//!   `BigInt`, `BigUint`, `DBig`, `RBig`, … — are the dashu-backed default
//!   and remain the recommended entry point for application code today.
//! * [`native`] re-exports the ground-up Pure Rust types
//!   ([`native::BigUint`], [`native::BigInt`], [`native::BigFloat`],
//!   [`native::BigRational`]) implemented in `oxinum-int`, `oxinum-float`,
//!   and `oxinum-rational`. Reach for these when you want zero `dashu`
//!   dependence, explicit limb / rounding-mode control, or to migrate
//!   incrementally toward the eventual native default. Use the parallel
//!   aliases [`native::Int`], [`native::Natural`], [`native::Float`], and
//!   [`native::Rational`] to mirror the top-level shape.
//!
//! No type at the crate root is shadowed — the namespaces are disjoint, so
//! existing code keeps compiling unchanged.
//!
//! ## Quick start
//!
//! ```
//! use oxinum::prelude::*;
//!
//! // Arbitrary-precision integer arithmetic
//! let big = factorial(20);
//! assert_eq!(big.to_string(), "2432902008176640000");
//!
//! // High-precision pi
//! let pi = constants::pi(50);
//! assert!(pi.to_string().starts_with("3.14159265358979"));
//! ```

pub use oxinum_core::{OxiNumError, OxiNumResult, ParseNumberError, RoundingMode, Sign};

pub use oxinum_int::{
    binomial, extended_gcd, factorial, fibonacci, is_prime, lucas, mod_pow, next_prime, BigInt,
    BigUint, Gcd, IBig, UBig,
};

pub use oxinum_float::{
    atan, atan2, compute_e, compute_ln2, compute_pi, cos, cosh, exp, ln, pow, sin, sinh, sqrt, tan,
    tanh, BigFloat, Context, DBig, FBig,
};

pub use oxinum_rational::{
    best_rational_approximation, continued_fraction, from_continued_fraction, mediant,
    mixed_number, rational_abs, rational_ceil, rational_floor, rational_pow, rational_reciprocal,
    rational_round, rational_signum, rational_truncate, to_decimal_string, BigRational, RBig,
    Relaxed,
};

/// Rounding modes for arbitrary-precision floating-point operations.
///
/// Re-exported from `oxinum-float` for convenience; mirrors `dashu_float`'s
/// rounding mode module so callers need only depend on `oxinum`.
pub mod round {
    pub use oxinum_float::round::*;
}

// ---------------------------------------------------------------------------
// Top-level type aliases
// ---------------------------------------------------------------------------

/// Arbitrary-precision signed integer.
pub type Int = oxinum_int::IBig;

/// Arbitrary-precision unsigned integer.
pub type Natural = oxinum_int::UBig;

/// Arbitrary-precision decimal floating-point number.
pub type Float = oxinum_float::DBig;

/// Arbitrary-precision exact rational number.
pub type Rational = oxinum_rational::RBig;

// ---------------------------------------------------------------------------
// Pure-native (no-dashu) re-exports
// ---------------------------------------------------------------------------

/// Pure-native arbitrary-precision types and helpers (no `dashu` backend).
///
/// These are the ground-up Pure Rust implementations from `oxinum-int`,
/// `oxinum-float`, and `oxinum-rational`. They coexist with — and do not
/// replace — the dashu-backed types re-exported at the crate root, so
/// callers can migrate to the native stack incrementally without breaking
/// existing imports.
///
/// # Examples
///
/// ```
/// use oxinum::native::{BigInt, BigUint, BigFloat, RoundingMode};
///
/// let n: BigUint = BigUint::from(10_u64);
/// let m: BigInt = BigInt::from(-3_i64);
/// let f = BigFloat::from_i64(7, 32, RoundingMode::HalfEven);
/// assert_eq!(n.to_string(), "10");
/// assert_eq!(m.to_string(), "-3");
/// assert!(!f.is_zero());
/// ```
pub mod native {
    pub use oxinum_float::native::{BigFloat, RoundingMode};
    pub use oxinum_int::native::{
        checked_divrem, divrem, divrem_int, factorial, gcd, gcd_binary, gcd_extended, gcd_int,
        is_probably_prime, mod_inv, mod_mul, mod_pow, prime_sieve, BigInt, BigUint,
        MontgomeryContext, KARATSUBA_THRESHOLD, NEWTON_DIV_THRESHOLD,
    };
    pub use oxinum_rational::native::BigRational;

    /// Native counterpart of [`oxinum::Int`](crate::Int) (the dashu-backed
    /// default). Alias for [`BigInt`].
    pub type Int = BigInt;

    /// Native counterpart of [`oxinum::Natural`](crate::Natural). Alias for
    /// [`BigUint`].
    pub type Natural = BigUint;

    /// Native counterpart of [`oxinum::Float`](crate::Float). Alias for
    /// [`BigFloat`].
    pub type Float = BigFloat;

    /// Native counterpart of [`oxinum::Rational`](crate::Rational). Alias
    /// for [`BigRational`].
    pub type Rational = BigRational;
}

// ---------------------------------------------------------------------------
// Submodules
// ---------------------------------------------------------------------------

pub mod constants;

pub mod convert;

pub mod prelude;

mod parse;

pub use parse::{parse, ParsedNumber};

/// Returns the version of the `oxinum` crate.
///
/// # Examples
///
/// ```
/// assert!(!oxinum::version().is_empty());
/// ```
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn version_is_nonempty() {
        assert!(!super::version().is_empty());
    }

    #[test]
    fn facade_ubig() {
        let n = UBig::from(100u32);
        assert_eq!(n.to_string(), "100");
    }

    #[test]
    fn facade_ibig_negative() {
        let n = IBig::from(-1i32);
        assert_eq!(n.to_string(), "-1");
    }

    #[test]
    fn facade_dbig() {
        let n: DBig = DBig::from(1u32);
        assert_eq!(n.to_string(), "1");
    }

    #[test]
    fn facade_rbig() {
        let n = RBig::from(7u32);
        assert_eq!(n.to_string(), "7");
    }

    #[test]
    fn facade_context_round() {
        let ctx = Context::<round::HalfAway>::new(50usize);
        assert_eq!(
            format!("{ctx:?}"),
            "Context { precision: 50, rounding: HalfAway }"
        );
    }

    #[test]
    fn facade_relaxed() {
        let r: Relaxed = Relaxed::from(1u32);
        assert_eq!(r.to_string(), "1");
    }

    #[test]
    fn facade_dbig_from_str() {
        let d = DBig::from_str("2.718281828").expect("valid decimal");
        assert!(d.to_string().starts_with("2.718"));
    }

    #[test]
    fn facade_type_aliases() {
        let i: Int = Int::from(42);
        let n: Natural = Natural::from(42u32);
        let r: Rational = Rational::from(42u32);
        assert_eq!(i.to_string(), "42");
        assert_eq!(n.to_string(), "42");
        assert_eq!(r.to_string(), "42");
    }

    #[test]
    fn facade_number_theory() {
        assert_eq!(factorial(5), UBig::from(120u32));
        assert_eq!(fibonacci(10), UBig::from(55u32));
        assert!(is_prime(&UBig::from(17u32), 0));
    }
}
