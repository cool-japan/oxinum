//! Proptest-based parser harness for `BigUint::from_str_radix`.
//!
//! Two dimensions:
//!
//! 1. **Valid input round-trip cross-validation** — generate a random integer,
//!    convert to a string in a given radix, then parse back. The result must
//!    equal the original.
//!
//! 2. **Garbage input safety** — feed arbitrary strings (and arbitrary radix
//!    values) to `from_str_radix`. The contract is: it must return `Ok(x)` or
//!    `Err(...)`, **never panic, never hang, never loop infinitely**.
//!
//! Run with:
//!   `cargo nextest run -p oxinum-int --all-features`

use oxinum_int::native::BigUint;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Dimension A — valid input round-trip cross-validation
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(256))]

    #[test]
    fn from_str_radix_roundtrip_base10(n in any::<u128>()) {
        let x = BigUint::from_u128(n);
        let s = x.to_radix(10).expect("to_radix(10) must succeed for valid radix");
        let y = BigUint::from_str_radix(&s, 10)
            .expect("from_str_radix must succeed for well-formed base-10 string");
        prop_assert_eq!(x, y);
    }

    #[test]
    fn from_str_radix_roundtrip_base16(n in any::<u128>()) {
        let x = BigUint::from_u128(n);
        let s = x.to_radix(16).expect("to_radix(16) must succeed for valid radix");
        let y = BigUint::from_str_radix(&s, 16)
            .expect("from_str_radix must succeed for well-formed base-16 string");
        prop_assert_eq!(x, y);
    }

    #[test]
    fn from_str_radix_roundtrip_base2(n in any::<u128>()) {
        let x = BigUint::from_u128(n);
        let s = x.to_radix(2).expect("to_radix(2) must succeed for valid radix");
        let y = BigUint::from_str_radix(&s, 2)
            .expect("from_str_radix must succeed for well-formed base-2 string");
        prop_assert_eq!(x, y);
    }

    #[test]
    fn from_str_radix_roundtrip_base36(n in any::<u128>()) {
        let x = BigUint::from_u128(n);
        let s = x.to_radix(36).expect("to_radix(36) must succeed for valid radix");
        let y = BigUint::from_str_radix(&s, 36)
            .expect("from_str_radix must succeed for well-formed base-36 string");
        prop_assert_eq!(x, y);
    }
}

// ---------------------------------------------------------------------------
// Dimension B — garbage input safety (must not panic, hang, or crash)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(512))]

    #[test]
    fn from_str_radix_garbage_base2_no_panic(s in any::<String>()) {
        // Must return Ok or Err — never panic.
        let _ = BigUint::from_str_radix(&s, 2);
    }

    #[test]
    fn from_str_radix_garbage_base10_no_panic(s in any::<String>()) {
        let _ = BigUint::from_str_radix(&s, 10);
    }

    #[test]
    fn from_str_radix_garbage_base16_no_panic(s in any::<String>()) {
        let _ = BigUint::from_str_radix(&s, 16);
    }

    #[test]
    fn from_str_radix_garbage_base36_no_panic(s in any::<String>()) {
        let _ = BigUint::from_str_radix(&s, 36);
    }

    #[test]
    fn from_str_radix_invalid_radix_no_panic(
        s in any::<String>(),
        radix in any::<u32>()
    ) {
        // Even completely invalid radices must not panic — only Err.
        let _ = BigUint::from_str_radix(&s, radix);
    }
}

// ---------------------------------------------------------------------------
// Explicit deterministic edge cases
// ---------------------------------------------------------------------------

#[test]
fn from_str_radix_edge_cases() {
    // Empty string → error.
    assert!(BigUint::from_str_radix("", 10).is_err());
    // Whitespace-only → error.
    assert!(BigUint::from_str_radix("  ", 10).is_err());
    assert!(BigUint::from_str_radix("\t", 10).is_err());
    // "0" → canonical zero.
    let z = BigUint::from_str_radix("0", 10).expect("zero parses");
    assert_eq!(z, BigUint::ZERO);
    // Leading zeros are not an error (some parsers reject them — ours accepts).
    let _ = BigUint::from_str_radix("007", 10);
    // Very long valid decimal string.
    let long = "9".repeat(2000);
    let _ = BigUint::from_str_radix(&long, 10).expect("large decimal number parses");
    // Very long valid hex string.
    let hex_long = "f".repeat(2000);
    let _ = BigUint::from_str_radix(&hex_long, 16).expect("large hex number parses");
    // Very long garbage string — must not panic.
    let garbage: String = "abc!@#$%^".repeat(100);
    let _ = BigUint::from_str_radix(&garbage, 10);
    // Invalid radix 0 → error.
    assert!(BigUint::from_str_radix("0", 0).is_err());
    // Invalid radix 1 → error.
    assert!(BigUint::from_str_radix("0", 1).is_err());
    // Invalid radix 37 → error.
    assert!(BigUint::from_str_radix("0", 37).is_err());
    // Hex in base 10 → error.
    assert!(BigUint::from_str_radix("ff", 10).is_err());
    // Binary digit in wrong base → error.
    assert!(BigUint::from_str_radix("2", 2).is_err());
    // Mixed valid/invalid → error.
    assert!(BigUint::from_str_radix("1234a", 10).is_err());
}
