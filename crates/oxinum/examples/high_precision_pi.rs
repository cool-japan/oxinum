//! Compute pi to N significant decimal digits using OxiNum.
//!
//! This example demonstrates two routes:
//!  1. The pre-computed `oxinum::constants::pi(N)` reference (fast).
//!  2. A live computation via Machin's formula
//!     `pi/4 = 4*atan(1/5) - atan(1/239)`
//!     using `oxinum::atan` at a guard precision.
//!
//! Run:
//!     cargo run --example high_precision_pi --release -- 80
//!
//! The numeric argument is the number of significant digits to display.
//! Defaults to 50 digits when omitted.

use oxinum::{atan, constants, DBig};
use std::env;
use std::str::FromStr;

fn main() {
    let digits: usize = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    println!("--- pi to {digits} significant digits ---");

    // Route 1: pre-stored constant (truncated to `digits`).
    let pi_ref = constants::pi(digits);
    println!("constants::pi({digits}) = {pi_ref}");

    // Route 2: live Machin computation at a guard precision.
    let guard = digits + 20;
    let machin_pi = machin_pi(guard);
    let machin_pi_clamped = machin_pi.with_precision(digits).value();
    println!("Machin           = {machin_pi_clamped}");

    // Show how many leading characters agree (sanity check).
    let r = pi_ref.to_string();
    let m = machin_pi_clamped.to_string();
    let common = r.chars().zip(m.chars()).take_while(|(a, b)| a == b).count();
    println!("common prefix length = {common}");
}

/// Compute pi via Machin's formula at the given guard precision.
fn machin_pi(guard: usize) -> DBig {
    let one_fifth = compute_one_over_n(5, guard);
    let one_239 = compute_one_over_n(239, guard);
    let atan_fifth = atan(&one_fifth, guard).expect("atan(1/5)");
    let atan_239 = atan(&one_239, guard).expect("atan(1/239)");
    let four = dbig_at_precision("4.0", guard);
    let pi_over_4 = &(&four * &atan_fifth) - &atan_239;
    &pi_over_4 * &four
}

fn compute_one_over_n(n: u64, precision: usize) -> DBig {
    let one = DBig::from(1u32).with_precision(precision).value();
    let n_big = DBig::from(n).with_precision(precision).value();
    (&one / &n_big).with_precision(precision).value()
}

fn dbig_at_precision(s: &str, precision: usize) -> DBig {
    let v = DBig::from_str(s).expect("valid decimal literal");
    v.with_precision(precision).value()
}
