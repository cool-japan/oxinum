//! Cross-validation: CBig (decimal) vs BigComplex (native binary) agree on
//! the full arithmetic surface, and serde round-trips are correct for both.
//!
//! The file is organised into five sections:
//! 1. Known-value arithmetic cross-validation (unit tests)
//! 2. Serde round-trips (feature-gated on `serde`)
//! 3. num-traits Zero/One parity (feature-gated on `num-traits`)
//! 4. Property-based cross-validation (proptest)
//! 5. Wider-magnitude transcendental round-trips (CBig only)

use oxinum_complex::native::{BigComplex, RoundingMode};
use oxinum_complex::{CBig, DBig};
use oxinum_float::native::BigFloat;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// CBig decimal precision (significant digits).
const PREC: usize = 40;

/// BigComplex binary precision (bits).
const PREC_NAT: u32 = 53;

/// Rounding mode for all native operations.
const MODE: RoundingMode = RoundingMode::HalfEven;

/// Cross-family agreement tolerance (about 13–14 decimal digits at the above
/// precisions).
const TOL: f64 = 1e-9;

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

fn cbig(re: f64, im: f64) -> CBig {
    CBig::from_f64(re, im).expect("finite (re, im)")
}

fn native(re: f64, im: f64) -> BigComplex {
    BigComplex::from_f64(re, im, PREC_NAT).expect("finite (re, im)")
}

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    if a.is_finite() && b.is_finite() {
        (a - b).abs() < tol
    } else {
        (a.is_nan() && b.is_nan()) || a == b
    }
}

fn cross_approx(z_c: &CBig, z_n: &BigComplex, tol: f64) -> bool {
    let (cr, ci) = z_c.to_f64_parts();
    let (nr, ni) = z_n.to_f64_parts();
    approx_eq(cr, nr, tol) && approx_eq(ci, ni, tol)
}

// ---------------------------------------------------------------------------
// 1. Known-value arithmetic cross-validation
// ---------------------------------------------------------------------------

#[test]
fn add_same_result() {
    let (a, b) = (1.5f64, -0.7f64);
    let (c, d) = (2.3f64, 1.2f64);
    let cbig_r = cbig(a, b) + cbig(c, d);
    let nat_r = native(a, b) + native(c, d);
    assert!(
        cross_approx(&cbig_r, &nat_r, TOL),
        "CBig: {:?}, native: {:?}",
        cbig_r.to_f64_parts(),
        nat_r.to_f64_parts()
    );
}

#[test]
fn sub_same_result() {
    let cbig_r = cbig(3.0, 2.0) - cbig(1.5, 0.5);
    let nat_r = native(3.0, 2.0) - native(1.5, 0.5);
    assert!(
        cross_approx(&cbig_r, &nat_r, TOL),
        "CBig: {:?}, native: {:?}",
        cbig_r.to_f64_parts(),
        nat_r.to_f64_parts()
    );
}

#[test]
fn mul_same_result() {
    let cbig_r = cbig(1.0, 2.0) * cbig(3.0, 4.0);
    let nat_r = native(1.0, 2.0) * native(3.0, 4.0);
    assert!(
        cross_approx(&cbig_r, &nat_r, TOL),
        "CBig: {:?}, native: {:?}",
        cbig_r.to_f64_parts(),
        nat_r.to_f64_parts()
    );
}

