# oxinum-float — Arbitrary-precision floats for OxiNum

[![Crates.io](https://img.shields.io/crates/v/oxinum-float.svg)](https://crates.io/crates/oxinum-float)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`oxinum-float` is the floating-point layer of the OxiNum arbitrary-precision numeric tower. It wraps `dashu-float`'s `FBig` / `DBig` and exposes the decimal big-float `BigFloat` (= `DBig`) together with precision-aware elementary functions — `exp`, `ln`, `sqrt`, `pow` — trigonometric functions (`sin`, `cos`, `tan`, `atan`, `atan2`), hyperbolic functions (`sinh`, `cosh`, `tanh`), and high-precision constants (`pi`, `e`, `ln 2`). Every transcendental takes an explicit decimal-`precision` argument and returns an `OxiNumResult`, surfacing precision and domain problems as recoverable errors rather than panics.

A parallel ground-up, **Pure Rust** binary-base float lives in the [`native`](#native-module) module — a `b = 2` arbitrary-precision `BigFloat` with explicit precision tracking and post-operation rounding, built on `oxinum-int`'s native limb-vector integer. The crate is `#![forbid(unsafe_code)]` and has no GMP/MPFR or other C/C++/Fortran dependency.

> **Note on `Hash`:** `DBig` deliberately does **not** implement `Hash`, mirroring the standard library's treatment of `f32`/`f64`. Distinct representations can compare equal (`1.0` vs `1.00`, `+0` vs `-0`) and `NaN != NaN`, so a well-defined hash requires a canonicalised newtype chosen by the caller.

## Installation

```toml
[dependencies]
oxinum-float = "0.1.0"

# Optional capabilities:
oxinum-float = { version = "0.1.0", features = ["serde", "num-traits"] }
```

## Quick Start

```rust
use oxinum_float::{exp, ln, sqrt, sin, compute_pi, DBig};
use std::str::FromStr;

// sqrt(2) to 30 significant digits.
let two = DBig::from_str("2.0")?;
let root2 = sqrt(&two, 30)?;
assert!(root2.to_string().starts_with("1.4142135"));

// e^1 to 30 digits.
let one = DBig::from_str("1.0")?;
assert!(exp(&one, 30)?.to_string().starts_with("2.71828"));

// High-precision pi.
let pi = compute_pi(50);
assert!(pi.to_string().starts_with("3.14159265358979"));

// Transcendentals return Results for domain/precision errors.
assert!(ln(&DBig::from_str("-1.0")?, 30).is_err()); // ln of a negative
# Ok::<(), oxinum_float::OxiNumError>(())
```

## API Overview

### Type aliases & core re-exports

| Item | Description |
|------|-------------|
| `BigFloat` | Alias for `dashu_float::DBig` — decimal (`base 10`) arbitrary-precision float. |
| `DBig`, `FBig` | The underlying `dashu-float` types, re-exported. |
| `Context` | `dashu_float::Context` — precision + rounding-mode bundle, re-exported. |
| `OxiNumError`, `OxiNumResult`, `RoundingMode` | Re-exported from `oxinum-core`. |

### Elementary functions

Each takes a `&DBig` (and a second operand where relevant) plus a `precision: usize` of significant decimal digits, returning `OxiNumResult<DBig>`. `precision == 0` yields `OxiNumError::Precision`.

| Function | Signature | Notes |
|----------|-----------|-------|
| `exp` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | `e^x`. |
| `ln` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | Natural log; errors if `x <= 0`. |
| `sqrt` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | Square root; errors if `x < 0`. |
| `pow` | `fn(&DBig, &DBig, usize) -> OxiNumResult<DBig>` | `base^exp` via `e^(exp·ln base)`; errors if `base <= 0` (with `exp == 0` short-circuiting to 1). |

### Trigonometric & hyperbolic functions

| Function | Signature | Notes |
|----------|-----------|-------|
| `sin` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | Taylor series with argument reduction modulo `2π`. |
| `cos` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | Taylor series with argument reduction. |
| `tan` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | `sin/cos`; errors with `DivByZero` when `cos(x) == 0`. |
| `atan` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | Range-reduced Taylor series / half-angle / reciprocal identity. |
| `atan2` | `fn(&DBig, &DBig, usize) -> OxiNumResult<DBig>` | Quadrant-aware `atan2(y, x)` in `(-π, π]`. |
| `sinh` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | `(e^x − e^−x)/2`. |
| `cosh` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | `(e^x + e^−x)/2`. |
| `tanh` | `fn(&DBig, usize) -> OxiNumResult<DBig>` | `sinh/cosh`. |

### Constants

Pre-computed to 200 decimal digits and parsed/truncated to the requested precision (clamped to `1..=200`).

| Function | Signature | Value |
|----------|-----------|-------|
| `compute_pi` | `fn(usize) -> DBig` | π. |
| `compute_e` | `fn(usize) -> DBig` | Euler's number *e*. |
| `compute_ln2` | `fn(usize) -> DBig` | ln 2. |

### `precision` module

Ergonomic free-function wrappers over `dashu-float`'s precision primitives, all operating on `DBig`:

| Function | Signature | Description |
|----------|-----------|-------------|
| `precision::with_precision` | `fn(&DBig, usize) -> DBig` | Rebind a value to a new working precision (discards the inexact flag). `precision == 0` means *unlimited*. |
| `precision::epsilon` | `fn(usize) -> OxiNumResult<DBig>` | Smallest positive unit `10^(1 − precision)` at the requested precision. Errors when `precision == 0`. |
| `precision::ulp` | `fn(&DBig) -> OxiNumResult<DBig>` | Unit in the last place of `x` at its carried precision. Errors when `x` has unlimited precision. |

### `round` module

```rust
pub use dashu_float::round::mode::{Down, HalfAway, HalfEven, Up, Zero};
```

Re-exports the `dashu-float` rounding-mode marker types so callers can select a mode (e.g. for `Context`) without a direct `dashu-float` dependency.

### `native` module

A ground-up Pure Rust binary-base (`b = 2`) float, intentionally **not** re-exported at the crate root (to avoid clashing with the decimal `DBig` alias). Access it as `oxinum_float::native::*`.

| Item | Kind | Description |
|------|------|-------------|
| `BigFloat` | struct | Binary arbitrary-precision float with explicit precision tracking, post-op rounding, `total_cmp`, NaN/inf handling, and `Add`/`Sub`/`Mul`/`Div`/`Neg`. |
| `FloatClass` | enum | `Finite` / `Infinite` / `Nan`. |
| `RoundingMode` | enum | Native rounding-mode selector. |
| `FloatContext` | struct | Precision + rounding context for native operations. |
| `pi`, `e_const`, `ln2` | fn | Native high-precision constants (via binary splitting / AGM). |
| `binary_splitting`, `nonfinite` | module | Constant-evaluation and non-finite helper submodules. |
| `ParseBigFloatError` | struct | *(feature `num-traits`)* parse-error type. |

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | off | `serde::Serialize` / `Deserialize` for `DBig` (enables `dashu-float/serde` and `oxinum-int/serde`). |
| `num-traits` | off | `num-traits` integration for the native float type. |

## Cross-references

- [`oxinum-core`](https://crates.io/crates/oxinum-core) — shared traits, errors, and `RoundingMode`.
- [`oxinum-int`](https://crates.io/crates/oxinum-int) — arbitrary-precision integers (the native float's foundation).
- [`oxinum-rational`](https://crates.io/crates/oxinum-rational) — exact rationals.
- [`oxinum`](https://crates.io/crates/oxinum) — the façade re-exporting all elementary/trig functions and constants.

## License

Apache-2.0 — COOLJAPAN OU (Team Kitasan)
