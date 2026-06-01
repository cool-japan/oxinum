//! Common types and functions for glob import.
//!
//! ```
//! use oxinum::prelude::*;
//!
//! let n = factorial(10);
//! assert_eq!(n, UBig::from(3_628_800u32));
//! ```

// Core error types and traits
pub use oxinum_core::{OxiNumError, OxiNumResult, ParseNumberError, RoundingMode, Sign};

// Integer types and number theory
pub use oxinum_int::{
    binomial, extended_gcd, factorial, fibonacci, is_prime, lucas, mod_pow, next_prime, BigInt,
    BigUint, Gcd, IBig, UBig,
};

// Float types and elementary functions
pub use oxinum_float::{
    atan, atan2, cos, cosh, exp, ln, pow, sin, sinh, sqrt, tan, tanh, BigFloat, Context, DBig, FBig,
};

// Rational types and operations
pub use oxinum_rational::{
    best_rational_approximation, continued_fraction, from_continued_fraction, mediant,
    rational_abs, rational_reciprocal, to_decimal_string, BigRational, RBig, Relaxed,
};

// Top-level type aliases
pub use crate::{Float, Int, Natural, Rational};

// Constants and conversions as modules
pub use crate::constants;
pub use crate::convert;

// Universal parser
pub use crate::{parse, ParsedNumber};
