//! Precision-control helpers for the `DBig` wrapper.
//!
//! These free functions wrap `dashu-float` precision primitives in an
//! ergonomic, allocation-friendly API:
//!
//! * [`with_precision`] — rebind a value to a different working precision.
//! * [`epsilon`] — the smallest positive *unit* representable at a given
//!   decimal precision, i.e. `10^(1 - precision)` at the requested
//!   precision.
//! * [`ulp`] — the unit in the last place of a specific value at the
//!   precision currently carried by its context.
//!
//! All three operate on the decimal big-float (`DBig = FBig<HalfAway, 10>`)
//! so callers do not have to spell out the underlying generic parameters.
//!
//! # `precision == 0` (unlimited)
//!
//! `dashu-float` reserves a `precision` of `0` to mean *unlimited*.
//! `ulp` is not meaningful in that regime and the underlying
//! `dashu_float::FBig::ulp` panics in that case.  We surface that as an
//! [`OxiNumError::Precision`] instead — see [`ulp`] for details.

use std::str::FromStr;

use crate::{DBig, OxiNumError, OxiNumResult};

/// Return `x` rebound to the requested *decimal* precision.
///
/// This delegates to [`dashu_float::FBig::with_precision`], which returns
/// an [`dashu_base::Approximation`] indicating whether the rebind was
/// exact (the new precision was at least the old digit count) or
/// inexact (digits were rounded off).  This helper discards that
/// distinction and returns the rounded value directly.  Use the raw
/// `dashu-float` API if you need the inexact-flag.
///
/// `precision == 0` is *unlimited* precision in `dashu-float`'s
/// vocabulary, which is also passed through unchanged.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use oxinum_float::{precision::with_precision, DBig};
///
/// let a = DBig::from_str("1.234").expect("parse 1.234");
/// let a100 = with_precision(&a, 100);
/// assert_eq!(a100.precision(), 100);
/// ```
#[must_use]
pub fn with_precision(x: &DBig, precision: usize) -> DBig {
    // `with_precision` consumes `self`, so we clone the borrowed input.
    // `.value()` discards the inexact-flag (see module docs).
    x.clone().with_precision(precision).value()
}

/// Return the smallest positive "unit" representable at the requested
/// decimal precision, i.e. `10^(1 - precision)` rounded to `precision`
/// significant digits.
///
/// In decimal base, a value with precision `p` has its least significant
/// digit at position `10^(1 - p)` when the leading digit is `1` —
/// this is the "epsilon" in the floating-point sense.
///
/// `precision == 0` is *unlimited* precision in `dashu-float`'s
/// vocabulary; in that case there is no meaningful machine epsilon, so
/// this helper returns an error.
///
/// # Errors
///
/// Returns [`OxiNumError::Precision`] if `precision == 0`.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use oxinum_float::{precision::epsilon, DBig};
///
/// let eps10 = epsilon(10).expect("precision 10");
/// // 10^(1 - 10) = 10^-9, at precision 10.
/// assert_eq!(eps10.precision(), 10);
///
/// let expected = DBig::from_str("1e-9")
///     .expect("parse 1e-9")
///     .with_precision(10)
///     .value();
/// assert_eq!(eps10, expected);
/// ```
pub fn epsilon(precision: usize) -> OxiNumResult<DBig> {
    if precision == 0 {
        return Err(OxiNumError::Precision(
            "epsilon is undefined for unlimited precision (precision = 0)".into(),
        ));
    }
    // Build "1e-(precision-1)" as the canonical decimal epsilon, then
    // rebind to the requested precision so callers can rely on
    // `epsilon(p).precision() == p`.
    let raw = format!("1e-{}", precision - 1);
    let parsed = DBig::from_str(&raw)
        .map_err(|e| OxiNumError::Parse(format!("epsilon('{raw}'): {e}").into()))?;
    Ok(parsed.with_precision(precision).value())
}

/// Return the unit in the last place of `x` at its current precision.
///
/// This is `dashu_float::FBig::ulp` re-exported as a free function for
/// API symmetry with [`epsilon`] and [`with_precision`].  The result is
/// a positive value whose least significant decimal position matches
/// `x` (i.e. `ulp(x)` is the smallest positive `d` such that `x + d`
/// is the next representable neighbour of `x`).
///
/// # Errors
///
/// Returns [`OxiNumError::Precision`] if `x` carries unlimited
/// precision (`x.precision() == 0`).  In that case `ulp` is
/// undefined — `dashu_float::FBig::ulp` would itself panic, which we
/// surface as a recoverable error here.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use oxinum_float::{precision::ulp, DBig};
///
/// let x = DBig::from_str("1.23").expect("parse 1.23");
/// let u = ulp(&x).expect("finite precision");
/// assert_eq!(u, DBig::from_str("0.01").expect("parse 0.01"));
/// // ulp does not change the carried precision.
/// assert_eq!(u.precision(), x.precision());
/// ```
pub fn ulp(x: &DBig) -> OxiNumResult<DBig> {
    if x.precision() == 0 {
        return Err(OxiNumError::Precision(
            "ulp is undefined for values with unlimited precision (precision = 0)".into(),
        ));
    }
    Ok(x.ulp())
}
