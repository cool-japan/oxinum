//! Known-value integration tests for [`oxinum_complex::CBig`] transcendentals.
//!
//! Each case fixes a complex argument whose `exp`/`ln`/`sqrt`/`abs`/`arg`/`pow`
//! has a closed-form value and checks the result two ways:
//!
//! * float projection — `to_f64_parts()` compared to the reference with an
//!   absolute tolerance of `1e-12`; and
//! * exact decimal strings — for the algebraic identities (`(1+i)² = 2i`,
//!   `|3+4i| = 5`) that land on integers no rounding can perturb.
//!
//! π is built at the working precision with [`oxinum_float::compute_pi`] (which
//! returns a [`DBig`] directly, not a `Result`). All transcendentals run at
//! precision 40–50 significant decimal digits.
//!
//! The off-axis `pow`/`z^1` case at `z = 2 − 3i` exercises the recently fixed
//! `atan2`/`atan` path in `oxinum-float`: with the fix in place the
//! `exp(w·ln z)` round trip is accurate to full precision, so a tight `1e-12`
//! tolerance is expected to hold.

use core::str::FromStr;

use oxinum_complex::{CBig, DBig};
use oxinum_float::compute_pi;

/// Absolute tolerance for `f64`-projected comparisons.
const TOL: f64 = 1e-12;

/// Working precision (significant decimal digits) for the transcendentals.
const PREC: usize = 40;

/// Precision used when materialising π (a couple of extra digits of headroom).
const PI_PREC: usize = 50;

/// Parse a decimal literal into a [`DBig`].
fn d(s: &str) -> DBig {
    DBig::from_str(s).expect("valid decimal literal")
}

/// Assert that `(re, im)` is within [`TOL`] of `(re_ref, im_ref)`.
fn assert_close(parts: (f64, f64), re_ref: f64, im_ref: f64, label: &str) {
    let (re, im) = parts;
    assert!(
        (re - re_ref).abs() < TOL,
        "{label}: re = {re}, expected {re_ref}"
    );
    assert!(
        (im - im_ref).abs() < TOL,
        "{label}: im = {im}, expected {im_ref}"
    );
}

// ---------------------------------------------------------------------------
// exp
// ---------------------------------------------------------------------------

#[test]
fn exp_i_pi_is_minus_one() {
    // Euler's identity: exp(iπ) = −1 + 0i.
    let z = CBig::from_parts(d("0"), compute_pi(PI_PREC));
    let r = z.exp(PREC).expect("exp");
    assert_close(r.to_f64_parts(), -1.0, 0.0, "exp(iπ)");
}

#[test]
fn exp_zero_is_one() {
    // exp(0) = 1 + 0i.
    let r = CBig::zero().exp(PREC).expect("exp");
    assert_close(r.to_f64_parts(), 1.0, 0.0, "exp(0)");
}

// ---------------------------------------------------------------------------
// ln
// ---------------------------------------------------------------------------

#[test]
fn ln_minus_one_is_i_pi() {
    // ln(−1) = 0 + iπ.
    let z = CBig::from_real(d("-1"));
    let r = z.ln(PREC).expect("ln");
    assert_close(r.to_f64_parts(), 0.0, std::f64::consts::PI, "ln(−1)");
}

#[test]
fn ln_i_is_half_i_pi() {
    // ln(i) = 0 + i·π/2.
    let r = CBig::i().ln(PREC).expect("ln");
    assert_close(r.to_f64_parts(), 0.0, std::f64::consts::FRAC_PI_2, "ln(i)");
}

#[test]
fn ln_zero_is_err() {
    // ln(0) is undefined → Err.
    assert!(CBig::zero().ln(PREC).is_err(), "ln(0) should error");
}

// ---------------------------------------------------------------------------
// sqrt
// ---------------------------------------------------------------------------

#[test]
fn sqrt_minus_one_is_i() {
    // sqrt(−1) = 0 + i  (principal branch).
    let z = CBig::from_real(d("-1"));
    let r = z.sqrt(PREC).expect("sqrt");
    assert_close(r.to_f64_parts(), 0.0, 1.0, "sqrt(−1)");
}

#[test]
fn sqrt_two_i_is_one_plus_i() {
    // sqrt(2i) = 1 + i.
    let z = CBig::from_parts(d("0"), d("2"));
    let r = z.sqrt(PREC).expect("sqrt");
    assert_close(r.to_f64_parts(), 1.0, 1.0, "sqrt(2i)");
}

