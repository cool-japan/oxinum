//! IEEE-754 non-finite propagation helpers for native [`BigFloat`] arithmetic.
//!
//! Two entry points:
//! - [`nonfinite_binop`] — used by the arithmetic **operators** (`+`, `-`, `*`, `/`, `%`).
//!   Handles both NaN/Inf *propagation* and non-finite *generation* (e.g. `finite / 0`
//!   produces `±Inf`; `0 / 0` or `Inf - Inf` produces `NaN`).
//! - [`nonfinite_propagate`] — input-propagation subset only (no generation from finite
//!   inputs). Used by the **checked methods** (`div_ref`, `sqrt_ref`, …) so they keep
//!   their `Err(DivByZero)` / `Err(Domain)` contract on finite-domain errors while still
//!   propagating non-finite inputs.
//!
//! # Rem special case
//!
//! For `Rem`, when `rhs` is `Infinite` and `lhs` is finite, `nonfinite_binop` returns
//! `None` — the caller should return `lhs.clone()` (IEEE rule: `finite % Inf = finite`).

use oxinum_core::Sign;

use super::float::{BigFloat, FloatClass};

/// Describes which binary operation is being performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

/// IEEE-754 binary operation table for non-finite inputs **or** non-finite results
/// generated from finite inputs (zero divisors).
///
/// Returns `Some(result)` when the result is determined by IEEE rules (either operand
/// is non-finite, or `op` generates non-finite from finite inputs such as `finite / 0`).
/// Returns `None` when both operands are finite and `op` on finite inputs is well-defined
/// (the caller should proceed with normal computation).
///
/// # Rem special case
///
/// For `Rem`, when `rhs` is `Infinite` and `lhs` is finite, returns `None` — the caller
/// should return `lhs.clone()` (IEEE rule: `finite % Inf = finite`).
pub fn nonfinite_binop(lhs: &BigFloat, rhs: &BigFloat, op: BinOp) -> Option<BigFloat> {
    let prec = lhs.precision.max(rhs.precision);

    // Universal NaN propagation: any NaN operand → NaN.
    if lhs.is_nan() || rhs.is_nan() {
        return Some(BigFloat::nan(prec));
    }

    match op {
        BinOp::Add => nonfinite_add(lhs, rhs, prec),
        BinOp::Sub => {
            // a - b == a + (-b): negate rhs and route through add.
            let neg_rhs = rhs.neg();
            nonfinite_add(lhs, &neg_rhs, prec)
        }
        BinOp::Mul => nonfinite_mul(lhs, rhs, prec),
        BinOp::Div => nonfinite_div(lhs, rhs, prec, true),
        BinOp::Rem => nonfinite_rem(lhs, rhs, prec),
    }
}

/// Input-propagation subset only: propagates NaN/Inf inputs; does **not** generate
/// Inf from finite-zero divisors (the caller keeps `Err(DivByZero)` for that case).
///
/// Returns `Some(result)` when an input is non-finite; `None` when both are finite.
pub fn nonfinite_propagate(lhs: &BigFloat, rhs: &BigFloat, op: BinOp) -> Option<BigFloat> {
    let prec = lhs.precision.max(rhs.precision);

    if lhs.is_nan() || rhs.is_nan() {
        return Some(BigFloat::nan(prec));
    }

    if lhs.is_finite() && rhs.is_finite() {
        return None; // Both finite — checked method handles domain errors itself.
    }

    // At least one operand is Infinite; route to the full table but skip
    // the finite-zero-divisor generation in Div (pass `generate_from_finite = false`).
    match op {
        BinOp::Add => nonfinite_add(lhs, rhs, prec),
        BinOp::Sub => {
            let neg_rhs = rhs.neg();
            nonfinite_add(lhs, &neg_rhs, prec)
        }
        BinOp::Mul => nonfinite_mul(lhs, rhs, prec),
        BinOp::Div => nonfinite_div(lhs, rhs, prec, false),
        BinOp::Rem => nonfinite_rem(lhs, rhs, prec),
    }
}

// ---------------------------------------------------------------------------
// Per-operation tables
// ---------------------------------------------------------------------------

