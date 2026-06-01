# oxinum-core TODO

## Status
Foundation layer: `OxiNumError` enum (Parse/Precision/DivByZero/Overflow/InvalidRadix, `#[non_exhaustive]`), `OxiNumResult<T>` alias, dashu-independent `RoundingMode` enum, numeric trait hierarchy (`OxiNum`/`OxiSigned`/`OxiUnsigned`), conversion traits (`FromRadix`/`ToRadix`), and `Pow`/`Roots`/`ModularArithmetic`/`Primality` trait definitions. ~290 SLOC production code, 13 tests.

## Core Implementation
- [x] Add `Overflow` error variant for arithmetic overflow detection
- [x] Add `InvalidRadix` error variant for base conversion errors
- [x] Define numeric trait hierarchy: `OxiNum`, `OxiSigned`, `OxiUnsigned` abstracting over integer/float/rational backends
- [x] Define `FromRadix` / `ToRadix` traits for arbitrary-base string conversion
- [x] Define `Pow` trait for exponentiation across all numeric types
- [x] Define `Roots` trait (sqrt, cbrt, nth_root) across numeric types
- [x] Re-export `Gcd` / `ExtendedGcd` traits (from dashu-base)
- [x] Define `ModularArithmetic` trait (mod_add, mod_sub, mod_mul, mod_pow)
- [x] Define `Primality` trait (is_probably_prime, next_prime)
- [x] Define `RoundingMode` enum (Up, Down, Ceiling, Floor, HalfUp, HalfDown, HalfEven, Unnecessary) independent of dashu
- [x] Define `ParseNumberError` with line/column info for better diagnostics (40 SLOC) (planned 2026-05-28)
  - **Plan ID:** Item 2 — oxinum-core error & diagnostics enrichment
  - **Goal:** Richer parse diagnostics, error context chaining, optional serde — all additive (no change to `OxiNumError` variants → `<= 32 bytes` size assertion stays green).
  - **Design:** `pub struct ParseNumberError { message: String, line: usize, column: usize }` standalone + `Display` + `pub fn new(...)` + `From<ParseNumberError> for OxiNumError` (→ `Parse` with `"{msg} (line {line}, col {column})"`); `OxiNumError::context(self, ctx: impl AsRef<str>) -> Self` **verbatim semantics:** preserve variant and prefix message for `Parse`/`Precision`/`Overflow` (`"{ctx}: {orig}"`); return `DivByZero`/`InvalidRadix` unchanged (Display already conveys the kind). serde: `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` on `OxiNumError` and `RoundingMode`; `serde` optional dep + `[features] serde = ["dep:serde"]`.
  - **Files:** `crates/oxinum-core/src/lib.rs` (or split `error.rs` if it nears 2000 lines), `crates/oxinum-core/Cargo.toml`.
  - **Prerequisites:** Item 0 (serde optional dep + proptest dev-dep wired).
  - **Tests:** construct `ParseNumberError` → convert to `OxiNumError` → Display contains line/col; `context()` prefixes message variants and leaves `DivByZero` unchanged; proptest Display round-trip; serde JSON round-trip (`--features serde`); existing `size_of::<OxiNumError>() <= 32` stays green.
  - **Risk:** Low (additive). `Cow<'static,str>` migration intentionally deferred (cross-crate construction-site churn).
- [x] Implement `From<OxiNumError>` for `std::io::Error`

## API Improvements
- [x] Add `#[non_exhaustive]` to `OxiNumError` for forward compatibility
- [x] Implement `serde::Serialize` / `serde::Deserialize` for `OxiNumError` behind `serde` feature flag (planned 2026-05-28; covered by Item 2; `RoundingMode` also gets serde)
- [x] Add `OxiNumError::context()` method for error chaining with caller context (planned 2026-05-28; covered by Item 2; preserves variant, annotates message-bearing variants only)
- [x] Add `OxiNumResult<T>` type alias for `Result<T, OxiNumError>`

## Testing
- [x] Property-based tests for error Display round-trip (error message preservation) (planned 2026-05-28; covered by Item 2; proptest)
- [x] Test all error variant conversions and From impls
- [x] Test trait object compatibility (`dyn std::error::Error`)
- [x] Ensure `Send + Sync` bounds on `OxiNumError`

## Performance
- [x] Ensure `OxiNumError` is small (measure with `std::mem::size_of`) -- asserted <= 32 bytes
- [x] Add `OxiNumError::Domain(String)` variant for out-of-domain errors + retrofit sqrt site (planned 2026-05-29)
  - **Plan ID:** C1 — `OxiNumError::Domain` variant
  - **Goal:** Add `OxiNumError::Domain(String)` for "input outside function's domain" errors. Retrofit current `Parse("sqrt of negative...")` in `oxinum-float/src/native/float_sqrt.rs` to use `Domain`. Pre-positions clean Domain returns for `ln(0)`, `ln(-1)`, and future complex-domain ops.
  - **Design:** `crates/oxinum-core/src/lib.rs`: add variant `Domain(String)` to `OxiNumError`, with `Display` returning `"Domain error: {msg}"`. Update the ≤32 bytes size assertion if needed — `String` is 24 bytes + tag, identical footprint to existing `Parse`/`Precision`/`Overflow`. Add `OxiNumError::domain<S: Into<String>>(msg: S) -> Self` helper. `context(self, ctx)` method extended to prepend context to `Domain` messages. Audit & retrofit: `oxinum-float/src/native/float_sqrt.rs` returns `Domain` (currently `Parse`).
  - **Files:** `crates/oxinum-core/src/lib.rs`, `crates/oxinum-float/src/native/float_sqrt.rs`.
  - **Prerequisites:** none.
  - **Tests:** `OxiNumError::domain("msg")` Display → `"Domain error: msg"`; `BigFloat::sqrt(neg)` returns `Err(OxiNumError::Domain(_))`; size assertion ≤32 bytes still holds; serde round-trip for `Domain` variant under `--features serde`.
  - **Risk:** Breaking change to `OxiNumError` — mitigated by `#[non_exhaustive]` already on enum (new variants don't break match-default consumers).
- [x] Use `Cow<'static, str>` instead of `String` in error variants to avoid allocation for static messages

## Integration
- [x] Ensure all numeric crates (oxinum-int, oxinum-float, oxinum-rational) use traits from oxinum-core
- [ ] Verify that trait hierarchy is compatible with SciRS2 numeric requirements
- [x] Ensure error types are compatible with thiserror derive in downstream crates (OxiNumError implements std::error::Error; verified by oxinumerror_implements_std_error test: assert_error<E: std::error::Error>() + Box<dyn std::error::Error>)