#[test]
fn exp_same_result() {
    let z = (0.5, 0.3);
    let cr = cbig(z.0, z.1).exp(PREC).expect("exp");
    let nr = native(z.0, z.1).exp(PREC_NAT, MODE).expect("exp");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn ln_same_result() {
    let cr = cbig(2.0, 1.0).ln(PREC).expect("ln");
    let nr = native(2.0, 1.0).ln(PREC_NAT, MODE).expect("ln");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn sqrt_same_result() {
    let cr = cbig(3.0, 4.0).sqrt(PREC).expect("sqrt");
    let nr = native(3.0, 4.0).sqrt(PREC_NAT, MODE).expect("sqrt");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn asin_same_result() {
    let cr = cbig(0.3, 0.4).asin(PREC).expect("asin");
    let nr = native(0.3, 0.4).asin(PREC_NAT, MODE).expect("asin");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn atan_same_result() {
    let cr = cbig(1.0, 0.5).atan(PREC).expect("atan");
    let nr = native(1.0, 0.5).atan(PREC_NAT, MODE).expect("atan");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn atanh_same_result() {
    let cr = cbig(0.2, 0.1).atanh(PREC).expect("atanh");
    let nr = native(0.2, 0.1).atanh(PREC_NAT, MODE).expect("atanh");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn powi_same_result() {
    let cr = cbig(1.0, 1.0).powi(3, PREC).expect("powi");
    let nr = native(1.0, 1.0).powi(3, PREC_NAT, MODE).expect("powi");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn powi_negative_same_result() {
    let cr = cbig(2.0, 1.0).powi(-2, PREC).expect("powi -2");
    let nr = native(2.0, 1.0).powi(-2, PREC_NAT, MODE).expect("powi -2");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn powf_same_result() {
    use core::str::FromStr;
    let exp_d = DBig::from_str("1.5").expect("1.5");
    let exp_bf = BigFloat::from_f64(1.5, PREC_NAT).expect("1.5");
    let cr = cbig(2.0, 1.0).powf(&exp_d, PREC).expect("powf");
    let nr = native(2.0, 1.0)
        .powf(&exp_bf, PREC_NAT, MODE)
        .expect("powf");
    assert!(
        cross_approx(&cr, &nr, TOL),
        "CBig: {:?}, native: {:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn polar_roundtrip_cbig() {
    // to_polar then from_polar gives back approximately the same complex (CBig).
    let z_c = cbig(3.0, 4.0);
    let (r_c, t_c) = z_c.to_polar(PREC).expect("to_polar cbig");
    let back_c = CBig::from_polar(&r_c, &t_c, PREC).expect("from_polar cbig");
    let (re0, im0) = z_c.to_f64_parts();
    let (re1, im1) = back_c.to_f64_parts();
    assert!(
        (re0 - re1).abs() < 1e-9 && (im0 - im1).abs() < 1e-9,
        "CBig polar roundtrip: ({re0}, {im0}) vs ({re1}, {im1})"
    );
}

#[test]
fn polar_roundtrip_native() {
    // to_polar then from_polar gives back approximately the same complex (native).
    let z_n = native(3.0, 4.0);
    let (r_n, t_n) = z_n.to_polar(PREC_NAT, MODE).expect("to_polar native");
    let back_n = BigComplex::from_polar(&r_n, &t_n, PREC_NAT, MODE).expect("from_polar native");
    let (re0, im0) = z_n.to_f64_parts();
    let (re1, im1) = back_n.to_f64_parts();
    assert!(
        (re0 - re1).abs() < 1e-9 && (im0 - im1).abs() < 1e-9,
        "native polar roundtrip: ({re0}, {im0}) vs ({re1}, {im1})"
    );
}

// ---------------------------------------------------------------------------
// 2. Serde round-trips (require --features serde)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn cbig_json_round_trip_wider() {
        let cases = [
            (0.0f64, 0.0f64),
            (1.0, 0.0),
            (0.0, 1.0),
            (std::f64::consts::PI, -std::f64::consts::E),
            (1e10, -1e10),
        ];
        for (re, im) in cases {
            let z = cbig(re, im);
            let json = serde_json::to_string(&z).expect("serialize CBig");
            let back: CBig = serde_json::from_str(&json).expect("deserialize CBig");
            assert_eq!(
                back.re().to_string(),
                z.re().to_string(),
                "re mismatch at ({re}, {im})"
            );
            assert_eq!(
                back.im().to_string(),
                z.im().to_string(),
                "im mismatch at ({re}, {im})"
            );
        }
    }

    #[test]
    fn bigcomplex_json_round_trip_wider() {
        let cases = [(0.0f64, 0.0f64), (1.0, 2.0), (-3.5, 4.25), (1e6, -1e6)];
        for (re, im) in cases {
            let z = native(re, im);
            let json = serde_json::to_string(&z).expect("serialize BigComplex");
            let back: BigComplex = serde_json::from_str(&json).expect("deserialize BigComplex");
            assert!(
                (back.re().to_f64() - z.re().to_f64()).abs() < 1e-12,
                "re mismatch at ({re}, {im})"
            );
            assert!(
                (back.im().to_f64() - z.im().to_f64()).abs() < 1e-12,
                "im mismatch at ({re}, {im})"
            );
        }
    }

    #[test]
    fn cbig_zero_serde_roundtrip() {
        let z = CBig::zero();
        let json = serde_json::to_string(&z).expect("serialize CBig::zero");
        let back: CBig = serde_json::from_str(&json).expect("deserialize CBig::zero");
        assert!(back.is_zero(), "deserialized CBig zero is not zero: {json}");
    }

    #[test]
    fn bigcomplex_zero_serde_roundtrip() {
        let z = BigComplex::zero(PREC_NAT);
        let json = serde_json::to_string(&z).expect("serialize BigComplex::zero");
        let back: BigComplex = serde_json::from_str(&json).expect("deserialize BigComplex::zero");
        assert!(
            back.is_zero(),
            "deserialized BigComplex zero is not zero: {json}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. num-traits Zero/One parity
// ---------------------------------------------------------------------------

#[cfg(feature = "num-traits")]
mod num_traits_parity {
    use super::*;
    use num_traits::{One, Zero};

    #[test]
    fn cbig_zero_is_additive_identity() {
        let z = CBig::from_f64(3.0, 4.0).expect("ok");
        let zero: CBig = Zero::zero();
        let sum = &z + &zero;
        // sum should equal z; compare via cross_approx against the native version
        assert!(
            cross_approx(&sum, &native(3.0, 4.0), 1e-12),
            "CBig z + 0 != z: {:?}",
            sum.to_f64_parts()
        );
    }

    #[test]
    fn bigcomplex_zero_is_additive_identity() {
        let z = native(3.0, 4.0);
        let zero: BigComplex = Zero::zero();
        let sum = &z + &zero;
        let (re, im) = sum.to_f64_parts();
        assert!((re - 3.0).abs() < 1e-12, "re: {re}");
        assert!((im - 4.0).abs() < 1e-12, "im: {im}");
    }

    #[test]
    fn cbig_one_is_multiplicative_identity() {
        let z = CBig::from_f64(2.5, 1.5).expect("ok");
        let one: CBig = One::one();
        let prod = &z * &one;
        assert!(
            cross_approx(&prod, &native(2.5, 1.5), 1e-12),
            "CBig z * 1 != z: {:?}",
            prod.to_f64_parts()
        );
    }

    #[test]
    fn bigcomplex_one_is_multiplicative_identity() {
        let z = native(2.5, 1.5);
        let one: BigComplex = One::one();
        // BigComplex does not implement the Mul operator — use mul_core which
        // is accessible via the internal free function on BigComplex.
        // Instead multiply via the * operator (which is implemented for &BigComplex).
        let prod = &z * &one;
        let (re, im) = prod.to_f64_parts();
        assert!((re - 2.5).abs() < 1e-12, "re: {re}");
        assert!((im - 1.5).abs() < 1e-12, "im: {im}");
    }

    #[test]
    fn both_have_zero_and_one() {
        // Structural parity: both types implement Zero and One.
        let _: CBig = Zero::zero();
        let _: CBig = One::one();
        let _: BigComplex = Zero::zero();
        let _: BigComplex = One::one();
    }

    #[test]
    fn cbig_zero_is_zero() {
        let z: CBig = Zero::zero();
        assert!(z.is_zero(), "CBig::zero() should satisfy is_zero()");
    }

    #[test]
    fn bigcomplex_zero_is_zero() {
        let z: BigComplex = Zero::zero();
        assert!(z.is_zero(), "BigComplex::zero() should satisfy is_zero()");
    }

    #[test]
    fn cbig_one_is_one() {
        let o: CBig = One::one();
        assert!(o.is_one(), "CBig::one() should satisfy is_one()");
    }

    #[test]
    fn bigcomplex_one_is_one() {
        let o: BigComplex = One::one();
        assert!(o.is_one(), "BigComplex::one() should satisfy is_one()");
    }

    #[test]
    fn cbig_zero_and_one_cross_agree() {
        // CBig zero and one at f64 projection agree with native zero and one.
        let cz: CBig = Zero::zero();
        let nz: BigComplex = Zero::zero();
        assert!(
            cross_approx(&cz, &nz, 1e-15),
            "CBig zero vs native zero mismatch"
        );

        let co: CBig = One::one();
        let no: BigComplex = One::one();
        assert!(
            cross_approx(&co, &no, 1e-15),
            "CBig one vs native one mismatch"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Property-based cross-validation
// ---------------------------------------------------------------------------

/// Reduced case count for expensive transcendental proptests.
const HEAVY_CASES: u32 = 16;

proptest! {
    #[test]
    fn prop_add_cbig_native_agree(
        re1 in -5.0f64..5.0f64,
        im1 in -5.0f64..5.0f64,
        re2 in -5.0f64..5.0f64,
        im2 in -5.0f64..5.0f64,
    ) {
        let cr = cbig(re1, im1) + cbig(re2, im2);
        let nr = native(re1, im1) + native(re2, im2);
        prop_assert!(
            cross_approx(&cr, &nr, TOL),
            "add mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }

    #[test]
    fn prop_sub_cbig_native_agree(
        re1 in -5.0f64..5.0f64,
        im1 in -5.0f64..5.0f64,
        re2 in -5.0f64..5.0f64,
        im2 in -5.0f64..5.0f64,
    ) {
        let cr = cbig(re1, im1) - cbig(re2, im2);
        let nr = native(re1, im1) - native(re2, im2);
        prop_assert!(
            cross_approx(&cr, &nr, TOL),
            "sub mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }

    #[test]
    fn prop_mul_cbig_native_agree(
        re1 in -5.0f64..5.0f64,
        im1 in -5.0f64..5.0f64,
        re2 in -5.0f64..5.0f64,
        im2 in -5.0f64..5.0f64,
    ) {
        let cr = cbig(re1, im1) * cbig(re2, im2);
        let nr = native(re1, im1) * native(re2, im2);
        prop_assert!(
            cross_approx(&cr, &nr, TOL),
            "mul mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: HEAVY_CASES, ..ProptestConfig::default() })]

    #[test]
    fn prop_exp_cbig_native_agree(
        // Keep small to avoid huge magnitudes.
        re in -2.0f64..2.0f64,
        im in -2.0f64..2.0f64,
    ) {
        let cr = cbig(re, im).exp(PREC).expect("exp");
        let nr = native(re, im).exp(PREC_NAT, MODE).expect("exp");
        prop_assert!(
            cross_approx(&cr, &nr, TOL),
            "exp mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }

    #[test]
    fn prop_sqrt_cbig_native_agree(
        re in -5.0f64..5.0f64,
        im in -5.0f64..5.0f64,
    ) {
        let cr = cbig(re, im).sqrt(PREC).expect("sqrt");
        let nr = native(re, im).sqrt(PREC_NAT, MODE).expect("sqrt");
        prop_assert!(
            cross_approx(&cr, &nr, TOL),
            "sqrt mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }

    #[test]
    fn prop_powi_cbig_native_agree(
        re in -3.0f64..3.0f64,
        im in -3.0f64..3.0f64,
        n in 1i32..6i32,
    ) {
        let cr = cbig(re, im).powi(n, PREC).expect("powi");
        let nr = native(re, im).powi(n, PREC_NAT, MODE).expect("powi");
        prop_assert!(
            cross_approx(&cr, &nr, 1e-8),
            "powi mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }

    #[test]
    fn prop_asin_cbig_native_agree(
        // Restrict to values well within the convergence region for asin.
        re in -0.9f64..0.9f64,
        im in -0.9f64..0.9f64,
    ) {
        let cr = cbig(re, im).asin(PREC).expect("asin");
        let nr = native(re, im).asin(PREC_NAT, MODE).expect("asin");
        prop_assert!(
            cross_approx(&cr, &nr, TOL),
            "asin mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }

    #[test]
    fn prop_atanh_cbig_native_agree(
        re in -0.9f64..0.9f64,
        im in -0.9f64..0.9f64,
    ) {
        let cr = cbig(re, im).atanh(PREC).expect("atanh");
        let nr = native(re, im).atanh(PREC_NAT, MODE).expect("atanh");
        prop_assert!(
            cross_approx(&cr, &nr, TOL),
            "atanh mismatch: cbig={:?} native={:?}",
            cr.to_f64_parts(),
            nr.to_f64_parts()
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Wider-magnitude transcendental round-trips (CBig only — large decimal
//    values where the decimal precision advantage is most useful).
// ---------------------------------------------------------------------------

#[test]
fn ln_exp_roundtrip_wide_cbig() {
    // z with larger real component.
    let z = cbig(5.0, 3.0);
    let ln_z = z.ln(PREC).expect("ln");
    let back = ln_z.exp(PREC).expect("exp");
    let (re0, im0) = z.to_f64_parts();
    let (re1, im1) = back.to_f64_parts();
    assert!((re0 - re1).abs() < 1e-9, "re: {re0} vs {re1}");
    assert!((im0 - im1).abs() < 1e-9, "im: {im0} vs {im1}");
}

#[test]
fn sqrt_squared_roundtrip_wide_cbig() {
    let z = cbig(7.0, -11.0);
    let sq = z.sqrt(PREC).expect("sqrt");
    let back = &sq * &sq;
    let (re0, im0) = z.to_f64_parts();
    let (re1, im1) = back.to_f64_parts();
    assert!((re0 - re1).abs() < 1e-9, "re: {re0} vs {re1}");
    assert!((im0 - im1).abs() < 1e-9, "im: {im0} vs {im1}");
}

#[test]
fn powi_large_n_cbig() {
    // (1+i)^8 = ((1+i)^2)^4 = (2i)^4 = 16
    let r = cbig(1.0, 1.0).powi(8, PREC).expect("powi");
    let (re, im) = r.to_f64_parts();
    assert!((re - 16.0).abs() < 1e-9, "re = {re}");
    assert!(im.abs() < 1e-9, "im = {im}");
}

#[test]
fn powi_large_n_agrees_cross() {
    // Both families agree on a moderately large power.
    let n = 5i32;
    let cr = cbig(1.2, 0.8).powi(n, PREC).expect("powi");
    let nr = native(1.2, 0.8).powi(n, PREC_NAT, MODE).expect("powi");
    assert!(
        cross_approx(&cr, &nr, 1e-8),
        "powi({n}) mismatch: cbig={:?} native={:?}",
        cr.to_f64_parts(),
        nr.to_f64_parts()
    );
}

#[test]
fn abs_arg_cross_agree() {
    // |3+4i| = 5 and arg(3+4i) = atan2(4,3) — both families agree.
    let cr_abs = cbig(3.0, 4.0).abs(PREC).expect("abs");
    let nr_abs = native(3.0, 4.0).abs(PREC_NAT, MODE).expect("abs");
    assert!(
        (cr_abs.to_f64().value() - nr_abs.to_f64()).abs() < TOL,
        "abs mismatch: cbig={} native={}",
        cr_abs.to_f64().value(),
        nr_abs.to_f64()
    );

    let cr_arg = cbig(3.0, 4.0).arg(PREC).expect("arg");
    let nr_arg = native(3.0, 4.0).arg(PREC_NAT, MODE).expect("arg");
    assert!(
        (cr_arg.to_f64().value() - nr_arg.to_f64()).abs() < TOL,
        "arg mismatch: cbig={} native={}",
        cr_arg.to_f64().value(),
        nr_arg.to_f64()
    );
}
