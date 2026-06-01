# oxinum-rational — Exact rational numbers for OxiNum

[![Crates.io](https://img.shields.io/crates/v/oxinum-rational.svg)](https://crates.io/crates/oxinum-rational)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`oxinum-rational` is the exact-rational layer of the OxiNum arbitrary-precision numeric tower. It re-exports `dashu-ratio`'s `RBig` (always-reduced) and `Relaxed` (lazily-reduced) rationals as `BigRational`, and layers on a rich set of rational utilities: exact `f32`/`f64` conversion, mixed-number parsing, continued-fraction expansion and reconstruction, best rational approximation, Stern–Brocot tree encoding, Farey-sequence generation, mediants, decimal-string rendering, and floor/ceil/round/truncate helpers.

A ground-up, **Pure Rust** native rational lives in the [`native`](#native-module) module — a `BigRational` always kept in lowest terms with a strictly positive denominator, built on `oxinum-int`'s native `BigInt` / `BigUint`. The crate is `#![forbid(unsafe_code)]` (bit-level float decoding uses only safe primitives) and free of GMP/MPFR or other C/C++/Fortran dependencies.

## Installation

```toml
[dependencies]
oxinum-rational = "0.1.0"

# Optional capabilities:
oxinum-rational = { version = "0.1.0", features = ["serde", "num-traits"] }
```

## Quick Start

```rust
use oxinum_rational::{
    continued_fraction, best_rational_approximation, from_f64, to_decimal_string,
    RBig, IBig, UBig,
};

// Exact fraction arithmetic — auto-reduced.
let half = RBig::from_parts(IBig::from(1), UBig::from(2u32));
let third = RBig::from_parts(IBig::from(1), UBig::from(3u32));
assert_eq!((half + third).to_string(), "5/6");

// Continued fraction of 355/113 (a famous pi approximation) is [3; 7, 16].
let pi_approx = RBig::from_parts(IBig::from(355), UBig::from(113u32));
assert_eq!(continued_fraction(&pi_approx),
           vec![IBig::from(3), IBig::from(7), IBig::from(16)]);

// Best approximation with a bounded denominator: 22/7.
let approx = best_rational_approximation(&pi_approx, &UBig::from(100u32));
assert_eq!(approx.to_string(), "22/7");

// f64 -> exact rational (lossless for finite floats).
assert_eq!(from_f64(0.25)?, RBig::from_parts(IBig::from(1), UBig::from(4u32)));

// Decimal rendering (truncated).
let t = RBig::from_parts(IBig::from(1), UBig::from(3u32));
assert_eq!(to_decimal_string(&t, 6), "0.333333");
# Ok::<(), oxinum_rational::OxiNumError>(())
```

## API Overview

### Type aliases & core re-exports

| Item | Description |
|------|-------------|
| `BigRational` | Alias for `dashu_ratio::RBig` — exact rational, always in lowest terms. |
| `RBig`, `Relaxed` | The `dashu-ratio` types, re-exported (`Relaxed` defers canonicalisation). |
| `IBig`, `UBig` | Re-exported from `dashu-int` for constructing rationals via `RBig::from_parts`. |
| `OxiNumError`, `OxiNumResult` | Re-exported from `oxinum-core`. |

### Float ↔ rational conversion

| Function | Signature | Description |
|----------|-----------|-------------|
| `from_f64` | `fn(f64) -> OxiNumResult<RBig>` | Exact `f64` → rational (finite floats are dyadic and lossless); errors on NaN/∞. |
| `from_f32` | `fn(f32) -> OxiNumResult<RBig>` | Exact `f32` → rational; errors on NaN/∞. |
| `to_f64` | `fn(&RBig) -> f64` | Nearest `f64` (round-to-nearest, ties-to-even); ±∞ on overflow. |
| `to_f64_exact` | `fn(&RBig) -> OxiNumResult<f64>` | Requires exact representability; `Overflow` out of range, `Precision` if rounding would occur. |

### Mixed numbers

| Item | Signature / Kind | Description |
|------|------------------|-------------|
| `parse_mixed` | `fn(&str) -> OxiNumResult<RBig>` | Parse `"3"`, `"3/4"`, `"-3/4"`, `"1 3/4"`, `"-2 1/3"` (sign binds to the whole value). |
| `MixedNumber` | `struct(pub RBig)` | Newtype implementing `FromStr` (mixed-number syntax) and `Display` (renders improper fractions as mixed numbers). |

### Continued fractions & approximation

| Function | Signature | Description |
|----------|-----------|-------------|
| `continued_fraction` | `fn(&RBig) -> Vec<IBig>` | Finite CF coefficients `[a0; a1, a2, …]`. |
| `from_continued_fraction` | `fn(&[IBig]) -> OxiNumResult<RBig>` | Inverse of `continued_fraction`; errors on an empty slice or a zero intermediate. |
| `best_rational_approximation` | `fn(&RBig, &UBig) -> RBig` | Closest rational with denominator ≤ `max_denom`, via CF convergents. |

### Enumeration: Stern–Brocot & Farey

| Function | Signature | Description |
|----------|-----------|-------------|
| `stern_brocot_path` | `fn(&RBig) -> OxiNumResult<Vec<bool>>` | Path encoding (`false` = left, `true` = right) for a positive rational; errors when `x <= 0`. |
| `from_stern_brocot_path` | `fn(&[bool]) -> RBig` | Reconstruct a positive rational from its path (empty path → `1/1`). |
| `farey_sequence` | `fn(u64) -> Vec<RBig>` | Order-`n` Farey sequence `F_n` in `[0, 1]`, strictly ascending from `0/1` to `1/1` (empty for `n == 0`). |

### Rational operations & helpers

| Function | Signature | Description |
|----------|-----------|-------------|
| `mediant` | `fn(&RBig, &RBig) -> RBig` | `(a_num + b_num)/(a_den + b_den)`. |
| `mixed_number` | `fn(&RBig) -> (IBig, RBig)` | Decompose into `(whole, fractional)` (toward zero, same sign). |
| `to_decimal_string` | `fn(&RBig, usize) -> String` | Truncated decimal string with `n` fractional digits. |
| `rational_floor` / `rational_ceil` / `rational_round` / `rational_truncate` | `fn(&RBig) -> IBig` | Floor / ceil / nearest (ties away) / toward-zero. |
| `rational_from_integer` | `fn(&IBig) -> RBig` | Construct `n/1`. |
| `rational_to_integer` | `fn(&RBig) -> Option<IBig>` | `Some(num)` iff the value is integer. |
| `rational_is_integer` | `fn(&RBig) -> bool` | True iff the reduced denominator is one. |
| `rational_abs` | `fn(&RBig) -> RBig` | Absolute value. |
| `rational_signum` | `fn(&RBig) -> IBig` | Sign as −1 / 0 / +1. |
| `rational_reciprocal` | `fn(&RBig) -> OxiNumResult<RBig>` | `1/x`; errors with `DivByZero` if `x == 0`. |
| `rational_pow` | `fn(&RBig, i32) -> OxiNumResult<RBig>` | Integer power; negative exponents take the reciprocal first. |

### `native` module

A ground-up Pure Rust rational, intentionally **not** re-exported at the crate root (to avoid clashing with the `RBig` alias). Access it as `oxinum_rational::native::*`.

| Item | Kind | Description |
|------|------|-------------|
| `BigRational` | struct | Always-reduced rational with `den > 0`; uniform `Eq`/`Hash`/`Ord`. Built on `oxinum_int::native`. |
| `continued_fraction` | module | Native continued-fraction support. |
| `float_to_rational`, `rational_to_float` | fn | Native float ↔ rational conversion. |
| `ParseBigRationalError` | struct | *(feature `num-traits`)* parse-error type. |

The native constructor reports a zero denominator as `OxiNumError::DivByZero`.

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | off | `serde` derives for `RBig` / `MixedNumber` (enables `dashu-ratio/serde` and `oxinum-int/serde`). |
| `num-traits` | off | `num-traits` integration for the native rational type. |

## Cross-references

- [`oxinum-core`](https://crates.io/crates/oxinum-core) — shared traits, errors, and `RoundingMode`.
- [`oxinum-int`](https://crates.io/crates/oxinum-int) — arbitrary-precision integers (numerator/denominator backbone).
- [`oxinum-float`](https://crates.io/crates/oxinum-float) — arbitrary-precision floats / decimals.
- [`oxinum`](https://crates.io/crates/oxinum) — the façade re-exporting the rational utilities and types.

## License

Apache-2.0 — COOLJAPAN OU (Team Kitasan)
