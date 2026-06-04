//! SciRS2 rational API compatibility verification.
//!
//! Proves that `oxinum-rational` satisfies the exact contract that
//! `scirs2-core/src/numeric/arbitrary_precision.rs` depends on.
//!
//! The SciRS2 consumer imports:
//!   ```ignore
//!   use oxinum_rational::{IBig as RIBig, RBig, UBig as RUBig};
//!   use oxinum_rational::to_f64 as rbig_to_f64;
//!   use oxinum_rational::rational_abs;
//!   use oxinum_rational::rational_reciprocal;
//!   RBig::from_parts(n: RIBig, d: RUBig)
//!   RBig::from(0u32) / RBig::from(n: u32)
//!   .numerator() -> &IBig
//!   .denominator() -> &UBig
//!   ```
//!
//! It also uses `UBig::ONE`, `UBig::from_str`, `IBig::from_str`, and the
//! `rational_abs` / `rational_reciprocal` free functions.

use std::str::FromStr;

use oxinum_rational::{
    rational_abs, rational_reciprocal, to_f64 as rbig_to_f64, IBig as RIBig, RBig, UBig as RUBig,
};

// -----------------------------------------------------------------------
// Compile-time contract assertions.
// -----------------------------------------------------------------------

#[allow(dead_code)]
fn _assert_rational_contract() {
    // RBig construction.
    let _zero = RBig::from(0_u32);
    let _one = RBig::from(1_u32);

    // RBig::from_parts(n: IBig, d: UBig).
    let n = RIBig::from(22_i32);
    let d = RUBig::from(7_u32);
    let _r = RBig::from_parts(n, d);

    // .numerator() and .denominator().
    let r22_7 = RBig::from_parts(RIBig::from(22_i32), RUBig::from(7_u32));
    let _num: &RIBig = r22_7.numerator();
    let _den: &RUBig = r22_7.denominator();

    // Arithmetic ops (used by SciRS2 via Div/Mul etc.).
    let a = RBig::from_parts(RIBig::from(1_i32), RUBig::from(3_u32));
    let b = RBig::from_parts(RIBig::from(1_i32), RUBig::from(6_u32));
    let _sum = a.clone() + b.clone();
    let _diff = a.clone() - b.clone();
    let _prod = a.clone() * b.clone();
    let _quot = a / b;

    // to_f64 free function.
    let r = RBig::from(1_u32);
    let _f: f64 = rbig_to_f64(&r);

    // rational_abs.
    let neg = RBig::from_parts(RIBig::from(-5_i32), RUBig::ONE);
    let _abs = rational_abs(&neg);

    // rational_reciprocal.
    let r2 = RBig::from_parts(RIBig::from(2_i32), RUBig::from(3_u32));
    let _recip: oxinum_rational::OxiNumResult<RBig> = rational_reciprocal(&r2);

    // UBig::ONE and from_str used internally via the SciRS2 from_parts helpers.
    let _u = RUBig::ONE;
    let _u2: RUBig = RUBig::from_str("42").expect("parse");
    let _i: RIBig = RIBig::from_str("-42").expect("parse");
}

// -----------------------------------------------------------------------
// Behavioural tests
// -----------------------------------------------------------------------

#[test]
fn rbig_from_parts_22_over_7() {
    let r = RBig::from_parts(RIBig::from(22_i32), RUBig::from(7_u32));
    assert_eq!(r.to_string(), "22/7");
}

#[test]
fn rbig_arithmetic() {
    let a = RBig::from_parts(RIBig::from(1_i32), RUBig::from(3_u32));
    let b = RBig::from_parts(RIBig::from(1_i32), RUBig::from(6_u32));
    let sum = a.clone() + b.clone();
    assert_eq!(sum.to_string(), "1/2");
}

#[test]
fn rbig_numerator_denominator_accessors() {
    let r = RBig::from_parts(RIBig::from(22_i32), RUBig::from(7_u32));
    assert_eq!(r.numerator().to_string(), "22");
    assert_eq!(r.denominator().to_string(), "7");
}

#[test]
fn rbig_auto_simplification() {
    // 6/4 → 3/2 via dashu's automatic simplification.
    let r = RBig::from_parts(RIBig::from(6_i32), RUBig::from(4_u32));
    assert_eq!(r.to_string(), "3/2");
}

