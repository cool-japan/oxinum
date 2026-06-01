# oxinum TODO (facade)

## Status
Facade crate re-exporting integer/float/rational types, number-theory and elementary functions, `Context`, rounding modes, and `OxiNumError` from sub-crates (feature-gated behind `pure`). Adds `prelude`, `constants`, `convert`, `parse` modules, top-level type aliases (`Int`/`Natural`/`Float`/`Rational`), and `version()`. ~190 SLOC + submodules, with cross-type and number-theory integration tests.

## Core Implementation
- [x] Re-export native BigInt/BigUint/BigFloat/BigRational once oxinum-int/float/rational implement them natively (50 SLOC) (planned 2026-05-28)
  - **Plan ID:** N5 — Facade native re-exports
  - **Goal:** Expose all four native types through the top-level `oxinum` facade in a `native::` namespace, so consumers can adopt native incrementally without breaking existing dashu-based code.
  - **Design:** `crates/oxinum/src/lib.rs` adds `pub mod native { pub use oxinum_int::native::{BigUint, BigInt, gcd, gcd_binary, gcd_int, divrem, divrem_int, KARATSUBA_THRESHOLD}; pub use oxinum_float::native::BigFloat; pub use oxinum_rational::native::BigRational; }` plus parallel top-level aliases in `pub mod native_aliases { pub type Int = super::native::BigInt; pub type Natural = super::native::BigUint; pub type Float = super::native::BigFloat; pub type Rational = super::native::BigRational; }`. Leave the dashu-backed names as default `oxinum::prelude::*`; document `oxinum::native::*` as the migration target. Short crate-level rustdoc paragraph on dual exposure.
  - **Files:** `crates/oxinum/src/lib.rs`.
  - **Prerequisites:** N3, N4a, N4b done.
  - **Tests:** smoke test `crates/oxinum/tests/native_facade.rs`: construct `oxinum::native::BigInt::from(42_i64)`, `oxinum::native::BigUint::from(7_u64)`, `oxinum::native::BigRational::from_integer(IBig::from(5))`, `oxinum::native::BigFloat::from_i64(3, 50)`. Verify all type aliases resolve.
  - **Risk:** Re-export ambiguity vs existing dashu re-exports. Mitigated by `oxinum::native::` namespace exclusively — no top-level shadowing.
- [x] Add `prelude` module gathering the most common types and traits for glob import
- [x] Add `constants` module with pi, e, ln2, sqrt2 at arbitrary precision
- [x] Add `convert` module with cross-type conversions: int-to-float, float-to-rational, rational-to-int, float-to-rational
- [x] Implement `oxinum::parse(s)` universal parser that auto-detects integer/float/rational format

## API Improvements
- [x] Remove `pure` feature gate once all sub-crates have native implementations (facade should work without feature flags)
- [x] Add top-level type aliases: `Int`, `Natural`, `Float`, `Rational`
- [x] Add `oxinum::version()` returning crate version string
- [x] Ensure all doc examples compile and run (41 doc tests pass)

## Testing
- [x] Integration tests exercising number theory and cross-type conversions
- [x] Integration test: compute pi via Machin's formula using BigFloat (planned 2026-05-28)
  - **Plan ID:** Item 5 — facade integration tests + worked examples
  - **Goal:** End-to-end demonstrations through the public facade API.
  - **Design:** integration test for pi via Machin's formula `π/4 = 4·atan(1/5) − atan(1/239)` using `oxinum::{atan, ...}`, assert against `oxinum::constants::pi(n)` to N digits; integration test for exact 3×3 rational determinant (Sarrus/Laplace over `RBig`), assert exact value. Worked examples (`crates/oxinum/examples/`): `high_precision_pi.rs` (Machin/constants, prints N-digit π) and `exact_rational_linear_solve.rs` (solve a small system exactly via Cramer's rule over `RBig`). Cargo auto-discovers `examples/*.rs`.
  - **Files:** `crates/oxinum/tests/machin_pi.rs`, `crates/oxinum/tests/rational_determinant.rs`, `crates/oxinum/examples/high_precision_pi.rs`, `crates/oxinum/examples/exact_rational_linear_solve.rs`.
  - **Prerequisites:** none beyond existing facade API.
  - **Tests:** the two integration tests; acceptance also runs `cargo build --examples -p oxinum`.
  - **Risk:** Machin precision/guard-digit tuning — assert to ~30 digits at guard precision 50.
- [x] Integration test: compute exact rational determinant of a 3x3 matrix (planned 2026-05-28; covered by Item 5)
- [x] Original arithmetic integration tests pass (15 tests, regression-checked)

## Performance
- [x] Benchmark end-to-end: 1000-digit pi computation time (planned 2026-05-29; BM1 — criterion bench in oxinum-float/benches, native vs dashu; 3322-bit ≈ 1000 decimal digits case added in BM2)
- [x] Benchmark BigInt factorial(1000) time (planned 2026-05-29; BM1 — criterion bench in oxinum-int/benches, native vs dashu; factorial.rs sweeps n ∈ [100, 1000, 5000])
- [x] Compare performance against num-bigint + num-rational for common operations

## Integration
- [ ] Ensure SciRS2 can depend solely on oxinum for all numeric needs
- [ ] Ensure OxiBLAS can use oxinum for arbitrary-precision matrix entries
- [ ] Verify oxinum works as drop-in for projects currently using dashu directly
