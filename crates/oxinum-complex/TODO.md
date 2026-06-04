# oxinum-complex TODO

## Status
Complex-number layer of OxiNum, delivered for the 0.1.0 release. Provides `CBig` (decimal-backed complex, each part a `DBig` from `oxinum-float`) and a ground-up native binary `native::BigComplex` (built on `oxinum_float::native::BigFloat`). Both carry full algebra (`Add`/`Sub`/`Mul`/`Div`/`Neg` + `*Assign`, conjugate, squared magnitude), the transcendental set (`abs`, `arg`, `exp`, `ln`, `sqrt`, `pow`) and the complex trigonometric / hyperbolic family (`sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`) at configurable precision, plus `serde` and `num-traits` (`Zero`/`One`) integration. Principal branches follow IEEE-754 / `num-complex` conventions (`sqrt(-1) = +i`, `ln` imaginary part in `(−π, π]`, `0^0 = 1`). Pure Rust, `#![forbid(unsafe_code)]`, GMP/MPFR-free. ~173 tests (unit + integration + proptest).

## Decimal `CBig` (delivered)
- [x] `CBig` type: `(re, im)` pair of `DBig`; `new`/`from_parts`/`from_real`/`from_imag`/`zero`/`one`/`i`/`from_f64`
- [x] Accessors `re`/`im`/`real`/`imag`/`into_parts`/`conj`/`norm_sqr`/`is_zero`/`is_real`/`is_imaginary`/`to_f64_parts`
- [x] `From<(DBig, DBig)>`, `From<DBig>`, `From<&DBig>`, `From<(i64, i64)>`, `From<i64>` — integer conversions are **exact** (unlimited `DBig` precision, so `norm_sqr` / `pow` keep full precision)
- [x] `Add`/`Sub`/`Mul`/`Div`/`Neg` for all four owned/borrowed variants + `AddAssign`/`SubAssign`/`MulAssign`/`DivAssign`
- [x] `Div` operator panics on a zero divisor (workspace convention); `checked_div` returns `OxiNumError::DivByZero`
- [x] `PartialEq` (component-wise), `Default` (= `zero`), `Display` (`"a + bi"` / `"a - bi"`), `Debug`
- [x] `abs` / `arg` (`atan2`, principal value in `(−π, π]`)
- [x] `exp` = `eᵃ·(cos b + i·sin b)`; `ln` = `½·ln(a²+b²) + i·atan2(b,a)` (`ln(0)` → `Domain`)
- [x] `sqrt` principal branch (`re ≥ 0`, `sqrt(-1) = +i`, radicands clamped non-negative); validated `sqrt(2i) = 1+i`
- [x] `pow` = `exp(w · ln z)` with `0^0 = 1`, `0^w = 0`
- [x] `sin`/`cos`/`tan`/`sinh`/`cosh`/`tanh` assembled component-wise; `tan`/`tanh` via `checked_div` (`DivByZero` at poles)

## Native `BigComplex` (delivered)
- [x] `BigComplex` type: `(re, im)` pair of native binary `BigFloat`; full constructor / accessor surface mirroring `CBig`
- [x] `RoundingMode` re-exported at `oxinum_complex::native::RoundingMode`
- [x] `Add`/`Sub`/`Mul`/`Neg` (four variants) + `*Assign`; `PartialEq` component-wise
- [x] `checked_div(&self, &BigComplex, prec, mode)` — no `Div` operator and no `Default` (both need a baked-in precision/mode)
- [x] `abs`/`arg`/`exp`/`ln`/`sqrt`/`pow` taking `(prec: u32, mode: RoundingMode)`
- [x] `sin`/`cos`/`tan`/`sinh`/`cosh`/`tanh`; hyperbolics derived internally from `exp` (`sinh = (eˣ−e⁻ˣ)/2`, etc.)
- [x] `tanh` real-axis fast path (`b == 0`) routes through the scalar helper to avoid spurious imaginary rounding

## Feature integrations (delivered)
- [x] `serde::Serialize` / `Deserialize` for `CBig` and `BigComplex` (flat `(re, im)` repr) behind `serde`
- [x] `num_traits::{Zero, One}` for `CBig` and `BigComplex` behind `num-traits`; `Num`/`Signed`/`Float` deliberately omitted (complex is unordered / unsigned), documented rather than stubbed