#[test]
fn to_f64_exact_values() {
    let half = RBig::from_parts(RIBig::from(1_i32), RUBig::from(2_u32));
    let f = rbig_to_f64(&half);
    assert!((f - 0.5).abs() < 1e-15);

    let zero = RBig::from(0_u32);
    assert_eq!(rbig_to_f64(&zero), 0.0);

    let one = RBig::from(1_u32);
    assert_eq!(rbig_to_f64(&one), 1.0);
}

#[test]
fn rational_abs_positive_unchanged() {
    let r = RBig::from_parts(RIBig::from(3_i32), RUBig::from(4_u32));
    let a = rational_abs(&r);
    assert_eq!(a.to_string(), "3/4");
}

#[test]
fn rational_abs_negative_becomes_positive() {
    let r = RBig::from_parts(RIBig::from(-3_i32), RUBig::from(4_u32));
    let a = rational_abs(&r);
    assert_eq!(a.to_string(), "3/4");
}

#[test]
fn rational_reciprocal_basic() {
    let r = RBig::from_parts(RIBig::from(2_i32), RUBig::from(3_u32));
    let recip = rational_reciprocal(&r).expect("non-zero");
    assert_eq!(recip.to_string(), "3/2");
}

#[test]
fn rational_reciprocal_of_zero_errors() {
    let zero = RBig::from(0_u32);
    assert!(rational_reciprocal(&zero).is_err());
}

#[test]
fn ubig_one_and_from_str() {
    let one = RUBig::ONE;
    assert_eq!(one.to_string(), "1");

    let n = RUBig::from_str("12345").expect("parse");
    assert_eq!(n.to_string(), "12345");
}

#[test]
fn ibig_from_str_negative() {
    let n = RIBig::from_str("-42").expect("parse");
    assert_eq!(n.to_string(), "-42");
}

#[test]
fn scirs2_arbitrary_rational_num_helper() {
    // Replicates the SciRS2 `num(num: i64, den: i64)` path:
    //   RBig::from_parts(RIBig::from(num), RUBig::from(den.unsigned_abs()))
    let num: i64 = 22;
    let den: i64 = 7;
    let n = RIBig::from(num);
    let d = RUBig::from(den.unsigned_abs());
    let r = RBig::from_parts(n, d);
    assert_eq!(r.to_string(), "22/7");
    let f = rbig_to_f64(&r);
    assert!((f - 22.0 / 7.0).abs() < 1e-13);
}

#[test]
fn scirs2_arbitrary_rational_recip_path() {
    // Replicates the SciRS2 recip() path:
    //   if self.value == RBig::from(0u32) → error
    //   else → rational_reciprocal
    let zero = RBig::from(0_u32);
    if zero == RBig::from(0_u32) {
        // Expected error path.
        assert!(rational_reciprocal(&zero).is_err());
    }

    let r = RBig::from_parts(RIBig::from(3_i32), RUBig::from(4_u32));
    if r != RBig::from(0_u32) {
        let recip = rational_reciprocal(&r).expect("non-zero");
        assert_eq!(recip.to_string(), "4/3");
    }
}

#[test]
fn scirs2_to_arbitrary_float_helper_path() {
    // Replicates SciRS2's `to_arbitrary_float` helper which does:
    //   let num_str = self.value.numerator().to_string();
    //   let den_str = self.value.denominator().to_string();
    //   let n = DBig::from_str(&num_str);
    //   let d = DBig::from_str(&den_str);
    //   n_prec / d_prec  (after setting precision via with_precision)
    //
    // The SciRS2 `to_arbitrary_float` method applies `with_precision` to both
    // operands before dividing; without it the default precision is the number
    // of significant digits in the string representation (e.g. "1" → 1 digit).
    use oxinum_float::{precision::with_precision, DBig};
    use std::str::FromStr as _;

    let r = RBig::from_parts(RIBig::from(1_i32), RUBig::from(3_u32));
    let num_str = r.numerator().to_string();
    let den_str = r.denominator().to_string();

    let digits = 30;
    let n = DBig::from_str(&num_str).expect("parse num");
    let d = DBig::from_str(&den_str).expect("parse den");
    // Apply precision (replicates `with_precision(&n, digits + 4)` in SciRS2).
    let n_prec = with_precision(&n, digits + 4);
    let d_prec = with_precision(&d, digits + 4);
    let result = with_precision(&(n_prec / d_prec), digits);
    let f: f64 = result.to_f64().value();
    assert!((f - 1.0 / 3.0).abs() < 1e-13, "1/3 path = {f}");
}
