#![forbid(unsafe_code)]
//! Arbitrary-precision integer arithmetic for the OxiNum ecosystem.
//!
//! Provides `BigInt` and `BigUint` type aliases over `dashu-int`'s `IBig`/`UBig`,
//! plus number-theory functions: factorial, fibonacci, binomial coefficients,
//! modular exponentiation, primality testing, and extended GCD.

pub use dashu_int::{IBig, UBig};
pub use oxinum_core::OxiNumError;
pub use oxinum_core::OxiNumResult;

// Re-export GCD operation trait so callers can use `.gcd()`.
pub use dashu_int::ops::Gcd;

// Re-export core traits and types that downstream crates will need.
pub use oxinum_core::Sign;

/// Type alias: `BigUint` is `dashu_int::UBig`.
pub type BigUint = UBig;

/// Type alias: `BigInt` is `dashu_int::IBig`.
pub type BigInt = IBig;

// Sub-modules
mod number_theory;
mod traits;

/// Native arbitrary-precision integer types implemented in pure Rust.
///
/// This module provides an additive, ground-up native implementation that
/// coexists with the `dashu`-backed `BigUint`/`BigInt` aliases re-exported
/// at the crate root. See [`native::BigUint`] for the unsigned core.
pub mod native;

// Number theory functions
pub use number_theory::{
    binomial, extended_gcd, factorial, fibonacci, is_prime, lucas, mod_pow, next_prime,
};

// Radix conversion and utility functions
pub use traits::{
    ibig_abs, ibig_from_radix, ibig_is_one, ibig_is_zero, ibig_signum, ibig_to_radix,
    ubig_from_radix, ubig_is_one, ubig_is_zero, ubig_to_radix,
};
