# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- 1282 tests passing, zero clippy warnings, zero rustdoc warnings.
