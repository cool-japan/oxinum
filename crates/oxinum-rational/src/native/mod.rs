//! Native arbitrary-precision rational number type built on top of the
//! `oxinum_int::native` `BigInt` / `BigUint` foundation.
//!
//! Provides [`BigRational`], a value that is **always** kept in lowest terms
//! with a strictly positive denominator. The canonical zero is the unique
//! `{ num: 0, den: 1 }` representation, so `Eq`, `Hash`, and `Ord` are uniform.
//!
//! This module coexists with — and does NOT replace — the
//! [`dashu_ratio::RBig`] re-exports at the crate root.
//!
//! # Invariants
//!
//! Every public constructor and every arithmetic operation maintains:
//!
//! 1. `den > 0` (a zero denominator is reported as
//!    [`OxiNumError::DivByZero`](oxinum_core::OxiNumError::DivByZero)).
//! 2. `gcd(|num|, den) == 1`.
//! 3. The canonical zero is exclusively `{ num: BigInt::ZERO, den: 1 }`.
//!
//! # Examples
//!
//! ```
//! use oxinum_rational::native::BigRational;
//! use oxinum_int::native::{BigInt, BigUint};
//!
//! // 6/4 auto-reduces to 3/2.
//! let r = BigRational::from_parts(BigInt::from(6i64), BigUint::from_u64(4))
//!     .expect("non-zero denominator");
//! assert_eq!(r.to_string(), "3/2");
//!
//! // Sign always lives on the numerator: -9/12 reduces to -3/4.
//! let r = BigRational::from_parts(BigInt::from(-9i64), BigUint::from_u64(12))
//!     .expect("non-zero denominator");
//! assert_eq!(r.to_string(), "-3/4");
//! ```

pub mod continued_fraction;
mod convert;
mod core_traits;
mod rational;
mod rational_ops;

#[cfg(feature = "num-traits")]
mod num_traits_impl;

#[cfg(feature = "serde")]
mod serde_impl;

pub use convert::{float_to_rational, rational_to_float};
pub use rational::BigRational;

#[cfg(feature = "num-traits")]
pub use num_traits_impl::ParseBigRationalError;
