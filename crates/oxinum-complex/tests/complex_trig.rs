//! Integration tests for [`oxinum_complex::CBig`] trigonometric and
//! hyperbolic functions.
//!
//! Coverage:
//!
//! * boundary values — `sin/cos/sinh/cosh` at `z = 0`;
//! * the Pythagorean identities `sin²z + cos²z = 1` and
//!   `cosh²z − sinh²z = 1` at an off-axis `z`;
//! * `tan z = sin z / cos z` consistency through the public ops; and
//! * a closed-form value `sin(i) = i·sinh(1)` (re ≈ 0, im ≈ 1.1752).
//!
//! All comparisons are on the `f64` projection; the identity sums are checked
//! at a slightly looser `1e-9` because they accumulate several guard-rounded
//! products before projection.

use oxinum_complex::CBig;

/// Working precision (significant decimal digits).
const PREC: usize = 45;

/// Build a `CBig` from two `f64`s.
fn c(re: f64, im: f64) -> CBig {
    CBig::from_f64(re, im).expect("finite parts")
}

// ---------------------------------------------------------------------------
// Boundary values at z = 0
// ---------------------------------------------------------------------------

#[test]
fn sin_zero_is_zero() {
    let (re, im) = CBig::zero().sin(PREC).expect("sin").to_f64_parts();
    assert!(re.abs() < 1e-12 && im.abs() < 1e-12, "sin(0) = {re}+{im}i");
}

#[test]
fn cos_zero_is_one() {
    let (re, im) = CBig::zero().cos(PREC).expect("cos").to_f64_parts();
    assert!(
        (re - 1.0).abs() < 1e-12 && im.abs() < 1e-12,
        "cos(0) = {re}+{im}i"
    );
}

#[test]
fn sinh_zero_is_zero() {
    let (re, im) = CBig::zero().sinh(PREC).expect("sinh").to_f64_parts();
    assert!(re.abs() < 1e-12 && im.abs() < 1e-12, "sinh(0) = {re}+{im}i");
}

#[test]
fn cosh_zero_is_one() {
    let (re, im) = CBig::zero().cosh(PREC).expect("cosh").to_f64_parts();
    assert!(
        (re - 1.0).abs() < 1e-12 && im.abs() < 1e-12,
        "cosh(0) = {re}+{im}i"
    );
}

// ---------------------------------------------------------------------------
// Pythagorean identities at an off-axis point
// ---------------------------------------------------------------------------

#[test]
fn sin_sq_plus_cos_sq_is_one() {
    // sin²z + cos²z = 1 at z = 0.5 + 0.3i.
    let z = c(0.5, 0.3);
    let s = z.sin(PREC).expect("sin");
    let co = z.cos(PREC).expect("cos");
    let sum = &(&s * &s) + &(&co * &co);
    let (re, im) = sum.to_f64_parts();
    assert!((re - 1.0).abs() < 1e-9, "re(sin²+cos²) = {re}");
    assert!(im.abs() < 1e-9, "im(sin²+cos²) = {im}");
}

#[test]
fn cosh_sq_minus_sinh_sq_is_one() {
    // cosh²z − sinh²z = 1 at z = 0.5 + 0.3i.
    let z = c(0.5, 0.3);
    let ch = z.cosh(PREC).expect("cosh");
    let sh = z.sinh(PREC).expect("sinh");
    let diff = &(&ch * &ch) - &(&sh * &sh);
    let (re, im) = diff.to_f64_parts();
    assert!((re - 1.0).abs() < 1e-9, "re(cosh²−sinh²) = {re}");
    assert!(im.abs() < 1e-9, "im(cosh²−sinh²) = {im}");
}

// ---------------------------------------------------------------------------
// tan = sin / cos consistency
// ---------------------------------------------------------------------------

#[test]
fn tan_equals_sin_over_cos() {
    let z = c(0.5, 0.3);
    let t = z.tan(PREC).expect("tan");
    let q = z
        .sin(PREC)
        .expect("sin")
        .checked_div(&z.cos(PREC).expect("cos"))
        .expect("non-zero cos");
    let (tre, tim) = t.to_f64_parts();
    let (qre, qim) = q.to_f64_parts();
    assert!((tre - qre).abs() < 1e-12, "re: {tre} vs {qre}");
    assert!((tim - qim).abs() < 1e-12, "im: {tim} vs {qim}");
}

#[test]
fn tanh_equals_sinh_over_cosh() {
    let z = c(0.4, 0.7);
    let t = z.tanh(PREC).expect("tanh");
    let q = z
        .sinh(PREC)
        .expect("sinh")
        .checked_div(&z.cosh(PREC).expect("cosh"))
        .expect("non-zero cosh");
    let (tre, tim) = t.to_f64_parts();
    let (qre, qim) = q.to_f64_parts();
    assert!((tre - qre).abs() < 1e-12, "re: {tre} vs {qre}");
    assert!((tim - qim).abs() < 1e-12, "im: {tim} vs {qim}");
}

// ---------------------------------------------------------------------------
// Known value: sin(i) = i·sinh(1)
// ---------------------------------------------------------------------------

#[test]
fn sin_of_i_is_i_sinh_one() {
    // sin(i) = i·sinh(1): re = 0, im = sinh(1) ≈ 1.1752011936438014.
    let r = CBig::i().sin(PREC).expect("sin");
    let (re, im) = r.to_f64_parts();
    assert!(re.abs() < 1e-12, "re(sin i) = {re}");
    assert!((im - 1.0_f64.sinh()).abs() < 1e-12, "im(sin i) = {im}");
}

#[test]
fn cos_of_i_is_cosh_one() {
    // cos(i) = cosh(1): re = cosh(1) ≈ 1.5430806348152437, im = 0.
    let r = CBig::i().cos(PREC).expect("cos");
    let (re, im) = r.to_f64_parts();
    assert!((re - 1.0_f64.cosh()).abs() < 1e-12, "re(cos i) = {re}");
    assert!(im.abs() < 1e-12, "im(cos i) = {im}");
}
