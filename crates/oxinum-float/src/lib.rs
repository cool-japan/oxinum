#![forbid(unsafe_code)]
//! Arbitrary-precision floating-point arithmetic for the OxiNum ecosystem.
//!
//! Provides wrappers over `dashu-float`'s `FBig`/`DBig` with convenience
//! functions for elementary math: `exp`, `ln`, `sqrt`, `pow`, trigonometric
//! functions (`sin`, `cos`, `tan`, `atan`, `atan2`), hyperbolic functions
//! (`sinh`, `cosh`, `tanh`), and high-precision constants (`pi`, `e`, `ln2`).
//!
//! Precision-control helpers ([`precision::with_precision`],
//! [`precision::epsilon`], [`precision::ulp`]) wrap the underlying
//! `dashu-float` precision primitives in an ergonomic free-function API.
//!
//! ## Why `Hash` is not implemented
//!
//! `DBig` (`FBig<HalfAway, 10>`) intentionally does **not** implement
//! [`core::hash::Hash`].  In IEEE 754 (and by extension in essentially
//! every numeric library that takes the standard seriously) floating
//! point values defy the `Hash` contract: distinct representations can
//! compare equal (e.g. `1.0` and `1.00` at different precisions, or
//! signed zeros `+0` and `-0`), special values like `NaN` are not
//! equal to themselves, and the canonical form needed to make hashing
//! well-defined varies by use case (totalOrder vs. mathematical
//! equality vs. bitwise identity).  Rust's standard library follows
//! the same convention: `f32` and `f64` deliberately do not implement
//! `Hash`.  Callers who need a hash key should either use a
//! canonicalised representation (e.g. a tuple of `(significand,
//! exponent, precision)`) or wrap `DBig` in a newtype with an
//! explicit, documented hashing policy.
//!
//! ## Features
//!
//! * `serde` *(off by default)* — enables `serde::Serialize` /
//!   `Deserialize` for `DBig` via `dashu-float`'s own `serde` feature.

pub use dashu_float::{Context, DBig, FBig};
pub use oxinum_core::{OxiNumError, OxiNumResult, RoundingMode};

/// Rounding modes re-exported from `dashu-float` so callers do not need a
/// direct dependency on `dashu-float` for basic rounding-mode selection.
pub mod round {
    pub use dashu_float::round::mode::{Down, HalfAway, HalfEven, Up, Zero};
}

/// Type alias for decimal big-float.
pub type BigFloat = DBig;

// Sub-modules
mod constants;
mod elementary;
pub mod precision;
mod trig;

/// Native binary-base arbitrary-precision floating-point implementation.
///
/// This module is additive: the crate-root [`BigFloat`] alias remains
/// `dashu_float::DBig` (decimal). Reach for [`native::BigFloat`] when you
/// want ground-up Pure Rust binary float arithmetic with explicit
/// rounding-mode control. The native type is intentionally **not**
/// re-exported at the crate root to avoid clashing with the `DBig` alias.
pub mod native;

pub use constants::{compute_e, compute_ln2, compute_pi};
pub use elementary::{exp, ln, pow, sqrt};
pub use trig::{atan, atan2, cos, cosh, sin, sinh, tan, tanh};
