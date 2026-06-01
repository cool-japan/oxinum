//! Proptest-based parser harness for `BigFloat` hex-float parsing and
//! `BigRational` string parsing.
//!
//! Two dimensions per parser:
//!
//! 1. **Valid input round-trip** — generate a value, format it, parse it back.
//!    The result must equal the original (bit-exact for hex float).
//!
//! 2. **Garbage input safety** — feed arbitrary strings to the parser. The
//!    contract is: it must return `Ok(x)` or `Err(...)`, **never panic, never
//!    hang, never loop infinitely**.
//!
//! Run with:
//!   `cargo nextest run -p oxinum-float --all-features`

use num_traits::Num;
use oxinum_float::native::{BigFloat, RoundingMode};
use oxinum_rational::native::BigRational;
use proptest::prelude::*;

const HE: RoundingMode = RoundingMode::HalfEven;
/// Fixed precision used throughout: 64-bit significand.
const PREC: u32 = 64;

// ---------------------------------------------------------------------------
// BigFloat hex-float — Dimension A: valid round-trip
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(256))]

    /// `from_hex_float(to_hex_string(x), x.precision()) == x`, bit-exactly,
    /// for any integer-valued BigFloat derived from a random i64.
    #[test]
    fn hex_float_roundtrip_from_i64(n in any::<i64>()) {
        // Skip zero — zero always round-trips, and from_i64(0, PREC, HE)
        // returns the canonical zero which formats as "0x0p0".
        if n == 0 {
            return Ok(());
        }
        let x = BigFloat::from_i64(n, PREC, HE);
        let s = x.to_hex_string();
        let y = BigFloat::from_hex_float(&s, x.precision())
            .expect("to_hex_string output must be parseable by from_hex_float");
        prop_assert_eq!(x, y, "hex float round-trip failed for n={}", n);
    }

    /// Same round-trip for f64 values — covers subnormals, large/small
    /// exponents, and fractions that don't reduce to an integer.
    #[test]
    fn hex_float_roundtrip_from_f64(bits in any::<u64>()) {
        let f = f64::from_bits(bits);
        // Skip NaN and Inf — BigFloat::from_f64 returns Err for those.
        if !f.is_finite() {
            return Ok(());
        }
        let x = match BigFloat::from_f64(f, PREC) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        let s = x.to_hex_string();
        let y = BigFloat::from_hex_float(&s, x.precision())
            .expect("to_hex_string output must be parseable by from_hex_float");
        prop_assert_eq!(x, y, "hex float round-trip failed for f64 bits={:#018x}", bits);
    }
}

// ---------------------------------------------------------------------------
// BigFloat hex-float — Dimension B: garbage input safety
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(512))]

    /// Any string, when passed to `from_hex_float`, must return `Ok` or `Err`
    /// and must never panic.
    #[test]
    fn from_hex_float_garbage_no_panic(s in any::<String>()) {
        let _ = BigFloat::from_hex_float(&s, PREC);
    }

    /// Strings starting with "0x" followed by random hex-like characters must
    /// never panic — they may parse successfully or return an error.
    #[test]
    fn from_hex_float_0x_prefix_garbage_no_panic(
        rest in "[0-9a-fA-F.pP+\\-]{0,100}"
    ) {
        let s = format!("0x{rest}");
        let _ = BigFloat::from_hex_float(&s, PREC);
    }

    /// Strings that look like complete hex floats but with a garbage exponent.
    #[test]
    fn from_hex_float_struct_garbage_no_panic(
        int_hex in "[0-9a-fA-F]{0,20}",
        frac_hex in "[0-9a-fA-F]{0,20}",
        exp_part in any::<String>()
    ) {
        let s = format!("0x{int_hex}.{frac_hex}p{exp_part}");
        let _ = BigFloat::from_hex_float(&s, PREC);
    }
}

// ---------------------------------------------------------------------------
// BigFloat hex-float — deterministic edge cases
// ---------------------------------------------------------------------------

#[test]
fn hex_float_edge_cases() {
    // Canonical worked example: 0x1.8p3 = 1.5 × 2^3 = 12.
    let result = BigFloat::from_hex_float("0x1.8p3", 53).expect("0x1.8p3 must parse");
    assert_eq!(result.to_f64(), 12.0, "0x1.8p3 must equal 12.0");

    // Zero in several forms.
    let z = BigFloat::from_hex_float("0x0p0", 53).expect("0x0p0 must parse");
    assert!(z.is_zero(), "0x0p0 must be zero");
    let z2 = BigFloat::from_hex_float("0x0.0p5", 53).expect("0x0.0p5 must parse");
    assert!(z2.is_zero(), "0x0.0p5 must be zero");

    // Negative zero stays zero.
    let nz = BigFloat::from_hex_float("-0x0p0", 53).expect("-0x0p0 must parse");
    assert!(nz.is_zero(), "-0x0p0 must be zero");

    // Empty string → error.
    assert!(
        BigFloat::from_hex_float("", PREC).is_err(),
        "empty string must fail"
    );
    // Missing 0x prefix → error.
    assert!(
        BigFloat::from_hex_float("1.8p3", PREC).is_err(),
        "no prefix must fail"
    );
    // Missing p exponent → error.
    assert!(
        BigFloat::from_hex_float("0x1.8", PREC).is_err(),
        "no exponent must fail"
    );
    // No significand digits → error.
    assert!(
        BigFloat::from_hex_float("0xp3", PREC).is_err(),
        "no significand must fail"
    );
    // Trailing garbage → error.
    assert!(
        BigFloat::from_hex_float("0x1.8p3x", PREC).is_err(),
        "trailing garbage must fail"
    );
    // Leading whitespace → error.
    assert!(
        BigFloat::from_hex_float("  0x1.8p3", PREC).is_err(),
        "leading space must fail"
    );

    // Very long garbage string — must not panic.
    let garbage: String = "!@#$%^&*".repeat(200);
    let _ = BigFloat::from_hex_float(&garbage, PREC);

    // Very long valid hex float string — must parse without panic.
    let mantissa: String = "1".repeat(500);
    let s = format!("0x{mantissa}p0");
    let _ = BigFloat::from_hex_float(&s, PREC);
}