## Testing (delivered)
- [x] Hand-computed algebra: `(1+2i)(3+4i) = −5+10i`, `i² = −1`, `(1+i)² = 2i`, division round-trips
- [x] `checked_div` by zero → `DivByZero`; `Div` operator by zero panics (`#[should_panic]`)
- [x] `norm_sqr` exactness for integer parts incl. `i64::MAX` (regression against precision collapse)
- [x] Euler's identity `exp(iπ) ≈ −1`; `ln(−1) = iπ`; `ln(0)` → `Domain`
- [x] `sqrt(−1) = +i`, `sqrt(2i) = 1+i`, `sqrt(4) = 2`; `pow(i, 2) = −1`, `0^0 = 1`, `0^w = 0`
- [x] `|3+4i| = 5`, `arg(i) = π/2`
- [x] Pythagorean identities `sin²z + cos²z ≈ 1`, `cosh²z − sinh²z ≈ 1`; `tan` matches a known reference value
- [x] `serde` JSON round-trip for `CBig` and `BigComplex`
- [x] proptest coverage (algebra / identities for both `CBig` and `BigComplex`)
- [x] Criterion benchmark (`complex_transcendental`) for the transcendental hot paths

## Future / possible work
- [x] Inverse complex trig: `asin` / `acos` / `atan` (and `asinh` / `acosh` / `atanh`) (planned 2026-06-02)
  - **Goal:** `CBig` and `native::BigComplex` each gain `asin, acos, atan, asinh, acosh, atanh`, principal-branch.
  - **Design:** Closed forms from existing `ln`/`sqrt`/ops: `asin z = -i·ln(iz+√(1−z²))`, `acos z = -i·ln(z+i·√(1−z²))`, `atan z = (i/2)·[ln(1−iz)−ln(1+iz)]`, `asinh z = ln(z+√(z²+1))`, `acosh z = ln(z+√(z−1)·√(z+1))`, `atanh z = ½·[ln(1+z)−ln(1−z)]`. Guard precision +10. New files `src/inverse_trig.rs` + `src/native/inverse_trig.rs`.
  - **Files:** `src/inverse_trig.rs`, `src/native/inverse_trig.rs`, `src/lib.rs`, `src/native/mod.rs`
  - **Tests:** `asin 1 = π/2`, `atan 1 = π/4`, `acosh 1 = 0`, `atanh 0 = 0`; round-trips `sin(asin z) ≈ z`; CBig-vs-BigComplex agreement.
  - **Risk:** Branch-cut sign errors → derive from principal `ln`/`sqrt`, test axis values.
- [x] Polar helpers: `from_polar(r, θ)` and `to_polar() -> (DBig, DBig)` (magnitude + argument) (planned 2026-06-02)
  - **Goal:** `from_polar(r, θ)` constructor and `to_polar()` on both families.
  - **Design:** `from_polar = r·(cos θ + i·sin θ)` via existing real `cos`/`sin`; `to_polar = (abs, arg)` reusing existing methods. In `src/transcendental.rs` + `src/native/transcendental.rs`.
  - **Files:** `src/transcendental.rs`, `src/native/transcendental.rs`
  - **Tests:** `from_polar(2, π/2) ≈ 2i`; `to_polar(3+4i) ≈ (5, atan2(4,3))`; round-trip.
  - **Risk:** `arg` principal range covered by existing tests.
- [x] Additional `num-complex`-style interop (e.g. an optional `num_complex::Complex<T>` bridge / `Complex` trait surface) (delivered 2026-06-02)
  - **Goal:** Optional `num-complex` feature bridging both families to `num_complex::Complex<T>`.
  - **Design:** Add `num-complex` to root `[workspace.dependencies]`; optional dep in `oxinum-complex/Cargo.toml`; feature `"num-complex"`. Gate `From<Complex<f64>>`, `From<Complex<i64>>`, `From<&CBig> for Complex<f64>` in `src/convert.rs` + `src/native/convert.rs`.
  - **Files:** root `Cargo.toml`, `oxinum-complex/Cargo.toml`, `src/convert.rs`, `src/native/convert.rs`
  - **Tests:** Gated round-trips; integer-exact norm_sqr; default-feature build green.
  - **Risk:** Must not leak into default features; `num-complex` is pure Rust.
- [x] Scalar mixed-mode ops (`CBig × DBig`, `CBig × i64`) without first wrapping the scalar (delivered 2026-06-02)
  - **Goal:** `Add/Sub/Mul/Div` between `CBig` and `DBig`/`i64` scalars; `Add/Sub/Mul` between `BigComplex` and `BigFloat`/`i64`.
  - **Design:** Macro-generate scalar variants using existing `impl_binop!`/`impl_assign!` pattern. `z±d=(re±d,im)`, `z·d=(re·d,im·d)`, `z/d=(re/d,im/d)`. `i64` via exact-DBig path. Native: no `Div` operator (by design).
  - **Files:** `src/ops.rs`, `src/native/complex_ops.rs`
  - **Tests:** `i() * DBig::from(3) == 3i`; `5i64 - z`; `*Assign`; native scalar mul/add; exactness.
  - **Risk:** Coherence overlaps → use distinct scalar type parameters.
