//! Native arbitrary-precision floating-point implementation.
//!
//! This module provides [`BigFloat`], a binary-base (`b = 2`) arbitrary
//! precision floating-point number with explicit precision tracking and
//! post-operation rounding. It is implemented in pure safe Rust on top of
//! the native [`oxinum_int::native::BigUint`] limb-vector integer.
//!
//! The wrapper-level [`crate::BigFloat`] alias (decimal-base `DBig` via
//! `dashu-float`) is a separate type — they coexist intentionally. Reach
//! for the native `BigFloat` when you need ground-up Pure Rust binary
//! arithmetic with explicit rounding-mode control.
//!
//! # Phase 2 scope (this module)
//!
//! - Construction / decomposition: `zero`, `nan`, `infinity`, `neg_infinity`,
//!   `from_i64`, `from_f64`, `to_f64`, `from_parts`, `with_precision`,
//!   `round_to_precision`.
//! - Accessors: `precision`, `sign`, `mantissa`, `exponent`, `is_zero`,
//!   `is_finite`, `is_infinite`, `is_nan`, `is_normal`, `classify`,
//!   `is_sign_positive`, `is_sign_negative`, `signum`, `abs`, `neg`.
//! - Classification: `FloatClass` enum (`Finite`, `Infinite`, `Nan`).
//! - Arithmetic: `Add`, `Sub`, `Neg`, plus the `*Assign` variants.
//! - Comparison: `PartialOrd`, `PartialEq` (precision-independent, NaN-aware).
//!   **Note:** `Ord` and `Eq` are *not* implemented — NaN breaks reflexivity
//!   and totality. Use `BigFloat::total_cmp` for a sort-stable total order.
//! - Display: hex-float-like `0xb<binary>p<exponent>`, `NaN`, `inf`, `-inf`.
//!
//! Multiplication, division, and square root land in `float_mul`,
//! `float_div`, and `float_sqrt`. They feed back through
//! [`BigFloat::from_parts`] so the canonical-form invariants are uniformly
//! enforced.
//!
//! # Examples
//!
//! ```
//! use oxinum_float::native::{BigFloat, RoundingMode};
//!
//! let one = BigFloat::from_i64(1, 32, RoundingMode::HalfEven);
//! let two = BigFloat::from_i64(2, 32, RoundingMode::HalfEven);
//! let sum = &one + &two;
//! assert_eq!(sum.to_f64(), 3.0);
//!
//! let three = BigFloat::from_i64(3, 32, RoundingMode::HalfEven);
//! let six = &two * &three;
//! assert_eq!(six.to_f64(), 6.0);
//! ```

mod atan;
pub mod binary_splitting;
mod bs_transcendental;
mod constants;
mod context;
mod core_traits;
mod float;
mod float_add;
mod float_convert;
mod float_div;
mod float_exp;
mod float_ln;
mod float_mul;
mod float_sqrt;
mod format_ext;
mod ln_agm;
pub mod nonfinite;
mod pow;
mod trig;

#[cfg(feature = "num-traits")]
mod num_traits_impl;

#[cfg(feature = "serde")]
mod serde_impl;

pub use constants::{e_const, ln2, pi};
pub use context::FloatContext;
pub use float::{BigFloat, FloatClass, RoundingMode};

#[cfg(feature = "num-traits")]
pub use num_traits_impl::ParseBigFloatError;
