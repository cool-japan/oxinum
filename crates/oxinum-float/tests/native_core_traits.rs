//! Integration tests for `OxiNum` and `OxiSigned` impls on native `BigFloat`.

use oxinum_core::{OxiNum, OxiSigned, Sign};
use oxinum_float::native::{BigFloat, RoundingMode};

const MODE: RoundingMode = RoundingMode::HalfEven;

fn mk(n: i64, prec: u32) -> BigFloat {
    BigFloat::from_i64(n, prec, MODE)
}

// -----------------------------------------------------------------------
// Generic helpers that exercise the traits — NOT direct method calls on BigFloat.
// -----------------------------------------------------------------------

fn check_oxi_num<T: OxiNum>(x: &T, expect_zero: bool, expect_one: bool) {
    assert_eq!(
        T::is_zero(x),
        expect_zero,
        "is_zero mismatch (expected {expect_zero})"
    );
    assert_eq!(
        T::is_one(x),
        expect_one,
        "is_one mismatch (expected {expect_one})"
    );
}

fn check_oxi_signed<T: OxiSigned + Clone>(x: &T, expected_sign: Sign) {
    assert_eq!(
        <T as OxiSigned>::signum(x),
        expected_sign,
        "signum mismatch (expected {expected_sign:?})"
    );
    let abs_val = <T as OxiSigned>::abs(x);
    assert!(
        !<T as OxiSigned>::is_negative(&abs_val),
        "abs value should not be negative"
    );
}

// -----------------------------------------------------------------------
// OxiNum: is_zero / is_one
// -----------------------------------------------------------------------

#[test]
fn bigfloat_oxi_num_zero_is_zero_not_one() {
    let zero = BigFloat::zero(64);
    check_oxi_num(&zero, true, false);
}

#[test]
fn bigfloat_oxi_num_one_is_one_not_zero() {
    let one = mk(1, 64);
    check_oxi_num(&one, false, true);
}

#[test]
fn bigfloat_oxi_num_one_higher_precision() {
    // Verify is_one works for precisions other than 64 — the normalization
    // at different bit widths must still compare equal.
    let one_128 = mk(1, 128);
    check_oxi_num(&one_128, false, true);
}

#[test]
fn bigfloat_oxi_num_two_is_not_one() {
    let two = mk(2, 64);
    check_oxi_num(&two, false, false);
}

#[test]
fn bigfloat_oxi_num_five_is_not_one() {
    let five = mk(5, 64);
    check_oxi_num(&five, false, false);
}

#[test]
fn bigfloat_oxi_num_neg_one_is_not_one() {
    // −1 and +1 should not compare equal as "one".
    let neg_one = mk(-1, 64);
    check_oxi_num(&neg_one, false, false);
}

#[test]
fn bigfloat_oxi_num_half_is_not_one() {
    let half = BigFloat::from_f64(0.5, 64).expect("0.5");
    check_oxi_num(&half, false, false);
}

// -----------------------------------------------------------------------
// OxiSigned: signum / abs / is_negative / is_positive
// -----------------------------------------------------------------------

#[test]
fn bigfloat_oxi_signed_positive() {
    let pos = mk(5, 64);
    check_oxi_signed(&pos, Sign::Positive);
    assert!(<BigFloat as OxiSigned>::is_positive(&pos));
    assert!(!<BigFloat as OxiSigned>::is_negative(&pos));
}

#[test]
fn bigfloat_oxi_signed_negative() {
    let neg = mk(-5, 64);
    check_oxi_signed(&neg, Sign::Negative);
    assert!(!<BigFloat as OxiSigned>::is_positive(&neg));
    assert!(<BigFloat as OxiSigned>::is_negative(&neg));
}

#[test]
fn bigfloat_oxi_signed_zero_is_positive_sign() {
    // The canonical zero always has Positive sign.
    let zero = BigFloat::zero(64);
    check_oxi_signed(&zero, Sign::Positive);
    // is_zero overrides is_positive: zero is neither positive nor negative.
    assert!(!<BigFloat as OxiSigned>::is_positive(&zero));
    assert!(!<BigFloat as OxiSigned>::is_negative(&zero));
}

#[test]
fn bigfloat_oxi_signed_abs_of_negative() {
    let neg = mk(-7, 64);
    let a = <BigFloat as OxiSigned>::abs(&neg);
    assert_eq!(a.to_f64(), 7.0);
    assert!(!<BigFloat as OxiSigned>::is_negative(&a));
}

#[test]
fn bigfloat_oxi_signed_abs_of_positive_unchanged() {
    let pos = mk(7, 64);
    let a = <BigFloat as OxiSigned>::abs(&pos);
    assert_eq!(a.to_f64(), 7.0);
}

#[test]
fn bigfloat_oxi_signed_signum_one_is_positive() {
    let one = mk(1, 64);
    assert_eq!(<BigFloat as OxiSigned>::signum(&one), Sign::Positive);
}
