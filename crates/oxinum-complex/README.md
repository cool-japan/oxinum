# oxinum-complex — Arbitrary-precision complex numbers for OxiNum

[![Crates.io](https://img.shields.io/crates/v/oxinum-complex.svg)](https://crates.io/crates/oxinum-complex)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`oxinum-complex` is the complex-number layer of the OxiNum arbitrary-precision numeric tower. It provides [`CBig`], a complex value whose real and imaginary parts are each a [`DBig`](https://crates.io/crates/oxinum-float) (decimal arbitrary-precision float re-exported from `oxinum-float`), together with precision-aware transcendental functions — `abs`, `arg`, `exp`, `ln`, `sqrt`, `pow` — and the full complex trigonometric / hyperbolic family — `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`. Every transcendental takes an explicit decimal-`precision` argument and returns an `OxiNumResult`, surfacing domain and precision problems as recoverable errors rather than panics. The principal branches match IEEE-754 / `num-complex` conventions (`sqrt(-1) = +i`, `ln` imaginary part in `(-π, π]`, `0^0 = 1`).

A parallel ground-up, **Pure Rust** binary-base complex lives in the [`native`](#native-module) module — a [`BigComplex`] built directly on `oxinum-float`'s native binary `BigFloat` with explicit rounding-mode control. The crate is `#![forbid(unsafe_code)]` and free of GMP/MPFR or other C/C++/Fortran dependencies.

> **Note on `Hash`, `Eq`, and `Ord`.** The complex field has no order compatible with its ring structure, so no `Ord` / `PartialOrd` is provided — sort on a derived scalar (magnitude or the lexicographic `(re, im)` pair) instead. `CBig` is built from `DBig`, which deliberately omits `Hash`/`Eq` (distinct representations can compare equal across precisions, mirroring `f32`/`f64`), so `CBig` inherits the same constraints. A component-wise `PartialEq` is provided.

## Installation

```toml
[dependencies]
oxinum-complex = "0.1.0"

# Optional capabilities:
oxinum-complex = { version = "0.1.0", features = ["serde", "num-traits"] }
```

## Quick Start

```rust
use oxinum_complex::CBig;
use oxinum_float::compute_pi;

// |3 + 4i| = 5  (via the exact squared magnitude, then the real sqrt).
let z = CBig::from((3i64, 4i64));      // integer parts are exact (unlimited precision)
assert_eq!(z.norm_sqr().to_string(), "25");
let m = z.abs(30)?;
assert!(m.to_string().starts_with('5'));

// (1 + i)² = 2i, via the Mul operator.
let one_plus_i = CBig::from_f64(1.0, 1.0)?;
let sq = &one_plus_i * &one_plus_i;
assert_eq!(sq.re().to_string(), "0");
assert_eq!(sq.im().to_string(), "2");

// exp(iπ) ≈ −1  (Euler's identity); build π with oxinum-float.
let i_pi = CBig::from_parts(oxinum_complex::DBig::from(0u32), compute_pi(50));
let e = i_pi.exp(40)?;
let (re, im) = e.to_f64_parts();
assert!((re + 1.0).abs() < 1e-12 && im.abs() < 1e-12);

// sqrt(2i) = 1 + i  (principal branch).
let two_i = CBig::from((0i64, 2i64));
let r = two_i.sqrt(40)?;
let (rre, rim) = r.to_f64_parts();
assert!((rre - 1.0).abs() < 1e-12 && (rim - 1.0).abs() < 1e-12);

// Transcendentals return Results for domain errors.
assert!(CBig::zero().ln(30).is_err()); // ln(0) is undefined
# Ok::<(), oxinum_complex::OxiNumError>(())
```

## API Overview

### Type aliases & core re-exports

| Item | Description |
|------|-------------|
| `CBig` | Arbitrary-precision complex `re + im·i`, each component a decimal `DBig`. |
| `Complex` | Convenience alias for `CBig` (mirrors the `BigFloat = DBig` convention). |
| `DBig` | Re-exported from `oxinum-float` for constructing parts via `from_parts`. |
| `OxiNumError`, `OxiNumResult` | Re-exported from `oxinum-core`. |

### Constructors

| Item | Signature | Description |
|------|-----------|-------------|
| `new` / `from_parts` | `fn(DBig, DBig) -> CBig` | Build from explicit `(re, im)` parts (`new` is an alias). |
| `from_real` | `fn(DBig) -> CBig` | Real axis (`im = 0`). |
| `from_imag` | `fn(DBig) -> CBig` | Imaginary axis (`re = 0`). |
| `zero` / `one` / `i` | `fn() -> CBig` | The constants `0`, `1`, and the imaginary unit `i`. |
| `from_f64` | `fn(f64, f64) -> OxiNumResult<CBig>` | From a pair of `f64`s; errors (`Parse`) on `NaN`/`∞`. |
| `From<(DBig, DBig)>`, `From<DBig>`, `From<&DBig>` | trait | Build from `DBig` parts (precision preserved as supplied). |
| `From<(i64, i64)>`, `From<i64>` | trait | Build from integers — **exact**, at unlimited `DBig` precision. |

### Accessors & algebra (no precision argument)

| Item | Signature | Description |
|------|-----------|-------------|
| `re` / `im` | `fn(&self) -> &DBig` | Shared reference to the real / imaginary part. |
| `real` / `imag` | `fn(&self) -> DBig` | Owned clone of the real / imaginary part. |
| `into_parts` | `fn(self) -> (DBig, DBig)` | Decompose into the owned `(re, im)` pair. |
| `conj` | `fn(&self) -> CBig` | Complex conjugate `re − im·i`. |
| `norm_sqr` | `fn(&self) -> DBig` | Squared magnitude `re² + im²` (real, non-negative). |
| `is_zero` / `is_real` / `is_imaginary` | `fn(&self) -> bool` | Component predicates. |
| `to_f64_parts` | `fn(&self) -> (f64, f64)` | Lossy projection of both parts to `f64`. |

### Operators

`Add`, `Sub`, `Mul`, `Div`, and `Neg` are implemented for all four owned/borrowed combinations, with the matching `AddAssign` / `SubAssign` / `MulAssign` / `DivAssign`. `PartialEq` (component-wise), `Default` (= `zero`), and `Display` (`"a + bi"` / `"a - bi"`) are also provided.

| Item | Signature | Description |
|------|-----------|-------------|
| `Div` operator (`/`) | `CBig / CBig` | `(a · conj(b)) / |b|²`. **Panics** on a zero divisor, matching the other oxinum crates. |
| `checked_div` | `fn(&self, &CBig) -> OxiNumResult<CBig>` | No-panic division; returns `DivByZero` for a zero divisor. |

### Transcendental functions

Each takes a `precision: usize` of significant decimal digits and returns an `OxiNumResult`. Internals run at a guard precision (`precision + 10`) to absorb rounding noise.

| Function | Signature | Notes |
|----------|-----------|-------|
| `abs` | `fn(&self, usize) -> OxiNumResult<DBig>` | Magnitude `|z| = sqrt(a² + b²)` (real). |
| `arg` | `fn(&self, usize) -> OxiNumResult<DBig>` | Argument `atan2(b, a)`; principal value in `(−π, π]`. |
| `exp` | `fn(&self, usize) -> OxiNumResult<CBig>` | `exp(z) = eᵃ·(cos b + i·sin b)`. |
| `ln` | `fn(&self, usize) -> OxiNumResult<CBig>` | `½·ln(a² + b²) + i·atan2(b, a)`; `ln(0)` → `Domain` error. |
| `sqrt` | `fn(&self, usize) -> OxiNumResult<CBig>` | Principal branch (`re ≥ 0`, so `sqrt(-1) = +i`). |
| `pow` | `fn(&self, &CBig, usize) -> OxiNumResult<CBig>` | `z^w = exp(w · ln z)`; `0^0 = 1`, `0^w = 0` otherwise. |

### Trigonometric & hyperbolic functions

Each takes a `precision: usize` and returns `OxiNumResult<CBig>`, assembled component-wise from the real `sin`/`cos`/`sinh`/`cosh` of `oxinum-float`.

| Function | Signature | Notes |
|----------|-----------|-------|
| `sin` | `fn(&self, usize) -> OxiNumResult<CBig>` | `sin a · cosh b + i · cos a · sinh b`. |
| `cos` | `fn(&self, usize) -> OxiNumResult<CBig>` | `cos a · cosh b − i · sin a · sinh b`. |
| `tan` | `fn(&self, usize) -> OxiNumResult<CBig>` | `sin z / cos z` (via `checked_div`); `DivByZero` at the poles. |
| `sinh` | `fn(&self, usize) -> OxiNumResult<CBig>` | `sinh a · cos b + i · cosh a · sin b`. |
| `cosh` | `fn(&self, usize) -> OxiNumResult<CBig>` | `cosh a · cos b + i · sinh a · sin b`. |
| `tanh` | `fn(&self, usize) -> OxiNumResult<CBig>` | `sinh z / cosh z` (via `checked_div`); `DivByZero` at the poles. |

### `native` module

A ground-up Pure Rust binary-base (`b = 2`) complex, built on `oxinum_float::native::BigFloat`, intentionally **not** re-exported at the crate root (to avoid clashing with the decimal-backed `CBig`). Access it as `oxinum_complex::native::*`.

| Item | Kind | Description |
|------|------|-------------|
| `BigComplex` | struct | Binary `(re, im)` pair of native `BigFloat`s; same surface as `CBig`, but numeric methods take `(prec: u32, mode: RoundingMode)`. |
| `RoundingMode` | enum | Re-exported from `oxinum_float::native` for constructing / rounding `BigComplex` values. |

The numeric API mirrors `CBig` (`from_parts`/`from_real`/`from_imag`/`zero`/`one`/`i`/`from_f64`, `re`/`im`/`real`/`imag`/`into_parts`/`conj`/`norm_sqr`/`is_zero`/`to_f64_parts`, plus `abs`/`arg`/`exp`/`ln`/`sqrt`/`pow` and `sin`/`cos`/`tan`/`sinh`/`cosh`/`tanh`), with these differences:

- numeric methods take an explicit `(prec: u32, mode: RoundingMode)` pair;
- the hyperbolics are derived internally from `exp` (the native `BigFloat` exposes no hyperbolic primitives);
- division is exposed only as `checked_div(&self, &BigComplex, prec, mode) -> OxiNumResult<BigComplex>` — there is **no** `Div` operator and **no** `Default`, since both would need a baked-in precision / rounding mode. `Add`/`Sub`/`Mul`/`Neg` and the `*Assign` traits are provided as usual.

```rust
use oxinum_complex::native::{BigComplex, RoundingMode};

// sqrt(2i) = 1 + i at 80 bits, banker's rounding.
let two_i = BigComplex::from_f64(0.0, 2.0, 80)?;
let r = two_i.sqrt(80, RoundingMode::HalfEven)?;
assert!((r.re().to_f64() - 1.0).abs() < 1e-12);
assert!((r.im().to_f64() - 1.0).abs() < 1e-12);
# Ok::<(), oxinum_complex::OxiNumError>(())
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | off | `serde::Serialize` / `Deserialize` for `CBig` and `BigComplex` (enables `dashu-float/serde`, `oxinum-float/serde`, `oxinum-core/serde`). |
| `num-traits` | off | `num_traits::{Zero, One}` for both `CBig` and `BigComplex`. `Num`/`Signed`/`Float` are intentionally **not** implemented — the complex field is neither ordered nor signed. |

## Cross-references

- [`oxinum-core`](https://crates.io/crates/oxinum-core) — shared traits, errors, and `RoundingMode`.
- [`oxinum-float`](https://crates.io/crates/oxinum-float) — arbitrary-precision floats / decimals (the real-component backbone).
- [`oxinum-rational`](https://crates.io/crates/oxinum-rational) — exact rationals.
- [`oxinum`](https://crates.io/crates/oxinum) — the façade re-exporting the complex types.

## License

Apache-2.0 — COOLJAPAN OU (Team Kitasan)
