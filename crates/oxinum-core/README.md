# oxinum-core — Core traits and types for OxiNum

[![Crates.io](https://img.shields.io/crates/v/oxinum-core.svg)](https://crates.io/crates/oxinum-core)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`oxinum-core` defines the foundational vocabulary shared by every crate in the OxiNum arbitrary-precision numeric tower. It contains the error types, rounding modes, and the numeric trait hierarchy that `oxinum-int` (big integers), `oxinum-float` (arbitrary-precision floats/decimals), and `oxinum-rational` (exact rationals) all build on, plus the top-level `oxinum` façade.

This crate carries no arithmetic logic of its own. It is a thin, dependency-light layer (`dashu-base` only, plus an optional `serde` feature) and is `#![forbid(unsafe_code)]`. It also re-exports the most useful `dashu-base` operation traits so that downstream crates obtain `Gcd`, `SquareRoot`, `DivRem`, and friends from a single place. Everything here is **100% Pure Rust** — no GMP, MPFR, or other C/C++/Fortran dependencies.

## Installation

```toml
[dependencies]
oxinum-core = "0.1.0"

# With serde derives for the error/rounding types:
oxinum-core = { version = "0.1.0", features = ["serde"] }
```

## Quick Start

```rust
use oxinum_core::{OxiNumError, OxiNumResult, RoundingMode};

fn checked_divide(a: i64, b: i64) -> OxiNumResult<i64> {
    if b == 0 {
        return Err(OxiNumError::DivByZero);
    }
    Ok(a / b)
}

let err = checked_divide(1, 0).unwrap_err();
assert_eq!(err.to_string(), "division by zero");

// Attach context to a message-bearing error variant.
let e = OxiNumError::Parse("bad digit".into()).context("while reading row 4");
assert!(e.to_string().contains("while reading row 4:"));

// A shared rounding vocabulary, independent of any backend library.
assert_eq!(RoundingMode::HalfEven.to_string(), "HalfEven");
```

## API Overview

### Error types

| Type | Description |
|------|-------------|
| `OxiNumError` | The unified, `#[non_exhaustive]` error enum for all OxiNum operations. Implements `Display`, `std::error::Error`, and `From<OxiNumError> for std::io::Error`. |
| `OxiNumResult<T>` | Convenience alias for `Result<T, OxiNumError>`. |
| `ParseNumberError` | Standalone rich parse diagnostic carrying a `message` plus 1-based `line` / `column`. Implements `Display`, `Error`, and `From<ParseNumberError> for OxiNumError`. |

`OxiNumError::context(self, ctx)` prepends `ctx` to the message of the message-bearing variants (`Parse`, `Precision`, `Overflow`, `Domain`) and returns the kind-only variants (`DivByZero`, `InvalidRadix`) unchanged. The error type is deliberately kept small (≤ 32 bytes); positional parse details live in the separate `ParseNumberError` so the `OxiNumError` size envelope is preserved.

#### `OxiNumError` variants

| Variant | Description |
|---------|-------------|
| `Parse(Cow<'static, str>)` | Failed to parse a number from a string. |
| `Precision(Cow<'static, str>)` | A precision constraint was violated. |
| `DivByZero` | Division by zero. |
| `Overflow(Cow<'static, str>)` | Arithmetic overflow (e.g. a result exceeds a primitive range). |
| `InvalidRadix(u32)` | Radix outside the supported `2..=36` range. |
| `Domain(Cow<'static, str>)` | Input outside a function's domain (e.g. `sqrt` of a negative). |

### `RoundingMode` enum

A backend-independent set of rounding modes giving a common vocabulary for precision control across all OxiNum numeric types. Derives `Debug, Clone, Copy, PartialEq, Eq, Hash` and implements `Display`.

| Variant | Behaviour |
|---------|-----------|
| `Up` | Round toward positive infinity. |
| `Down` | Round toward negative infinity. |
| `Ceiling` | Round toward positive infinity (alias for `Up` in unsigned contexts). |
| `Floor` | Round toward negative infinity (alias for `Down` in unsigned contexts). |
| `HalfUp` | Round half toward positive infinity. |
| `HalfDown` | Round half toward negative infinity. |
| `HalfEven` | Round half to the nearest even digit (banker's rounding). |
| `Unnecessary` | Exact result required — error if rounding would occur. |

### Numeric trait hierarchy

| Trait | Required items | Description |
|-------|----------------|-------------|
| `OxiNum` | `is_zero`, `is_one` | Marker trait for all OxiNum numeric types. Supertraits: `Display + Debug + Clone + PartialEq`. |
| `OxiSigned: OxiNum` | `signum`, `abs` (+ default `is_negative`, `is_positive`) | Numeric types that carry a sign. `signum` returns a `Sign`. |
| `OxiUnsigned: OxiNum` | — | Marker for unsigned numeric types. |

### Conversion traits

| Trait | Method | Description |
|-------|--------|-------------|
| `FromRadix` | `from_radix(src, radix) -> OxiNumResult<Self>` | Parse from an arbitrary-radix (`2..=36`) string. |
| `ToRadix` | `to_radix(&self, radix) -> OxiNumResult<String>` | Format as a string in an arbitrary radix (`2..=36`). |

### Power / root / modular / primality traits

| Trait | Methods | Description |
|-------|---------|-------------|
| `Pow<Exp>` | `type Output; pow(&self, exp) -> Output` | Exponentiation. |
| `Roots` | `sqrt`, `cbrt`, `nth_root(n)` | Integer (floor) root extraction. |
| `ModularArithmetic` | `mod_add`, `mod_sub`, `mod_mul`, `mod_pow` | Modular arithmetic; `mod_pow` uses binary exponentiation. |
| `Primality` | `is_probably_prime(witnesses)`, `next_prime` | Probabilistic primality (Miller–Rabin) and prime succession. |

### Re-exports from `dashu-base`

`oxinum-core` re-exports the `Sign` type and a curated set of operation traits so downstream crates can import them from one location:

```rust
pub use dashu_base::Sign; // Sign::Positive / Sign::Negative
pub use dashu_base::{
    Abs, AbsOrd, BitTest, CubicRoot, DivEuclid, DivRem, DivRemAssign, DivRemEuclid,
    EstimatedLog2, ExtendedGcd, Gcd, Inverse, PowerOfTwo, RemEuclid, Signed,
    SquareRoot, UnsignedAbs,
};
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | off | Derives `serde::Serialize` / `Deserialize` for `OxiNumError`, `ParseNumberError`, and `RoundingMode`. |

## Cross-references

`oxinum-core` is the base of the OxiNum tower:

- [`oxinum-int`](https://crates.io/crates/oxinum-int) — arbitrary-precision integers (`BigInt` / `BigUint`).
- [`oxinum-float`](https://crates.io/crates/oxinum-float) — arbitrary-precision floats / decimals (`BigFloat`).
- [`oxinum-rational`](https://crates.io/crates/oxinum-rational) — exact rationals (`BigRational`).
- [`oxinum`](https://crates.io/crates/oxinum) — the top-level façade that re-exports all of the above.

## License

Apache-2.0 — COOLJAPAN OU (Team Kitasan)
