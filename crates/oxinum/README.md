# oxinum — The COOLJAPAN Pure-Rust arbitrary-precision math façade

[![Crates.io](https://img.shields.io/crates/v/oxinum.svg)](https://crates.io/crates/oxinum)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`oxinum` is the top-level façade of the OxiNum numeric tower: a single dependency that re-exports arbitrary-precision integers, floats/decimals, and exact rationals, plus number-theory and elementary functions, cross-type conversions, and a universal parser. It is the COOLJAPAN Pure-Rust replacement for GMP/MPFR-based arbitrary-precision math — **100% Pure Rust**, `#![forbid(unsafe_code)]`, with the `dashu` family as its backend.

The constituent crates are [`oxinum-core`](https://crates.io/crates/oxinum-core) (traits, errors, rounding modes), [`oxinum-int`](https://crates.io/crates/oxinum-int) (`BigInt`/`BigUint`), [`oxinum-float`](https://crates.io/crates/oxinum-float) (`BigFloat`/`DBig`), and [`oxinum-rational`](https://crates.io/crates/oxinum-rational) (`BigRational`/`RBig`). Most applications only need `oxinum`.

## Dual exposure: dashu-backed default and pure-native

Two coexisting type families are exposed, with **disjoint namespaces** so neither shadows the other:

- **Crate-root re-exports** (`Int`, `Natural`, `Float`, `Rational`, `BigInt`, `BigUint`, `DBig`, `RBig`, …) are the **dashu-backed default** and the recommended entry point for application code today.
- The [`native`](#the-native-module) module re-exports the **ground-up Pure Rust** types (`native::BigInt`, `native::BigUint`, `native::BigFloat`, `native::BigRational`, plus parallel `native::Int` / `Natural` / `Float` / `Rational` aliases). Reach for these when you want zero `dashu` dependence, explicit limb / rounding-mode control, or to migrate incrementally toward the eventual native default.

## Installation

```toml
[dependencies]
oxinum = "0.1.0"

# With dashu literal macros:
oxinum = { version = "0.1.0", features = ["macros"] }
```

## Quick Start

```rust
use oxinum::prelude::*;

// Arbitrary-precision integer arithmetic.
let big = factorial(20);
assert_eq!(big.to_string(), "2432902008176640000");

// High-precision pi (50 significant digits).
let pi = constants::pi(50);
assert!(pi.to_string().starts_with("3.14159265358979"));

// Number theory.
assert_eq!(fibonacci(10), UBig::from(55u32));
assert!(is_prime(&UBig::from(17u32), 0));
```

## Top-level type aliases

The recommended, ergonomic names for the four numeric kinds:

| Alias | Underlying type | Meaning |
|-------|-----------------|---------|
| `Int` | `oxinum_int::IBig` | Arbitrary-precision signed integer. |
| `Natural` | `oxinum_int::UBig` | Arbitrary-precision unsigned integer. |
| `Float` | `oxinum_float::DBig` | Arbitrary-precision decimal float. |
| `Rational` | `oxinum_rational::RBig` | Arbitrary-precision exact rational. |

## Re-exported types at crate root

From `oxinum-core`:

- `OxiNumError`, `OxiNumResult`, `ParseNumberError`, `RoundingMode`, `Sign`

From `oxinum-int`:

- `BigInt`, `BigUint`, `IBig`, `UBig`, `Gcd`
- number theory: `factorial`, `fibonacci`, `lucas`, `binomial`, `extended_gcd`, `mod_pow`, `is_prime`, `next_prime`

From `oxinum-float`:

- `BigFloat`, `DBig`, `FBig`, `Context`
- elementary / trig: `exp`, `ln`, `sqrt`, `pow`, `sin`, `cos`, `tan`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`
- constants: `compute_pi`, `compute_e`, `compute_ln2`

From `oxinum-rational`:

- `BigRational`, `RBig`, `Relaxed`
- operations: `continued_fraction`, `from_continued_fraction`, `best_rational_approximation`, `mediant`, `mixed_number`, `to_decimal_string`, `rational_abs`, `rational_signum`, `rational_reciprocal`, `rational_pow`, `rational_floor`, `rational_ceil`, `rational_round`, `rational_truncate`

## Universal parser

Auto-detects the numeric format of a string and returns the appropriate variant:

```rust
use oxinum::{parse, ParsedNumber};

assert!(matches!(parse("42")?,   ParsedNumber::Integer(_)));  // no '.' / '/'
assert!(matches!(parse("3/4")?,  ParsedNumber::Rational(_))); // contains '/'
assert!(matches!(parse("1.25")?, ParsedNumber::Float(_)));    // contains '.'/'e'/'E'
# Ok::<(), oxinum::OxiNumError>(())
```

| Item | Signature / Kind | Description |
|------|------------------|-------------|
| `parse` | `fn(&str) -> OxiNumResult<ParsedNumber>` | Detection order: `/` → rational, `.`/`e`/`E` → float, otherwise integer. Errors on empty / unrecognised input. |
| `ParsedNumber` | enum `Integer(IBig)` / `Rational(RBig)` / `Float(DBig)` | Implements `Display`. |

## `constants` module

Discoverable, precision-parameterised mathematical constants (thin wrappers over `oxinum-float`):

| Function | Signature | Value |
|----------|-----------|-------|
| `constants::pi` | `fn(usize) -> DBig` | π. |
| `constants::e` | `fn(usize) -> DBig` | Euler's number *e*. |
| `constants::ln2` | `fn(usize) -> DBig` | ln 2. |
| `constants::sqrt2` | `fn(usize) -> DBig` | √2. |

## `convert` module

Cross-type conversions between integers, floats, and rationals, with explicit precision where a conversion is lossy:

| Function | Signature | Description |
|----------|-----------|-------------|
| `convert::int_to_float` | `fn(&IBig) -> DBig` | Exact integer → decimal float. |
| `convert::int_to_rational` | `fn(&IBig) -> RBig` | Exact integer → rational. |
| `convert::rational_to_float` | `fn(&RBig, usize) -> DBig` | Rational → float at `n` significant digits. |
| `convert::float_to_rational` | `fn(&DBig) -> OxiNumResult<RBig>` | Exact decimal float → rational (`significand / 10ⁿ`). |
| `convert::rational_to_int` | `fn(&RBig) -> IBig` | Rational → integer by truncation toward zero. |
| `convert::float_from_str` | `fn(&str) -> OxiNumResult<DBig>` | Parse a decimal float from a string. |

## `round` module

Re-exports `oxinum-float`'s rounding-mode marker types (`Down`, `HalfAway`, `HalfEven`, `Up`, `Zero`) so callers can configure a `Context` without depending on `dashu-float` directly.

## Prelude

```rust
use oxinum::prelude::*;
```

Imports the core error/rounding types and `Sign`; the integer types and number-theory functions; the float types and elementary functions; the headline rational types and operations; the `Int` / `Natural` / `Float` / `Rational` aliases; the `constants` and `convert` modules; and the `parse` function with `ParsedNumber`.

## The `native` module

The ground-up Pure Rust stack with **no `dashu` backend**, re-exported from the sibling crates' `native` modules. Coexists with the crate-root dashu-backed types — existing imports keep compiling unchanged.

```rust
use oxinum::native::{BigInt, BigUint, BigFloat, RoundingMode};

let n: BigUint = BigUint::from(10_u64);
let m: BigInt  = BigInt::from(-3_i64);
let f = BigFloat::from_i64(7, 32, RoundingMode::HalfEven);
assert_eq!(n.to_string(), "10");
assert_eq!(m.to_string(), "-3");
assert!(!f.is_zero());
```

Re-exported items include:

- **Types:** `BigInt`, `BigUint`, `BigFloat`, `RoundingMode`, `MontgomeryContext`, `BigRational`
- **Parallel aliases:** `native::Int`, `native::Natural`, `native::Float`, `native::Rational`
- **Functions:** `factorial`, `gcd`, `gcd_binary`, `gcd_int`, `gcd_extended`, `mod_inv`, `mod_mul`, `mod_pow`, `divrem`, `checked_divrem`, `divrem_int`, `is_probably_prime`, `prime_sieve`
- **Constants:** `KARATSUBA_THRESHOLD`, `NEWTON_DIV_THRESHOLD`

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `pure` | on | The 100% Pure-Rust configuration (default). |
| `macros` | off | Enables `dashu-macros` for compile-time numeric literals. |

## Version

```rust
let v: &str = oxinum::version(); // returns env!("CARGO_PKG_VERSION")
```

## License

Apache-2.0 — COOLJAPAN OU (Team Kitasan)
