//! Native binary-base arbitrary-precision complex numbers built from the
//! ground up on [`oxinum_float::native::BigFloat`].
//!
//! Provides `BigComplex`, the ordered pair `(re, im)` of binary
//! `BigFloat`s representing `re + im·i`. Unlike the decimal-backed
//! crate-root [`crate::CBig`], this type works in base 2 with explicit
//! rounding-mode control, exactly mirroring the `oxinum_float::native`
//! foundation it sits on. Everything is Pure Rust — no GMP, no MPFR.
//!
//! The native `RoundingMode` is re-exported here for convenience so
//! callers constructing `BigComplex` values do not need to reach into
//! `oxinum_float::native` directly.

pub use oxinum_float::native::RoundingMode;

mod complex;
mod complex_ops;
mod convert;
mod core_traits;
mod inverse_trig;
mod transcendental;
mod trig;

#[cfg(feature = "num-traits")]
mod num_traits_impl;
#[cfg(feature = "serde")]
mod serde_impl;

pub use complex::BigComplex;
