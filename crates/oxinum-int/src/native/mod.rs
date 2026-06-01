//! Native arbitrary-precision unsigned integer implementation.
//!
//! This module provides [`BigUint`], a little-endian `Vec<u64>`-limb
//! arbitrary-precision unsigned integer implemented entirely in safe Rust.
//! It coexists with the `dashu`-backed `BigUint` type alias at the crate
//! root — this module's `BigUint` is a fully independent, ground-up native
//! implementation intended as the foundation for the OxiNum integer core.
//!
//! # Invariants
//!
//! - Limbs are stored **little-endian**: `limbs[0]` is the least-significant
//!   limb, `limbs[limbs.len() - 1]` is the most-significant.
//! - The internal representation is **normalized**: no trailing-zero limbs.
//! - Canonical zero is the empty `Vec<u64>`.
//!
//! # Examples
//!
//! ```
//! use oxinum_int::native::BigUint;
//!
//! let a = BigUint::from_u64(1_000_000_000_000u64);
//! let b = BigUint::from_u64(7);
//! let prod = &a * &b;
//! assert_eq!(prod, BigUint::from_u64(7_000_000_000_000u64));
//! ```

mod bitwise;
mod bytes_signed;
mod convert;
mod div;
mod ext_gcd;
mod factorial;
mod gcd;
mod int;
pub(crate) mod lucas;
mod mod_arith;
mod montgomery;
mod mul;
mod ops_int;
mod ops_uint;
mod primality;
mod radix;
#[cfg(feature = "rand")]
pub mod rand_impl;
mod roots;
mod sieve;
mod uint;

#[cfg(feature = "num-traits")]
mod num_traits_impl;

mod core_traits;

pub use div::{checked_divrem, divrem, NEWTON_DIV_THRESHOLD};
pub use ext_gcd::{gcd_extended, mod_inv};
pub use factorial::factorial;
pub use gcd::{gcd, gcd_binary, gcd_int};
pub use int::BigInt;
pub use lucas::lucas_uv;
pub use mod_arith::{mod_mul, mod_pow};
pub use montgomery::MontgomeryContext;
#[cfg(feature = "num-traits")]
pub use num_traits_impl::{ParseBigIntError, ParseBigUintError};
pub use ops_int::divrem_int;
pub use primality::is_probably_prime;
#[cfg(feature = "rand")]
pub use rand_impl::BigUintBits;
pub use sieve::prime_sieve;
pub use uint::BigUint;
pub use uint::KARATSUBA_THRESHOLD;