// ---------------------------------------------------------------------------
// BigRational string parsing — Dimension A: valid round-trip
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(256))]

    /// `from_str_radix(to_string(r), 10) == r` for any rational formed from
    /// random integer numerator and positive denominator.
    #[test]
    fn bigrational_roundtrip_integer(n in any::<i64>()) {
        let r = BigRational::from_i64(n);
        let s = r.to_string();
        let back: BigRational = Num::from_str_radix(&s, 10)
            .expect("to_string output must be parseable back by from_str_radix");
        prop_assert_eq!(r, back, "BigRational round-trip failed for integer n={}", n);
    }

    /// Fraction round-trip: a/b round-trips through to_string/from_str_radix.
    #[test]
    fn bigrational_roundtrip_fraction(
        num in any::<i32>(),
        den in 1i32..=i32::MAX
    ) {
        use oxinum_int::native::{BigInt, BigUint};
        let r = BigRational::from_parts(
            BigInt::from(num as i64),
            BigUint::from_u64(den as u64),
        )
        .expect("non-zero denominator");
        let s = r.to_string();
        let back: BigRational = Num::from_str_radix(&s, 10)
            .expect("to_string output must be parseable back");
        prop_assert_eq!(r, back, "BigRational round-trip failed for {}/{}", num, den);
    }
}

// ---------------------------------------------------------------------------
// BigRational string parsing — Dimension B: garbage input safety
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(512))]

    /// Any string must not panic when passed to `from_str_radix(s, 10)`.
    #[test]
    fn bigrational_garbage_no_panic(s in any::<String>()) {
        let _: Result<BigRational, _> = Num::from_str_radix(&s, 10);
    }

    /// Non-10 radices must return an error (not panic) for any string.
    #[test]
    fn bigrational_nonbase10_no_panic(
        s in any::<String>(),
        radix in 2u32..=36u32
    ) {
        if radix == 10 {
            return Ok(());
        }
        // BigRational::from_str_radix only supports radix 10; any other radix
        // must return Err without panicking.
        let _: Result<BigRational, _> = Num::from_str_radix(&s, radix);
    }
}

// ---------------------------------------------------------------------------
// BigRational string parsing — deterministic edge cases
// ---------------------------------------------------------------------------

#[test]
fn bigrational_edge_cases() {
    // Zero parses to zero.
    let z: BigRational = Num::from_str_radix("0", 10).expect("0 must parse");
    assert!(z.is_zero(), "0 must be zero");

    // Integer strings.
    let n: BigRational = Num::from_str_radix("42", 10).expect("42 must parse");
    assert_eq!(n, BigRational::from_i64(42));

    // Fraction strings.
    let r: BigRational = Num::from_str_radix("3/4", 10).expect("3/4 must parse");
    assert_eq!(r.to_string(), "3/4");

    // Negative integer.
    let neg: BigRational = Num::from_str_radix("-7", 10).expect("-7 must parse");
    assert_eq!(neg, BigRational::from_i64(-7));

    // Negative fraction.
    let nf: BigRational = Num::from_str_radix("-5/2", 10).expect("-5/2 must parse");
    assert_eq!(nf.to_string(), "-5/2");

    // Auto-reduction: 6/4 → 3/2.
    let reduced: BigRational = Num::from_str_radix("6/4", 10).expect("6/4 must parse");
    assert_eq!(reduced.to_string(), "3/2");

    // Empty string → error.
    let r: Result<BigRational, _> = Num::from_str_radix("", 10);
    assert!(r.is_err(), "empty string must fail");

    // Zero denominator → error.
    let r: Result<BigRational, _> = Num::from_str_radix("1/0", 10);
    assert!(r.is_err(), "1/0 must fail");

    // Non-base-10 radix → error (documented limitation).
    let r: Result<BigRational, _> = Num::from_str_radix("ff", 16);
    assert!(r.is_err(), "radix 16 must return error");

    // Garbage strings → error, not panic.
    // Note: "++1" parses as +1 (double-plus is accepted); that is documented
    // behaviour, not a bug, so it is not in this list.
    let garbage_inputs = ["abc!@#", "1/", "/1", "1/2/3", " 3/4", "3/ 4"];
    for s in &garbage_inputs {
        let r: Result<BigRational, _> = Num::from_str_radix(s, 10);
        assert!(r.is_err(), "garbage input {s:?} must fail");
    }

    // Very long valid integer string.
    let big_int_str: String = "9".repeat(2000);
    let _: BigRational =
        Num::from_str_radix(&big_int_str, 10).expect("very large integer must parse");

    // Very long garbage string — must not panic.
    let garbage: String = "!@#$%^&*".repeat(200);
    let _: Result<BigRational, _> = Num::from_str_radix(&garbage, 10);
}
