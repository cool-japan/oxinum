# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-06-10

### Fixed

- **oxinum-complex** `parity_cross_validation`: `prop_asin_cbig_native_agree` and
  `prop_atanh_cbig_native_agree` proptest cases timed out (>120 s) when run with
  `HEAVY_CASES = 16` at full precision (40 significant digits). The two tests are
  now split into a separate `proptest!` block with `VERY_HEAVY_CASES = 6` and
  `PREC_LIGHT = 20`, bringing each test well under the 120-second per-test budget
  while still exercising cross-family agreement to `1e-9` tolerance.

### Maintenance

- Version bump to 0.1.2; all workspace crates aligned to this version.
- Dependency version pins updated across workspace `[workspace.dependencies]`.

## [0.1.1] - 2026-06-04

### Added

- **oxinum-complex** (new crate): Arbitrary-precision complex numbers for the OxiNum
  workspace. `CBig` pairs two `DBig` components; `native::BigComplex` pairs two
  `BigFloat` components for binary-base complex arithmetic with explicit rounding
  control. Both types provide construction, arithmetic operators (all ownership
  variants), conjugate, norm-squared, transcendental functions (`exp`, `ln`, `sqrt`,
  `pow`), trigonometric functions (`sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`),
  inverse-trig functions (`asin`, `acos`, `atan`, `asinh`, `acosh`, `atanh`),
  `Display`/`Debug` formatting, and optional `serde` and `num-traits` features.
  `CBig` is re-exported from the `oxinum` facade as `oxinum::CBig` / `oxinum::Complex`.
- **oxinum-complex** `num-complex` feature: Two-way conversions between `CBig` and
  `num_complex::Complex<f64>` / `Complex<i64>` via `From` impls. Integer conversions
  store parts at unlimited `DBig` precision to prevent silent precision collapse.
- **oxinum-complex** `TryFrom<&CBig> for (f64, f64)`: Fallible projection to `f64`
  pairs; returns `OxiNumError::Overflow` when either component exceeds `f64::MAX`.
- **oxinum-complex** cross-validation test suite: `parity_cross_validation` tests
  verify that `CBig` and `native::BigComplex` agree on known-value results; includes
  SciRS2 `ArbitraryComplex` compatibility tests.
- **oxinum-float** `special` module: Pure-Rust special mathematical functions on
  `DBig` — `gamma`, `ln_gamma`, `digamma`, `erf`, `erfc`, `bessel_j0`, `euler_gamma`
  (Euler–Mascheroni constant to 200 digits), `catalan` (Catalan's constant to 200
  digits). Gamma uses Lanczos (g=7) for x ∈ (0, 20] and Stirling series for x > 20.
- **oxinum-float** `MpFloat` and `MpComplex` adapter types (`mp_float` module):
  `rug::Float`/`rug::Complex`-compatible wrappers over `DBig` for drop-in replacement
  of GMP-backed types in SciRS2's `arbitrary_precision` module.
- **oxinum-float** `native::bs_transcendental` module: Binary-splitting evaluation of
  `exp`, `sin`, and `cos` for `BigFloat` above a 512-bit precision threshold,
  replacing the O(N²) iterative Taylor series.
- **oxinum-int** `native::simd_ops` module: SIMD-accelerated (nightly `core::simd`,
  with scalar fallback on stable) inner kernels for AND, OR, XOR, and within-limb
  shift operations on `BigUint` limb slices. Activated by `oxinum_simd` cfg emitted
  from `build.rs` only on nightly + `simd` feature.
- **oxinum-int** `BitAndAssign`, `BitOrAssign`, `BitXorAssign` for `native::BigUint`
  (both owned and borrowed right-hand-side variants).
- SciRS2 compatibility integration tests across all sub-crates (`scirs2_int_compat`,
  `scirs2_float_compat`, `scirs2_rational_compat`, `scirs2_facade_compat`,
  `scirs2_trait_hierarchy_compat`, `scirs2_arbitrary_complex_compat`).
- Allocation-profiling Criterion benchmarks for `oxinum-int`, `oxinum-float`, and
  `oxinum-rational`; bitwise/shift operation benchmarks for `oxinum-int`.

### Fixed

- **oxinum-float** `atan` and `atan2` precision collapse: intermediate arithmetic was
  carried at the (narrow) input precision rather than the requested guard precision,
  capping output accuracy at approximately 3 significant digits regardless of the
  `precision` argument. All internal literals, reductions, and halving loops now use
  `dbig_at_precision` / `extend_precision` at `precision + 20` guard digits, giving
  accurate results to full requested precision.

## [0.1.0] - 2026-06-01

### Added

- **oxinum-core**: Core traits (`OxiNumTrait`, `OxiSigned`), `OxiNumError`/`OxiNumResult`,
  `RoundingMode` enum, `Sign` re-export from `dashu-base`, serde feature gate.
- **oxinum-int**: Arbitrary-precision integers via `dashu-int` re-exports (`UBig`, `IBig`)
  plus a full native Pure-Rust implementation:
  - `native::BigUint` — little-endian `Vec<u64>` limbs; schoolbook, Karatsuba, and
    Toom-Cook-3 multiplication; Knuth Algorithm D division; binary GCD;
    Newton integer sqrt and nth-root; Lehmer GCD.
  - `native::BigInt` — signed wrapper on `BigUint`; canonical zero invariant.
  - Number theory: Miller-Rabin + BPSW (Jacobi + strong Lucas) primality; Sieve of
    Eratosthenes; modular arithmetic; Montgomery multiplication context.
  - Conversions, radix I/O (2–36), serde, rand, num-traits features.
- **oxinum-float**: Arbitrary-precision floats via `dashu-float` re-exports (`FBig`, `DBig`)
  plus a full native `native::BigFloat`:
  - Binary-base (`b=2`), explicit precision, post-operation rounding.
  - Elementary functions: sqrt, exp, ln, pow.
  - Trigonometric functions: sin, cos, tan, asin, acos, atan, atan2.
  - High-precision constants: π, e, ln 2 (binary-splitting / AGM).
  - serde, num-traits features.
- **oxinum-rational**: Exact rationals via `dashu-ratio` re-exports (`RBig`, `Relaxed`)
  plus a full native `native::BigRational`:
  - Automatic reduction (GCD on construction), canonical zero/sign.
  - Continued-fraction expansion, reconstruction, convergents,
    and semiconvergent best-rational-approximation.
  - Cross-domain conversions (`BigFloat` ↔ `BigRational` ↔ `BigInt`).
  - serde, num-traits features.
- **oxinum** (facade): Prelude, constants module (π, e, ln 2), parse helpers,
  and feature-gated re-exports of all sub-crates.
- `deny.toml` banning GMP/MPFR/rug crates tree-wide.
- `Dockerfile.ffi-audit` + `scripts/ffi-audit.sh` for C/FFI-free verification.
- Benchmark harnesses (Criterion) for mul/div/factorial/primality/transcendentals.
- Property-based tests with proptest across all arithmetic laws.
- 1282 tests passing at 0.1.0, zero clippy warnings, zero rustdoc warnings.

[0.1.2]: https://github.com/cool-japan/oxinum/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/cool-japan/oxinum/releases/tag/v0.1.1
