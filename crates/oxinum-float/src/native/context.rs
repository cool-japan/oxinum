//! [`FloatContext`] — precision + rounding-mode builder for native `BigFloat`
//! transcendentals.
//!
//! Captures `(precision, rounding)` once so callers avoid repeating them at
//! every transcendental call site.
//!
//! # Design
//!
//! The native [`BigFloat`] transcendental methods all take `(prec: u32, mode:
//! RoundingMode)` as explicit parameters. `FloatContext` is a thin wrapper that
//! stores these two values and forwards to the methods unchanged — it does not
//! add guard bits or change any computation.
//!
//! # Naming
//!
//! The type is named `FloatContext` rather than `Context` to avoid a collision
//! with `dashu::Context<R>`, which is re-exported in the same crate.
//!
//! # Constant methods
//!
//! [`FloatContext::pi`], [`FloatContext::e_const`], and [`FloatContext::ln2`]
//! return `OxiNumResult<BigFloat>` because the underlying constant generators
//! (`constants::pi`, etc.) are fallible — they may propagate arithmetic errors
//! from the internal Chudnovsky / binary-splitting computations.
//!
//! # Examples
//!
//! ```
//! use oxinum_float::native::{BigFloat, FloatContext, RoundingMode};
//!
//! let ctx = FloatContext::new(200);
//! let two = BigFloat::from_i64(2, 200, RoundingMode::HalfEven);
//! let ln2 = ctx.ln(&two).expect("ln(2)");
//! let back = ctx.exp(&ln2).expect("exp(ln(2))");
//! assert!(!back.is_zero());
//! ```

use oxinum_core::OxiNumResult;

use super::constants;
use super::float::{BigFloat, RoundingMode};

/// Precision context for native `BigFloat` computations.
///
/// Stores `precision` (bits) and `rounding` together so that a single
/// `FloatContext` value can be used across multiple transcendental calls
/// without repeating the parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatContext {
    precision: u32,
    rounding: RoundingMode,
}

impl FloatContext {
    /// Create a context with the given `precision` in bits and
    /// [`RoundingMode::HalfEven`] rounding.
    ///
    /// # Panics
    ///
    /// Panics if `precision == 0` (the `BigFloat` precision invariant requires
    /// `prec > 0`).
    pub fn new(precision: u32) -> Self {
        assert!(precision > 0, "FloatContext precision must be > 0");
        Self {
            precision,
            rounding: RoundingMode::HalfEven,
        }
    }

    /// Return a new context identical to `self` but with `rounding` replaced.
    ///
    /// Uses the builder pattern so callers can chain:
    ///
    /// ```
    /// use oxinum_float::native::{FloatContext, RoundingMode};
    /// let ctx = FloatContext::new(128).with_rounding(RoundingMode::ToInf);
    /// assert_eq!(ctx.rounding(), RoundingMode::ToInf);
    /// ```
    #[must_use]
    pub fn with_rounding(self, mode: RoundingMode) -> Self {
        Self {
            rounding: mode,
            ..self
        }
    }

    /// Returns the precision (bits) stored in this context.
    pub fn precision(&self) -> u32 {
        self.precision
    }

    /// Returns the rounding mode stored in this context.
    pub fn rounding(&self) -> RoundingMode {
        self.rounding
    }

    // -----------------------------------------------------------------------
    // Transcendental forwarders
    // -----------------------------------------------------------------------

    /// Return `sqrt(x)` at the context precision.
    pub fn sqrt(&self, x: &BigFloat) -> OxiNumResult<BigFloat> {
        x.sqrt(self.precision, self.rounding)
    }

    /// Return `e^x` at the context precision.
    pub fn exp(&self, x: &BigFloat) -> OxiNumResult<BigFloat> {
        x.exp(self.precision, self.rounding)
    }

    /// Return `ln(x)` at the context precision.
    pub fn ln(&self, x: &BigFloat) -> OxiNumResult<BigFloat> {
        x.ln(self.precision, self.rounding)
    }

    /// Return `log_base(x)` at the context precision.
    pub fn log(&self, x: &BigFloat, base: &BigFloat) -> OxiNumResult<BigFloat> {
        x.log(base, self.precision, self.rounding)
    }

    /// Return `x^exp` at the context precision.
    pub fn pow(&self, x: &BigFloat, exp: &BigFloat) -> OxiNumResult<BigFloat> {
        x.pow(exp, self.precision, self.rounding)
    }

    /// Return `sin(x)` at the context precision.
    pub fn sin(&self, x: &BigFloat) -> OxiNumResult<BigFloat> {
        x.sin(self.precision, self.rounding)
    }

    /// Return `cos(x)` at the context precision.
    pub fn cos(&self, x: &BigFloat) -> OxiNumResult<BigFloat> {
        x.cos(self.precision, self.rounding)
    }

    /// Return `tan(x)` at the context precision.
    pub fn tan(&self, x: &BigFloat) -> OxiNumResult<BigFloat> {
        x.tan(self.precision, self.rounding)
    }

    /// Return `atan(x)` at the context precision.
    pub fn atan(&self, x: &BigFloat) -> OxiNumResult<BigFloat> {
        x.atan(self.precision, self.rounding)
    }

    /// Return `atan2(y, x)` at the context precision.
    ///
    /// `y` is the first argument (`self` in the inherent method sense);
    /// `x` is the second.
    pub fn atan2(&self, y: &BigFloat, x: &BigFloat) -> OxiNumResult<BigFloat> {
        y.atan2(x, self.precision, self.rounding)
    }

    // -----------------------------------------------------------------------
    // Mathematical constants
    // -----------------------------------------------------------------------

    /// Return π at the context precision.
    pub fn pi(&self) -> OxiNumResult<BigFloat> {
        constants::pi(self.precision)
    }

    /// Return e (Euler's number) at the context precision.
    pub fn e_const(&self) -> OxiNumResult<BigFloat> {
        constants::e_const(self.precision)
    }

    /// Return ln 2 at the context precision.
    pub fn ln2(&self) -> OxiNumResult<BigFloat> {
        constants::ln2(self.precision)
    }
}
