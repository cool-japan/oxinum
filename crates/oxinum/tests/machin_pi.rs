//! Integration test: compute pi via Machin's formula
//! `pi/4 = 4*atan(1/5) - atan(1/239)` using `oxinum::atan` and assert against
//! `oxinum::constants::pi(N)` to ~30 significant digits.
//!
//! End-to-end exercise of the public `oxinum` facade.

use oxinum::{atan, constants, DBig};
use std::str::FromStr;

#[test]
fn machin_pi_matches_constants_to_30_digits() {
    let target_digits = 30usize;
    let guard = target_digits + 20; // computational guard precision

    let one_fifth = compute_one_over_n(5, guard);
    let one_239 = compute_one_over_n(239, guard);

    let atan_fifth = atan(&one_fifth, guard).expect("atan(1/5)");
    let atan_239 = atan(&one_239, guard).expect("atan(1/239)");

    let four = dbig_at_precision("4.0", guard);
    let pi_over_4 = &(&four * &atan_fifth) - &atan_239;
    let machin_pi = &pi_over_4 * &four;
    let machin_pi = machin_pi.with_precision(target_digits).value();

    let constants_pi = constants::pi(target_digits);

    // Assert first 28 digits match (tight margin under guard precision;
    // `constants::pi` truncates the pre-stored string while Machin rounds,
    // so the very last digit may differ by +/- 1).
    let m = machin_pi.to_string();
    let c = constants_pi.to_string();
    let common_prefix_len = m.chars().zip(c.chars()).take_while(|(a, b)| a == b).count();
    assert!(
        common_prefix_len >= 28,
        "Machin pi = {m}\nconst  pi = {c}\ncommon prefix = {common_prefix_len}"
    );
}

#[test]
fn machin_pi_starts_with_3_14159() {
    let guard = 40usize;
    let one_fifth = compute_one_over_n(5, guard);
    let one_239 = compute_one_over_n(239, guard);
    let atan_fifth = atan(&one_fifth, guard).expect("atan(1/5)");
    let atan_239 = atan(&one_239, guard).expect("atan(1/239)");
    let four = dbig_at_precision("4.0", guard);
    let pi_over_4 = &(&four * &atan_fifth) - &atan_239;
    let machin_pi = &pi_over_4 * &four;
    let machin_pi = machin_pi.with_precision(20).value();
    let s = machin_pi.to_string();
    assert!(s.starts_with("3.14159"), "Machin pi prefix mismatch: {s}");
}

/// Compute `1/n` as a `DBig` at the given number of significant decimal
/// digits. Uses the same precision-context pattern as `oxinum-float`:
/// extend both operands to the desired precision via `with_precision`,
/// then divide and re-clamp the result.
fn compute_one_over_n(n: u64, precision: usize) -> DBig {
    let one = DBig::from(1u32).with_precision(precision).value();
    let n_big = DBig::from(n).with_precision(precision).value();
    (&one / &n_big).with_precision(precision).value()
}

/// Parse a decimal literal and extend it to `precision` significant digits.
fn dbig_at_precision(s: &str, precision: usize) -> DBig {
    let v = DBig::from_str(s).expect("valid decimal literal");
    v.with_precision(precision).value()
}