fn nonfinite_add(lhs: &BigFloat, rhs: &BigFloat, prec: u32) -> Option<BigFloat> {
    match (lhs.class, rhs.class) {
        // Both Inf: same sign → Inf; opposite signs → NaN.
        (FloatClass::Infinite, FloatClass::Infinite) => {
            if lhs.sign == rhs.sign {
                Some(signed_inf(lhs.sign, prec))
            } else {
                Some(BigFloat::nan(prec))
            }
        }
        // One Inf, one finite: result is the Inf.
        (FloatClass::Infinite, FloatClass::Finite) => Some(signed_inf(lhs.sign, prec)),
        (FloatClass::Finite, FloatClass::Infinite) => Some(signed_inf(rhs.sign, prec)),
        // Both finite: caller handles.
        (FloatClass::Finite, FloatClass::Finite) => None,
        // NaN already handled in the caller.
        _ => unreachable!("NaN should have been handled before reaching nonfinite_add"),
    }
}

fn nonfinite_mul(lhs: &BigFloat, rhs: &BigFloat, prec: u32) -> Option<BigFloat> {
    match (lhs.class, rhs.class) {
        // Inf * 0 or 0 * Inf → NaN.
        (FloatClass::Infinite, FloatClass::Finite) if rhs.is_zero() => Some(BigFloat::nan(prec)),
        (FloatClass::Finite, FloatClass::Infinite) if lhs.is_zero() => Some(BigFloat::nan(prec)),
        // Inf * Inf or Inf * nonzero-finite → ±Inf (sign = XOR of signs).
        (FloatClass::Infinite, FloatClass::Infinite)
        | (FloatClass::Infinite, FloatClass::Finite)
        | (FloatClass::Finite, FloatClass::Infinite) => {
            let sign = xor_sign(lhs.sign, rhs.sign);
            Some(signed_inf(sign, prec))
        }
        // Both finite: caller handles.
        (FloatClass::Finite, FloatClass::Finite) => None,
        _ => unreachable!("NaN should have been handled before reaching nonfinite_mul"),
    }
}

fn nonfinite_div(
    lhs: &BigFloat,
    rhs: &BigFloat,
    prec: u32,
    generate_from_finite: bool,
) -> Option<BigFloat> {
    match (lhs.class, rhs.class) {
        // Inf / Inf = NaN.
        (FloatClass::Infinite, FloatClass::Infinite) => Some(BigFloat::nan(prec)),
        // Inf / finite.
        (FloatClass::Infinite, FloatClass::Finite) => {
            // Inf / 0 = NaN (indeterminate).
            if rhs.is_zero() {
                Some(BigFloat::nan(prec))
            } else {
                Some(signed_inf(xor_sign(lhs.sign, rhs.sign), prec))
            }
        }
        // finite / Inf = ±0 (sign from XOR, but we use canonical +0).
        (FloatClass::Finite, FloatClass::Infinite) => Some(BigFloat::zero(prec)),
        // finite / finite.
        (FloatClass::Finite, FloatClass::Finite) => {
            if generate_from_finite && rhs.is_zero() {
                // finite / 0 → ±Inf (sign from numerator; canonical zero has no sign).
                if lhs.is_zero() {
                    // 0 / 0 = NaN.
                    Some(BigFloat::nan(prec))
                } else {
                    Some(signed_inf(lhs.sign, prec))
                }
            } else {
                None // Caller handles (either Err(DivByZero) or normal division).
            }
        }
        _ => unreachable!("NaN should have been handled before reaching nonfinite_div"),
    }
}

fn nonfinite_rem(lhs: &BigFloat, rhs: &BigFloat, prec: u32) -> Option<BigFloat> {
    match (lhs.class, rhs.class) {
        // Inf % anything = NaN.
        (FloatClass::Infinite, _) => Some(BigFloat::nan(prec)),
        // anything % 0 = NaN.
        (FloatClass::Finite, FloatClass::Finite) if rhs.is_zero() => Some(BigFloat::nan(prec)),
        // finite % Inf: IEEE says the result is the finite value (lhs unchanged).
        // Return None to signal "caller should return lhs.clone()".
        // See module-level docs for Wave 2 contract.
        (FloatClass::Finite, FloatClass::Infinite) => None,
        // Both finite, rhs nonzero: caller handles.
        (FloatClass::Finite, FloatClass::Finite) => None,
        _ => unreachable!("NaN should have been handled before reaching nonfinite_rem"),
    }
}

