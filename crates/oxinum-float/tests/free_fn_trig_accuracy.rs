//! Accuracy regression tests for the `DBig` free-function `atan` / `atan2`.
//!
//! These functions live in `crates/oxinum-float/src/trig.rs` and operate on
//! `dashu_float::DBig` arguments (distinct from the `native::BigFloat::atan`
//! method, which is exercised in `native_transcendentals.rs`).
//!
//! Regression context: arguments that entered `atan_small`'s argument-halving
//! loop carrying only a few significant decimal digits (for example the
//! ratio `2.0 / 3.0`, which `DBig` division rounds to `0.67`) caused the
//! intermediate arithmetic to collapse to that narrow precision, yielding a
//! precision-independent error of roughly 3e-3 for inputs such as `2/3`,
//! `0.75`, and `1.5`. The fix carries guard digits through the ratio
//! formation and the halving loop. These tests pin the corrected behaviour:
//! at precision 50 the absolute error must be below 1e-40 across the range.
//!
//! Reference values were produced with `mpmath` at 50 decimal digits.

use dashu_float::DBig;
use oxinum_float::{atan, atan2};
use std::str::FromStr;

const PREC: usize = 50;

fn dbig(s: &str) -> DBig {
    DBig::from_str(s).expect("valid decimal literal")
}

/// Count the number of leading zeros in the fractional part of `|value|`.
///
/// For a value such as `0.000…0d…` this returns the number of `0` digits
/// between the decimal point and the first non-zero digit, which is a direct
/// lower bound on `-log10(|value|)`. Returns `usize::MAX` for an exact zero.
fn fractional_leading_zeros(value: &DBig) -> usize {
    let s = value.to_string();
    let s = s.trim_start_matches('-');
    // An exact zero is "0" or "0.0…"; treat it as infinitely small error.
    let all_zero = s.chars().all(|c| c == '0' || c == '.');
    if all_zero {
        return usize::MAX;
    }
    match s.find('.') {
        Some(dot) => {
            let int_part = &s[..dot];
            if int_part != "0" {
                // Magnitude >= 1, i.e. error >= 1 -> zero leading zeros.
                0
            } else {
                s[dot + 1..].chars().take_while(|&c| c == '0').count()
            }
        }
        // No decimal point: an integer; if it were "0" we'd have returned
        // above, so the magnitude is >= 1.
        None => 0,
    }
}

/// Assert that `atan`/`atan2` output matches `expected` to better than 1e-40.
fn assert_accurate(name: &str, actual: &DBig, expected: &str) {
    let diff = actual - &dbig(expected);
    let lz = fractional_leading_zeros(&diff);
    assert!(
        lz >= 40,
        "{name}: error too large (|diff| has only {lz} leading fractional zeros, \
         need >= 40)\n  actual   = {actual}\n  expected = {expected}\n  diff     = {diff}"
    );
}

#[test]
fn atan_accuracy_within_unit() {
    // |x| <= 0.5: direct Taylor, no halving.
    assert_accurate(
        "atan(0.5)",
        &atan(&dbig("0.5"), PREC).expect("atan(0.5)"),
        "0.46364760900080611621425623146121440202853705428612",
    );
    // |x| in (0.5, 1]: one or more halvings.
    assert_accurate(
        "atan(0.75)",
        &atan(&dbig("0.75"), PREC).expect("atan(0.75)"),
        "0.64350110879328438680280922871732263804151059111531",
    );
    assert_accurate(
        "atan(1)",
        &atan(&dbig("1.0"), PREC).expect("atan(1)"),
        "0.78539816339744830961566084581987572104929234984378",
    );
}

#[test]
fn atan_accuracy_low_precision_ratio() {
    // A high-precision `2/3` argument exercises the halving loop with a long
    // repeating-decimal input. (Note: `2.0 / 3.0` formed from two-digit `DBig`
    // literals rounds to `0.67` *before* reaching `atan`, so the ratio must be
    // computed at guard precision by the caller; `atan2` does this internally,
    // see `atan2_accuracy_quadrant_i`.)
    let two = dbig("2.0").with_precision(PREC + 20).value();
    let three = dbig("3.0").with_precision(PREC + 20).value();
    let two_thirds = &two / &three;
    assert_accurate(
        "atan(2/3)",
        &atan(&two_thirds, PREC).expect("atan(2/3)"),
        "0.58800260354756755124561108062508542760170724605592",
    );
}

#[test]
fn atan_accuracy_above_unit() {
    // |x| > 1: reciprocal path, atan(x) = pi/2 - atan(1/x).
    assert_accurate(
        "atan(1.5)",
        &atan(&dbig("1.5"), PREC).expect("atan(1.5)"),
        "0.98279372324732906798571061101466601449687745363163",
    );
    assert_accurate(
        "atan(2)",
        &atan(&dbig("2.0"), PREC).expect("atan(2)"),
        "1.1071487177940905030170654601785370400700476454014",
    );
    assert_accurate(
        "atan(10)",
        &atan(&dbig("10.0"), PREC).expect("atan(10)"),
        "1.4711276743037345918528755717617308518553063771832",
    );
}

#[test]
fn atan2_accuracy_quadrant_i() {
    assert_accurate(
        "atan2(1, 1)",
        &atan2(&dbig("1.0"), &dbig("1.0"), PREC).expect("atan2(1,1)"),
        "0.78539816339744830961566084581987572104929234984378",
    );
    assert_accurate(
        "atan2(3, 4)",
        &atan2(&dbig("3.0"), &dbig("4.0"), PREC).expect("atan2(3,4)"),
        "0.64350110879328438680280922871732263804151059111531",
    );
    assert_accurate(
        "atan2(2, 3)",
        &atan2(&dbig("2.0"), &dbig("3.0"), PREC).expect("atan2(2,3)"),
        "0.58800260354756755124561108062508542760170724605592",
    );
}

#[test]
fn atan2_accuracy_negative_x() {
    // x > 0 with y < 0: reduces to atan(y/x) with a negative argument.
    assert_accurate(
        "atan2(-3, 2)",
        &atan2(&dbig("-3.0"), &dbig("2.0"), PREC).expect("atan2(-3,2)"),
        "-0.98279372324732906798571061101466601449687745363163",
    );
    // Second quadrant: atan(y/x) + pi.
    assert_accurate(
        "atan2(1, -1)",
        &atan2(&dbig("1.0"), &dbig("-1.0"), PREC).expect("atan2(1,-1)"),
        "2.3561944901923449288469825374596271631478770495313",
    );
    // Third quadrant: atan(y/x) - pi.
    assert_accurate(
        "atan2(-1, -1)",
        &atan2(&dbig("-1.0"), &dbig("-1.0"), PREC).expect("atan2(-1,-1)"),
        "-2.3561944901923449288469825374596271631478770495313",
    );
}

#[test]
fn atan2_accuracy_on_axes() {
    // x == 0, y > 0: exactly pi/2.
    assert_accurate(
        "atan2(1, 0)",
        &atan2(&dbig("1.0"), &dbig("0.0"), PREC).expect("atan2(1,0)"),
        "1.5707963267948966192313216916397514420985846996876",
    );
    // x == 0, y < 0: exactly -pi/2.
    assert_accurate(
        "atan2(-1, 0)",
        &atan2(&dbig("-1.0"), &dbig("0.0"), PREC).expect("atan2(-1,0)"),
        "-1.5707963267948966192313216916397514420985846996876",
    );
}
