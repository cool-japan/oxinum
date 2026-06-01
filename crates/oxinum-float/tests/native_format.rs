//! Integration tests for the native `BigFloat` extended string formatting
//! (item EF1): decimal scientific / engineering notation and C99 `%a`-style
//! hexadecimal float parse/format.
//!
//! Coverage:
//!
//! - Engineering notation: worked examples, exponent always ≡ 0 (mod 3),
//!   negative values, several `sig_digits` widths, zero.
//! - Scientific notation: worked examples and digit-width control.
//! - Hex float round-trip: `from_hex_float(to_hex_string(x)) == x`
//!   *bit-exactly* over 200+ random `BigFloat`s (incl. zero, negative, tiny
//!   and huge exponents).
//! - Hex worked example `0x1.8p3 == 12.0`.
//! - Malformed hex inputs return `Err` (never panic).
//! - Fuzz: random ASCII through `from_hex_float` only ever returns `Ok`/`Err`.

use oxinum_core::Sign;
use oxinum_float::native::{BigFloat, RoundingMode};
use oxinum_int::native::BigUint;

const HE: RoundingMode = RoundingMode::HalfEven;

// ---------------------------------------------------------------------------
// Small deterministic PRNG (xorshift64*) so the tests are dependency-free and
// reproducible.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

// ===========================================================================
// Engineering notation
// ===========================================================================

#[test]
fn engineering_worked_examples() {
    // 12345 → 12.345e3
    let x = BigFloat::from_i64(12345, 64, HE);
    assert_eq!(x.to_engineering_string(5), "12.345e3");

    // 0.0001234 → 123.4e-6  (4 significant digits)
    let y = BigFloat::from_f64(0.0001234, 60).expect("finite");
    assert_eq!(y.to_engineering_string(4), "123.4e-6");

    // 1.0 → 1e0  (single significant digit)
    let one = BigFloat::from_i64(1, 64, HE);
    assert_eq!(one.to_engineering_string(1), "1e0");
}

#[test]
fn engineering_one_more_digit_widths() {
    let one = BigFloat::from_i64(1, 64, HE);
    // With three significant digits the mantissa keeps trailing zeros.
    assert_eq!(one.to_engineering_string(3), "1.00e0");

    let x = BigFloat::from_i64(12345, 64, HE);
    assert_eq!(x.to_engineering_string(3), "12.3e3");
    assert_eq!(x.to_engineering_string(2), "12e3");
    assert_eq!(x.to_engineering_string(1), "10e3"); // 1 sig digit, 2 int digits
}

#[test]
fn engineering_negative_values() {
    let x = BigFloat::from_i64(-12345, 64, HE);
    assert_eq!(x.to_engineering_string(5), "-12.345e3");

    let y = BigFloat::from_f64(-0.0001234, 60).expect("finite");
    assert_eq!(y.to_engineering_string(4), "-123.4e-6");
}

#[test]
fn engineering_exponent_always_multiple_of_three() {
    // Drive many magnitudes (positive and negative powers) and assert the
    // engineering exponent is always ≡ 0 (mod 3).
    let mut rng = Rng::new(0xEEEE_0001);
    for _ in 0..500 {
        // Construct a value m·2^e with random small m and a wide exponent.
        let m = BigUint::from_u64(1 + (rng.next_u64() % (1u64 << 40)));
        let e = (rng.next_u64() % 400) as i64 - 200;
        let val = BigFloat::from_parts(Sign::Positive, m, e, 64, HE);
        if val.is_zero() {
            continue;
        }
        for sig in [1usize, 2, 3, 4, 7, 10] {
            let s = val.to_engineering_string(sig);
            let exp = parse_e_exponent(&s);
            assert_eq!(
                exp.rem_euclid(3),
                0,
                "engineering exponent {exp} not a multiple of 3 (sig={sig}, s={s})"
            );
            // Mantissa (the part before 'e') must lie in [1, 1000) in magnitude.
            let mant = mantissa_magnitude(&s);
            assert!(
                (1.0..1000.0).contains(&mant),
                "engineering mantissa {mant} out of [1,1000) (s={s})"
            );
        }
    }
}

#[test]
fn engineering_zero() {
    let z = BigFloat::zero(53);
    assert_eq!(z.to_engineering_string(1), "0e0");
    assert_eq!(z.to_engineering_string(4), "0.000e0");
}