// ---------------------------------------------------------------------------
// Sign helpers
// ---------------------------------------------------------------------------

fn xor_sign(a: Sign, b: Sign) -> Sign {
    if a == b {
        Sign::Positive
    } else {
        Sign::Negative
    }
}

fn signed_inf(sign: Sign, prec: u32) -> BigFloat {
    if sign == Sign::Negative {
        BigFloat::neg_infinity(prec)
    } else {
        BigFloat::infinity(prec)
    }
}

// ---------------------------------------------------------------------------
// Tests — also serve to suppress dead_code warnings on Wave-1 items that
// Wave-2 operator files will consume once they are written.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{BigFloat, RoundingMode};

    const P: u32 = 53;

    fn finite(v: i64) -> BigFloat {
        BigFloat::from_i64(v, P, RoundingMode::HalfEven)
    }

    // -----------------------------------------------------------------------
    // nonfinite_binop: Add
    // -----------------------------------------------------------------------

    #[test]
    fn add_nan_propagates() {
        let nan = BigFloat::nan(P);
        let fin = finite(1);
        let r = nonfinite_binop(&nan, &fin, BinOp::Add);
        assert!(r.expect("should produce NaN").is_nan());
        let r2 = nonfinite_binop(&fin, &nan, BinOp::Add);
        assert!(r2.expect("should produce NaN").is_nan());
    }

    #[test]
    fn add_inf_plus_inf_same_sign() {
        let pos_inf = BigFloat::infinity(P);
        let r =
            nonfinite_binop(&pos_inf, &pos_inf, BinOp::Add).expect("+inf + +inf should be +inf");
        assert!(r.is_infinite());
        assert!(r.is_sign_positive());
    }

    #[test]
    fn add_inf_plus_neg_inf_is_nan() {
        let pos_inf = BigFloat::infinity(P);
        let neg_inf = BigFloat::neg_infinity(P);
        let r = nonfinite_binop(&pos_inf, &neg_inf, BinOp::Add).expect("+inf + -inf should be NaN");
        assert!(r.is_nan());
    }

    #[test]
    fn add_inf_plus_finite() {
        let pos_inf = BigFloat::infinity(P);
        let fin = finite(42);
        let r = nonfinite_binop(&pos_inf, &fin, BinOp::Add).expect("+inf + finite should be +inf");
        assert!(r.is_infinite() && r.is_sign_positive());
    }

    #[test]
    fn add_both_finite_returns_none() {
        let a = finite(1);
        let b = finite(2);
        assert!(nonfinite_binop(&a, &b, BinOp::Add).is_none());
    }

    // -----------------------------------------------------------------------
    // nonfinite_binop: Sub (routes through neg + add)
    // -----------------------------------------------------------------------

    #[test]
    fn sub_inf_minus_inf_same_sign_is_nan() {
        let pos_inf = BigFloat::infinity(P);
        let r = nonfinite_binop(&pos_inf, &pos_inf, BinOp::Sub).expect("+inf - +inf should be NaN");
        assert!(r.is_nan());
    }

    // -----------------------------------------------------------------------
    // nonfinite_binop: Mul
    // -----------------------------------------------------------------------

    #[test]
    fn mul_inf_times_zero_is_nan() {
        let pos_inf = BigFloat::infinity(P);
        let zero = BigFloat::zero(P);
        let r = nonfinite_binop(&pos_inf, &zero, BinOp::Mul).expect("inf * 0 should be NaN");
        assert!(r.is_nan());
    }

    #[test]
    fn mul_inf_times_nonzero_finite() {
        let pos_inf = BigFloat::infinity(P);
        let fin = finite(3);
        let r = nonfinite_binop(&pos_inf, &fin, BinOp::Mul).expect("+inf * 3 should be +inf");
        assert!(r.is_infinite() && r.is_sign_positive());
    }

    #[test]
    fn mul_pos_inf_times_neg_finite() {
        let pos_inf = BigFloat::infinity(P);
        let neg = finite(-3);
        let r = nonfinite_binop(&pos_inf, &neg, BinOp::Mul).expect("+inf * -3 should be -inf");
        assert!(r.is_infinite() && r.is_sign_negative());
    }

    #[test]
    fn mul_inf_times_inf() {
        let pos_inf = BigFloat::infinity(P);
        let neg_inf = BigFloat::neg_infinity(P);
        let r =
            nonfinite_binop(&pos_inf, &neg_inf, BinOp::Mul).expect("+inf * -inf should be -inf");
        assert!(r.is_infinite() && r.is_sign_negative());
    }

    // -----------------------------------------------------------------------
    // nonfinite_binop: Div
    // -----------------------------------------------------------------------

    #[test]
    fn div_inf_over_inf_is_nan() {
        let pos_inf = BigFloat::infinity(P);
        let r = nonfinite_binop(&pos_inf, &pos_inf, BinOp::Div).expect("inf/inf should be NaN");
        assert!(r.is_nan());
    }

    #[test]
    fn div_finite_over_zero() {
        let fin = finite(5);
        let zero = BigFloat::zero(P);
        let r = nonfinite_binop(&fin, &zero, BinOp::Div).expect("5/0 should be +inf");
        assert!(r.is_infinite() && r.is_sign_positive());
    }

    #[test]
    fn div_zero_over_zero_is_nan() {
        let zero = BigFloat::zero(P);
        let r = nonfinite_binop(&zero, &zero, BinOp::Div).expect("0/0 should be NaN");
        assert!(r.is_nan());
    }

    #[test]
    fn div_finite_over_inf_is_zero() {
        let fin = finite(7);
        let pos_inf = BigFloat::infinity(P);
        let r = nonfinite_binop(&fin, &pos_inf, BinOp::Div).expect("7/inf should be zero");
        assert!(r.is_zero());
    }

    // -----------------------------------------------------------------------
    // nonfinite_binop: Rem
    // -----------------------------------------------------------------------

    #[test]
    fn rem_inf_mod_anything_is_nan() {
        let pos_inf = BigFloat::infinity(P);
        let fin = finite(3);
        let r = nonfinite_binop(&pos_inf, &fin, BinOp::Rem).expect("inf % 3 should be NaN");
        assert!(r.is_nan());
    }

    #[test]
    fn rem_finite_mod_zero_is_nan() {
        let fin = finite(5);
        let zero = BigFloat::zero(P);
        let r = nonfinite_binop(&fin, &zero, BinOp::Rem).expect("5 % 0 should be NaN");
        assert!(r.is_nan());
    }

    #[test]
    fn rem_finite_mod_inf_returns_none() {
        // Caller should return lhs.clone() (IEEE: finite % Inf = finite).
        let fin = finite(5);
        let pos_inf = BigFloat::infinity(P);
        let r = nonfinite_binop(&fin, &pos_inf, BinOp::Rem);
        assert!(
            r.is_none(),
            "finite % inf should return None (caller returns lhs)"
        );
    }

    // -----------------------------------------------------------------------
    // nonfinite_propagate
    // -----------------------------------------------------------------------

    #[test]
    fn propagate_both_finite_returns_none() {
        let a = finite(1);
        let b = finite(2);
        assert!(nonfinite_propagate(&a, &b, BinOp::Add).is_none());
    }

    #[test]
    fn propagate_nan_returns_nan() {
        let nan = BigFloat::nan(P);
        let fin = finite(1);
        assert!(nonfinite_propagate(&nan, &fin, BinOp::Add)
            .expect("should be NaN")
            .is_nan());
    }

    #[test]
    fn propagate_div_finite_over_zero_returns_none() {
        // nonfinite_propagate should NOT generate Inf from finite/0 — that's
        // the caller's job (Err(DivByZero)).
        let fin = finite(5);
        let zero = BigFloat::zero(P);
        assert!(nonfinite_propagate(&fin, &zero, BinOp::Div).is_none());
    }
}