#[test]
fn sqrt_zero_is_zero() {
    // sqrt(0) = 0 (exact).
    let r = CBig::zero().sqrt(PREC).expect("sqrt");
    assert!(r.is_zero(), "sqrt(0) should be exactly zero");
}

// ---------------------------------------------------------------------------
// Algebraic identities via the Mul operator (exact integer strings)
// ---------------------------------------------------------------------------

#[test]
fn one_plus_i_squared_is_two_i_exact() {
    // (1 + i)² = 2i exactly. Components are built with `from_f64` so the
    // `DBig`s carry full (17-significant-digit) precision; the cross products
    // (`1·1`) are then exact and the result prints as the bare integers
    // "0" and "2". (Constructing via the `i64` `From` would seed each `DBig`
    // with only one significant digit, which is safe for `1·1` but not, e.g.,
    // `4·4` — see `abs_three_four_is_five`.)
    let z = CBig::from_f64(1.0, 1.0).expect("finite parts");
    let sq = &z * &z;
    assert_eq!(sq.re().to_string(), "0", "re of (1+i)²");
    assert_eq!(sq.im().to_string(), "2", "im of (1+i)²");
}

#[test]
fn abs_three_four_is_five() {
    // |3 + 4i| = 5; the prefix check tolerates trailing guard digits.
    //
    // Built with `from_f64`: an `i64`/short-string-constructed `DBig` carries
    // only one significant digit, so `4·4` inside `norm_sqr` would round to 20
    // (giving |z| = √30). `from_f64` seeds 17 significant digits, so `norm_sqr`
    // is the exact 25.
    let z = CBig::from_f64(3.0, 4.0).expect("finite parts");
    let m = z.abs(PREC).expect("abs");
    assert!(m.to_string().starts_with('5'), "|3+4i| = {m}");
    // And via the f64 projection for good measure.
    assert!((m.to_f64().value() - 5.0).abs() < TOL, "|3+4i| f64 = {m}");
}

// ---------------------------------------------------------------------------
// arg
// ---------------------------------------------------------------------------

#[test]
fn arg_of_i_is_half_pi() {
    // arg(i) = π/2.
    let a = CBig::i().arg(PREC).expect("arg");
    assert!(
        (a.to_f64().value() - std::f64::consts::FRAC_PI_2).abs() < TOL,
        "arg(i) = {a}"
    );
}

#[test]
fn arg_of_minus_one_is_pi() {
    // arg(−1) = π.
    let a = CBig::from_real(d("-1")).arg(PREC).expect("arg");
    assert!(
        (a.to_f64().value() - std::f64::consts::PI).abs() < TOL,
        "arg(−1) = {a}"
    );
}

#[test]
fn arg_of_one_plus_i_is_quarter_pi() {
    // arg(1 + i) = π/4.
    let a = CBig::from_f64(1.0, 1.0)
        .expect("finite parts")
        .arg(PREC)
        .expect("arg");
    assert!(
        (a.to_f64().value() - std::f64::consts::FRAC_PI_4).abs() < TOL,
        "arg(1+i) = {a}"
    );
}

// ---------------------------------------------------------------------------
// pow
// ---------------------------------------------------------------------------

/// The real exponent `n + 0i` at full precision (`from_f64`, not the
/// single-significant-digit `i64` `From`).
fn real_exp(n: f64) -> CBig {
    CBig::from_f64(n, 0.0).expect("finite exponent")
}

#[test]
fn pow_i_squared_is_minus_one() {
    // i² = −1.
    let r = CBig::i().pow(&real_exp(2.0), PREC).expect("pow");
    assert_close(r.to_f64_parts(), -1.0, 0.0, "i²");
}

#[test]
fn pow_off_axis_to_one_is_identity() {
    // z^1 = z for an OFF-AXIS z = 2 − 3i. Post-atan2-fix, the exp(1·ln z)
    // round trip is accurate to full precision even off the real axis. The
    // operand uses `from_f64` so `norm_sqr` (hence `ln`'s real part) is exact.
    let z = CBig::from_f64(2.0, -3.0).expect("finite parts");
    let r = z.pow(&CBig::one(), PREC).expect("pow");
    assert_close(r.to_f64_parts(), 2.0, -3.0, "(2−3i)^1");
}

#[test]
fn pow_one_plus_i_squared_is_two_i() {
    // (1 + i)^2 = 2i, via the transcendental pow path (exp∘ln).
    let z = CBig::from_f64(1.0, 1.0).expect("finite parts");
    let r = z.pow(&real_exp(2.0), PREC).expect("pow");
    assert_close(r.to_f64_parts(), 0.0, 2.0, "(1+i)^2");
}