/// Extract the integer exponent that follows the `'e'` in a rendered string.
fn parse_e_exponent(s: &str) -> i64 {
    let idx = s.find('e').expect("rendered string has an 'e'");
    s[idx + 1..].parse::<i64>().expect("exponent parses")
}

/// Parse the magnitude of the mantissa portion (before `'e'`) as an `f64`.
fn mantissa_magnitude(s: &str) -> f64 {
    let idx = s.find('e').expect("rendered string has an 'e'");
    let mant = &s[..idx];
    let mant = mant.strip_prefix('-').unwrap_or(mant);
    mant.parse::<f64>().expect("mantissa parses")
}

// ===========================================================================
// Scientific notation
// ===========================================================================

#[test]
fn scientific_worked_examples() {
    let x = BigFloat::from_i64(12345, 64, HE);
    assert_eq!(x.to_scientific_string(5), "1.2345e4");
    assert_eq!(x.to_scientific_string(3), "1.23e4");
    assert_eq!(x.to_scientific_string(1), "1e4");

    let one = BigFloat::from_i64(1, 64, HE);
    assert_eq!(one.to_scientific_string(1), "1e0");
    assert_eq!(one.to_scientific_string(3), "1.00e0");

    let neg = BigFloat::from_i64(-42, 64, HE);
    assert_eq!(neg.to_scientific_string(2), "-4.2e1");
}

#[test]
fn scientific_rounding_carry() {
    // 9.999… rounding up to 3 sig digits rolls into 1.00e1.
    let x = BigFloat::from_f64(9.999, 60).expect("finite");
    assert_eq!(x.to_scientific_string(3), "1.00e1");
}

#[test]
fn scientific_small_value() {
    // 1/1024 = 9.765625e-4 exactly (power of two).
    let x = BigFloat::from_f64(0.0009765625, 60).expect("finite");
    assert_eq!(x.to_scientific_string(7), "9.765625e-4");
}

// ===========================================================================
// Hex float
// ===========================================================================

#[test]
fn hex_worked_example() {
    let x = BigFloat::from_hex_float("0x1.8p3", 53).expect("valid");
    assert_eq!(x.to_f64(), 12.0);

    let twelve = BigFloat::from_f64(12.0, 53).expect("finite");
    assert_eq!(x, twelve);

    // And formatting reproduces it.
    assert_eq!(twelve.to_hex_string(), "0x1.8p3");
}

#[test]
fn hex_zero() {
    let z = BigFloat::zero(53);
    assert_eq!(z.to_hex_string(), "0x0p0");
    let parsed = BigFloat::from_hex_float("0x0p0", 53).expect("valid");
    assert!(parsed.is_zero());
    // Other spellings of zero parse to zero too.
    assert!(BigFloat::from_hex_float("0x0.0p5", 53)
        .expect("valid")
        .is_zero());
    assert!(BigFloat::from_hex_float("-0x0p0", 53)
        .expect("valid")
        .is_zero());
}

#[test]
fn hex_integer_part_more_than_one() {
    // 0x1a.bp0 = (0x1ab) × 2^(-4) = 427 / 16 = 26.6875.
    let x = BigFloat::from_hex_float("0x1a.bp0", 60).expect("valid");
    assert_eq!(x.to_f64(), 26.6875);
}

#[test]
fn hex_negative_and_signs() {
    let x = BigFloat::from_hex_float("-0x1.8p3", 53).expect("valid");
    assert_eq!(x.to_f64(), -12.0);
    assert_eq!(x.sign(), Sign::Negative);
    // Explicit + sign on significand and exponent.
    let y = BigFloat::from_hex_float("+0x1.8p+3", 53).expect("valid");
    assert_eq!(y.to_f64(), 12.0);
    let z = BigFloat::from_hex_float("0x1p-2", 53).expect("valid");
    assert_eq!(z.to_f64(), 0.25);
}

#[test]
fn hex_uppercase_markers() {
    let x = BigFloat::from_hex_float("0X1.8P3", 53).expect("valid");
    assert_eq!(x.to_f64(), 12.0);
    let y = BigFloat::from_hex_float("0x1.AbP0", 60).expect("valid");
    // 0x1.Ab = 1 + 0xAb/256 = 1 + 171/256 = 427/256.
    assert_eq!(y.to_f64(), 427.0 / 256.0);
}