- [x] More conversions (`From<num_complex::Complex64>`, `TryInto<(f64, f64)>` with overflow detection) (delivered 2026-06-02)
  - **Goal:** `TryFrom<&CBig> for (f64,f64)` and `TryFrom<&BigComplex> for (f64,f64)` with overflow detection; `From<Complex64>` gated on `num-complex`.
  - **Design:** `TryFrom` returns `Err` when any component is non-finite after `to_f64`. Use existing `OxiNumError` variant. `From<Complex64>` shares item 3's feature gate.
  - **Files:** `src/convert.rs`, `src/native/convert.rs`
  - **Tests:** Finite `Ok`; huge-magnitude `Err`; native non-finite component `Err`.
  - **Risk:** Error-variant choice → read `OxiNumError` enum first.
- [x] Native `BigComplex` `serde`/`num-traits` parity audit and expanded proptest coverage (more random magnitudes, cross-validation of `CBig` vs `BigComplex`) (planned 2026-06-02)
  - **Goal:** Confirm identical `serde`/`num-traits` surface; cross-validate `CBig` vs `BigComplex` under proptests.
  - **Design:** Audit `num_traits_impl.rs`/`native/num_traits_impl.rs`; close gaps. New `tests/parity_cross_validation.rs` with random-magnitude proptests comparing both families; serde JSON round-trips. Wave 2 (after items 1–5, 7).
  - **Files:** `tests/parity_cross_validation.rs`; `num_traits_impl.rs`/`native/num_traits_impl.rs` if gap found.
  - **Tests:** The proptests themselves; serde feature gating.
  - **Risk:** Decimal vs binary tolerance → compare via `to_f64_parts` with generous epsilons.
- [x] `powi` / `powf` integer / real fast paths avoiding the full `exp(w·ln z)` round trip (planned 2026-06-02)
  - **Goal:** `powi(n: i32)` via binary exponentiation; `powf(x: real)` via polar form `r^x·(cos(xθ)+i·sin(xθ))`.
  - **Design:** `powi`: exponentiation-by-squaring on existing complex `Mul`; n<0 via `checked_div`. `powf`: `r=abs`, `θ=arg`, real `r^x` via `oxinum_float::pow`/`BigFloat::pow`, then `cos(xθ)+i·sin(xθ)`. In `src/transcendental.rs` + `src/native/transcendental.rs`.
  - **Files:** `src/transcendental.rs`, `src/native/transcendental.rs`
  - **Tests:** `(1+i).powi(2) = 2i`; `i.powi(4) = 1`; `powi(-1)` = reciprocal; `powf` matches `pow`.
  - **Risk:** Negative-base `powf` branch-cut → document principal `arg`, test against existing `pow`.
- [x] Verify compatibility with SciRS2 `ArbitraryComplex` consumer requirements (planned 2026-06-03)
  - **Goal:** Prove `oxinum_complex::CBig` satisfies the exact contract `scirs2-core::ArbitraryComplex` depends on — compile-time (signature) and behavioral. No new public API needed (zero gaps; SciRS2 exists at `../scirs/scirs2-core/src/numeric/arbitrary_precision.rs` and wraps `CBig` directly).
  - **Design:** New `tests/scirs2_arbitrary_complex_compat.rs`. Compile-time: a never-called `fn _assert_contract(...)` enumerating every method the consumer calls with exact types (`CBig::zero()`, `CBig::from_f64(f64,f64)->OxiNumResult<CBig>`, `.to_f64_parts()->(f64,f64)`, `.conj()`, `.abs(usize)/.arg(usize)->OxiNumResult<DBig>`, `.ln/.exp/.sqrt(usize)->OxiNumResult<CBig>`, `.pow(&CBig,usize)->OxiNumResult<CBig>`, owned `Add/Sub/Mul/Div/Neg`, `PartialEq`). Behavioral: zero, from_f64 round-trip, NaN/Inf→Err, conj, arithmetic ops, abs((3,4))≈5, arg((0,1))≈π/2, exp(ln(z))≈z, sqrt(z)²≈z, Euler exp(iπ)+1≈0, pow — all using `bits_to_decimal_digits(128)` digits for precision matching the consumer's convention.
  - **Files:** new `crates/oxinum-complex/tests/scirs2_arbitrary_complex_compat.rs`; `TODO.md:87` → `[x]` on success.
  - **Prerequisites:** none — all consumed methods already exist in the crate.
  - **Tests:** the file is the test.
  - **Risk:** signature mismatch → extract from actual scirs2-core source; epsilon too tight → generous threshold + adequate digits.