#[test]
fn hex_round_trip_random_bit_exact() {
    // Round-trip from_hex_float(to_hex_string(x)) == x, bit-exactly, over a
    // wide spread of random BigFloats: random sign, mantissa, exponent, and
    // precision, including zero and extreme exponents.
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let mut checked = 0usize;
    for i in 0..300 {
        // Precision in [1, 256].
        let prec = 1 + (rng.next_u64() % 256) as u32;
        // Mantissa: a few random limbs (could be zero).
        let limbs = (rng.next_u64() % 4) as usize; // 0..=3 limbs
        let mut bytes = Vec::new();
        for _ in 0..limbs {
            bytes.extend_from_slice(&rng.next_u64().to_le_bytes());
        }
        let mantissa = BigUint::from_bytes_le(&bytes);
        // Exponent: spread across a very wide range, occasionally extreme.
        let raw = rng.next_u64();
        let exponent: i64 = match i % 4 {
            0 => (raw % 2_000_000) as i64 - 1_000_000,
            1 => (raw % 200) as i64 - 100,
            2 => i64::MAX / 2 - (raw % 1000) as i64, // huge positive
            _ => i64::MIN / 2 + (raw % 1000) as i64, // huge negative
        };
        let sign = if raw & 1 == 0 {
            Sign::Positive
        } else {
            Sign::Negative
        };
        let x = BigFloat::from_parts(sign, mantissa, exponent, prec, HE);

        let s = x.to_hex_string();
        let back = BigFloat::from_hex_float(&s, x.precision())
            .unwrap_or_else(|e| panic!("re-parse of {s:?} failed: {e:?}"));
        assert_eq!(back, x, "hex round-trip mismatch: {s:?}");
        // Also assert the reconstructed precision matches (so the bit pattern
        // is identical, not merely numerically equal).
        assert_eq!(back.precision(), x.precision(), "precision drift: {s:?}");
        checked += 1;
    }
    assert!(
        checked >= 200,
        "expected to check >= 200 values, got {checked}"
    );
}

#[test]
fn hex_round_trip_specific_powers() {
    // Exact powers of two and simple fractions across a range.
    for k in -60i64..=60 {
        let two_k = BigFloat::from_parts(Sign::Positive, BigUint::one(), k, 32, HE);
        let s = two_k.to_hex_string();
        let back = BigFloat::from_hex_float(&s, two_k.precision()).expect("valid");
        assert_eq!(back, two_k, "power 2^{k} round-trip via {s}");
    }
}

#[test]
fn hex_malformed_inputs_err() {
    let bad = [
        "",
        "0x",
        "0X",
        "1.5p3",     // missing 0x
        "0x1.5",     // missing p
        "0xG.1p0",   // invalid hex digit
        "0x1.1pZ",   // non-decimal exponent
        "0x1.8p",    // missing exponent digits
        "0xp3",      // no significand digits
        "0x.p3",     // no significand digits
        "0x1.8p3x",  // trailing garbage
        "  0x1.8p3", // leading whitespace
        "0x1.8 p3",  // internal space
        "0x1.8p3 ",  // trailing whitespace
        "0x1.8p1e3", // exponent not pure decimal
        "++0x1p0",   // double sign
    ];
    for s in bad {
        let r = BigFloat::from_hex_float(s, 53);
        assert!(
            r.is_err(),
            "expected Err for malformed input {s:?}, got {r:?}"
        );
    }
}

#[test]
fn hex_parser_never_panics_on_random_ascii() {
    // Drive random ASCII (and some structured-ish) strings through the parser
    // and assert it only ever returns Ok or Err — never panics.
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    let alphabet: &[u8] = b"0123456789abcdefABCDEFxXpP+-. ";
    for _ in 0..5000 {
        let n = (rng.next_u64() % 16) as usize;
        let mut s = String::with_capacity(n);
        for _ in 0..n {
            let c = alphabet[(rng.next_u64() as usize) % alphabet.len()];
            s.push(c as char);
        }
        // Result is intentionally ignored; the assertion is "does not panic".
        let _ = BigFloat::from_hex_float(&s, 53);
    }
    // Also throw fully-arbitrary bytes (filtered to printable ASCII).
    for _ in 0..5000 {
        let n = (rng.next_u64() % 20) as usize;
        let mut s = String::with_capacity(n);
        for _ in 0..n {
            let c = (0x20 + (rng.next_u64() % 0x5F) as u8) as char;
            s.push(c);
        }
        let _ = BigFloat::from_hex_float(&s, 53);
    }
}
