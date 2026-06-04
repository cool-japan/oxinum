# oxinum-rational TODO

## Status
Enriched facade over `dashu-ratio`. Re-exports `RBig`/`Relaxed` (with `BigRational` alias) plus an operations module: continued fraction expansion and reconstruction, best rational approximation (convergent-based), decimal string conversion, mediant, mixed-number decomposition, floor/ceil/round/truncate, abs/signum/reciprocal, and integer power (incl. negative exponents). ~360 SLOC production code, 44 tests. Automatic simplification and core arithmetic are provided by dashu (native is future work).

## Operations (wrapper-level, delivered)
- [x] continued_fraction(x) -> Vec<IBig> (validated 355/113 = [3; 7, 16])
- [x] from_continued_fraction(coeffs) -> RBig (roundtrip verified)
- [x] best_rational_approximation(x, max_denom) via convergents (355/113 -> 22/7)
- [x] to_decimal_string(x, places) configurable decimal places
- [x] mediant(a, b) = (a_num + b_num) / (a_den + b_den)
- [x] mixed_number(x) -> (whole: IBig, frac: RBig)
- [x] rational_floor / rational_ceil / rational_round / rational_truncate
- [x] rational_abs / rational_signum / rational_reciprocal (DivByZero on 0)
- [x] rational_pow(n: i32) with negative exponents
- [x] Automatic simplification (via dashu, e.g. 6/4 -> 3/2) verified

## Native implementation (future work -- not this milestone)
- [x] Implement native `BigRational` over oxinum-int's BigInt/BigUint, auto-reduced (200-250 SLOC) (planned 2026-05-28)
  - **Plan ID:** N3 — Native `BigRational` (Phase 3)
  - **Goal:** A correct, fully-tested `oxinum_rational::native::BigRational` always in lowest terms, with full arithmetic, conversions, comparisons, Display — available alongside the existing dashu `RBig` re-exports.
  - **Design (ultrathink — invariants matter):** Struct `BigRational { num: BigInt, den: BigUint }`. Invariants on every construction: (a) `den > 0` (DivByZero error if den==0), (b) `gcd(|num|, den) == 1` (auto-reduce via `oxinum_int::native::gcd`), (c) canonical zero `{ num: 0, den: 1 }`. Constructors: `from_parts(BigInt, BigUint) -> OxiNumResult<Self>`, `from_integer(BigInt)`, `from_i64`, `ZERO`/`ONE`, `is_zero`/`is_one`/`is_integer`/`signum`/`abs`/`recip`. Arithmetic owned + borrowed + Assign: Add/Sub via cross-multiplication with `g = gcd(b,d)` lcm optimization; Mul with diagonal pre-reduce `gcd(a,d) gcd(c,b)`; Div via reciprocal; Neg flips num sign; Rem trait completeness. PartialOrd/Ord via cross-mul with sign; Hash over `(sign(num), |num|, den)`. Display: "n/d" or "n" if integer. From primitives `i8..i128`/`u8..u128`/`isize/usize`.
  - **Files:** `crates/oxinum-rational/src/native/mod.rs`, `native/rational.rs`, `native/rational_ops.rs`; `crates/oxinum-rational/src/lib.rs` declares `pub mod native;`. Test: `crates/oxinum-rational/tests/native_rational.rs`.
  - **Prerequisites:** N1 + N2 (DONE in Round 1).
  - **Tests:** invariants post-construction (`6/4` → `3/2`, `-9/-12` → `3/4`, `0/5` → `0/1`); hand-picked equalities (`1/2 + 1/3 == 5/6`, `2/3 * 3/4 == 1/2`, `(-3)/4 + 3/4 == 0`); proptest add/mul comm + assoc + distrib, `a + (-a) == 0`, `a * a.recip() == 1`, ord transitivity, hash-eq consistency; cross-val vs `dashu_ratio::RBig` over ~300 random pairs with magnitudes up to 4096 bits; DivByZero on `from_parts(_, 0)` and `recip(0)`.
  - **Risk:** Sign + reduction interaction (e.g. `from_parts(-6, 4)` must reduce to `(-3, 2)` not `(3, -2)`); mitigated by sign-on-num invariant + adversarial-input unit tests.
- [x] Implement Stern-Brocot tree traversal for ordered rational enumeration (100-150 SLOC) (planned 2026-05-28)
  - **Plan ID:** Item 3 — oxinum-rational wrapper completions
  - **Goal:** Close the rational wrapper gaps: string/float interop + Stern-Brocot/Farey enumeration.
  - **Design (all on dashu `RBig`, reusing `rbig_from_signed`/`div_floor` in `ops.rs`):**
    - `parse_mixed(s: &str) -> OxiNumResult<RBig>` parses `"1 3/4"`, `"-2 1/3"`, plus plain `"3/4"`/`"3"` (delegating to dashu `RBig::from_str` for non-mixed); a thin wrapper newtype may carry `FromStr` to avoid orphan-rule on `RBig`.
    - `from_f64(x: f64) -> OxiNumResult<RBig>` / `from_f32`: exact via bit decomposition (`x.to_bits()` → sign/mantissa/exponent → `mantissa * 2^exp` via `RBig::from_parts`); error on NaN/Inf (`OxiNumError::Parse`). No `unsafe`.
    - `to_f64(x: &RBig) -> f64` (nearest) and `to_f64_exact(x: &RBig) -> OxiNumResult<f64>` (error if not exactly representable; detect via round-trip compare); overflow → `OxiNumError::Overflow`.
    - `stern_brocot_path(x: &RBig) -> Vec<bool>` (L/R turns to locate a positive rational) and `from_stern_brocot_path(&[bool]) -> RBig`.
    - `farey_sequence(n: u64) -> Vec<RBig>` (order-n Farey via mediant/neighbor recurrence, ascending in [0,1]).
  - **Files:** `crates/oxinum-rational/src/ops.rs` (extend) or new `src/convert.rs`/`src/enumerate.rs` (keep each < 2000 lines); re-export from `lib.rs`.
  - **Prerequisites:** Item 0 (proptest dev-dep) for property tests.
  - **Tests:** mixed-number round-trip `"1 3/4"` ⇒ 7/4 ⇒ display; `from_f64(0.5)==1/2`, `from_f64(0.1)` exact dyadic check; `to_f64(1/3)` ≈ 0.333…; Stern-Brocot path of 22/7 round-trips; **Farey(5) == [0/1,1/5,1/4,1/3,2/5,1/2,3/5,2/3,3/4,4/5,1/1]** strictly ascending; f64 round-trip for exact dyadic rationals.
  - **Risk:** dashu `RBig`→`f64` API shape — confirm at impl time, fall back to manual long-division rounding if needed.
- [x] Implement Farey sequence generation for order n (80-100 SLOC) (planned 2026-05-28; covered by Item 3)
- [x] Implement `FromStr` for mixed-number format "1 3/4" (dashu handles "3/4", "-1/2", "3") (60 SLOC) (planned 2026-05-28; covered by Item 3 — `parse_mixed` free fn + optional newtype FromStr to avoid orphan rule)
- [x] Implement `From<f32>`, `From<f64>` via exact binary fraction decomposition (80-100 SLOC) (planned 2026-05-28; covered by Item 3 — exposed as `from_f64`/`from_f32` free fns to avoid orphan rule on `RBig`)
- [x] Implement `TryInto<f64>` with overflow/precision-loss detection (40-50 SLOC) (planned 2026-05-28; covered by Item 3 as `to_f64_exact` free fn)

## API Improvements
- [x] Implement `std::ops::{Add, Sub, Mul, Div, Neg, Rem}` for owned and borrowed operands (planned 2026-05-28; covered via dashu `RBig` re-export — wrapper verification only)
- [x] Implement `AddAssign, SubAssign, MulAssign, DivAssign` (planned 2026-05-28; covered via dashu `RBig` re-export — wrapper verification only)
- [x] Implement `PartialOrd, Ord, Hash` (planned 2026-05-28; covered via dashu `RBig` re-export — wrapper verification only)
- [x] Implement `serde::Serialize` / `Deserialize` behind `serde` feature (planned 2026-05-28)
  - **Plan ID:** R1 — Rational wrapper polish
  - **Goal:** Close gaps identified in the wrapper audit: `serde` feature, `from_integer(BigInt)` convenience, `is_integer()` predicate, num_traits feature gating.
  - **Design:** `crates/oxinum-rational/Cargo.toml` adds `[features]` `serde = ["dep:serde", "dashu/serde"]` and `num-traits = ["dep:num-traits"]` with optional deps. Public free fns (avoiding orphan-rule on `RBig`): `rational_from_integer(&IBig) -> RBig`, `rational_is_integer(&RBig) -> bool` (denom == 1), `rational_to_integer(&RBig) -> Option<IBig>`. Serde for `RBig` passes through dashu's serde feature; `MixedNumber` newtype gets `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`. num_traits: open the feature flag, defer impls themselves to a follow-up round.
  - **Files:** `crates/oxinum-rational/Cargo.toml`, `crates/oxinum-rational/src/lib.rs` (re-export new fns), `crates/oxinum-rational/src/ops.rs` (extend) or new `src/traits.rs`.
  - **Prerequisites:** none.
  - **Tests:** `rational_from_integer(42)` round-trips `to_integer`; `is_integer(3/1)==true`, `is_integer(3/2)==false`; serde JSON round-trip for `RBig` and `MixedNumber` with `--features serde`; verify dashu serde feature passes through.
  - **Risk:** dashu's serde feature naming — verify at impl time by reading `dashu-ratio/Cargo.toml`.
- [x] Implement num_traits::{Zero,One,Num,Signed} on native BigRational (planned 2026-05-29)
  - **Plan ID:** N7 — num_traits impls (BigRational portion)
  - **Goal:** `num_traits::{Zero, One, Num, Signed}` on `native::BigRational`, gated behind `num-traits` feature.
  - **Design:** `crates/oxinum-rational/src/native/num_traits_impl.rs` (~250 LoC). `Zero::zero()` → `BigRational::zero()`; `One::one()` → `BigRational::one()`; `Num::from_str_radix(s, 10)` parses `"n/d"` (splitting on `/`, parsing each part via BigInt). `Signed::abs/signum/is_positive/is_negative` delegate to inherent methods. All gated `#[cfg(feature = "num-traits")]`.
  - **Files:** `crates/oxinum-rational/src/native/num_traits_impl.rs`, re-export from `native/mod.rs`. Tests in `crates/oxinum-rational/tests/native_rational.rs` under `#[cfg(feature = "num-traits")]`.
  - **Prerequisites:** num-traits feature flag (already in Cargo.toml from R1).
  - **Tests:** `BigRational::zero().is_zero()==true`; `Num::from_str_radix("3/4",10)==BigRational::from_parts(3,4)`; generic sum test; `BigRational::from(-5).is_negative()==true`.
  - **Risk:** `from_str_radix` on non-decimal bases — document "only base 10 supported" and return error for others.
- [x] Implement `num_traits::Zero`, `One`, `Num`, `Signed` (planned 2026-05-28; covered by R1 — feature flag opened, trait impls deferred to follow-up)
- [x] Add `BigRational::from_integer(n)` convenience constructor (planned 2026-05-28; covered by R1 as `rational_from_integer` free fn)
- [x] Add `BigRational::is_integer()` predicate (planned 2026-05-28; covered by R1 as `rational_is_integer` free fn)

## Testing
- [x] Test automatic simplification: 6/4 becomes 3/2
- [x] Test continued fraction expansion of known values (355/113 = [3; 7, 16]) + roundtrip
- [x] Test Stern-Brocot tree produces correct Farey sequence ordering (planned 2026-05-28; covered by Item 3 — Farey(5) exact assertion + ascending check)
- [x] Test f64 round-trip: BigRational::from(x).to_f64() preserves value for exact floats (planned 2026-05-28; covered by Item 3)
- [x] Test mixed number: 7/3 = 2 + 1/3 (and negative)
- [x] Property test: (a/b) * (b/a) = 1 for non-zero a, b (reciprocal roundtrip)
- [x] Test edge cases: 0/1, 1/1, negative fractions
- [x] Cross-validate against dashu-ratio for regression detection (broader) (planned 2026-05-29; CF1 — native continued-fraction cross-val + broader rational regression)
  - **Goal:** Cross-validate native BigRational continued fractions and arithmetic against dashu-ratio over broad random inputs.
  - **Design:** drive native continued_fraction()/from_continued_fraction()/best_rational_approximation() (CF1, in native/continued_fraction.rs) against the existing RBig wrapper CF helpers (src/ops.rs) over random rationals.
  - **Files:** crates/oxinum-rational/tests/native_rational.rs (cross-val tests).
  - **Tests:** cf(415/93)==[4,2,6,7]; from_cf(cf(r))==r (200 random); best_approx(π,den≤113)==355/113; agreement with dashu-ratio CF on random; negative/integer/1-over-n edges.

## Performance
- [x] Benchmark GCD-based simplification vs lazy simplification (Relaxed-style)
- [x] Profile allocation patterns in continued fraction expansion (planned 2026-06-03)
  - **Goal:** A report bench (`benches/alloc_profile.rs`, `harness=false`, plain `fn main`) that counts allocations in native `BigRational::{continued_fraction, convergents, best_rational_approximation}` over representative rationals, printing every `AllocStats` field. Also adds a native-vs-dashu baseline group (add/mul) to `benches/rational_arith.rs`.
  - **Design:** file-local `CountingAlloc` `#[global_allocator]` in the bench binary (legal; bench binaries do NOT inherit the library's `#![forbid(unsafe_code)]`). Atomics track alloc/dealloc calls, cumulative bytes, live bytes, peak bytes; helpers `reset()/snapshot()/measure(||…)`. Test rationals: Fibonacci F(n+1)/F(n) (all-1 CF, long expansion), 355/113 (π convergent), 1457/991, and a large-coefficient rational. Baseline group: oxinum native `BigRational` add/mul vs dashu `RBig::from_parts(IBig,UBig)`. CF has no dashu rational equivalent (dashu-ratio does not expose `continued_fraction`) — documented in the module comment; CF coverage is via alloc-profile only.
  - **Files:** new `crates/oxinum-rational/benches/alloc_profile.rs`; edit `crates/oxinum-rational/benches/rational_arith.rs`; add `[[bench]] name="alloc_profile" harness=false` to `crates/oxinum-rational/Cargo.toml`. No new external dep (dashu-ratio/int already normal deps).
  - **Prerequisites:** none.
  - **Tests:** `cargo bench -p oxinum-rational --no-run`; alloc_profile binary runs and prints.
  - **Risk:** `make_rat` uses `.expect()` on a known-nonzero denominator — allowed in bench code.
- [x] Benchmark rational arithmetic chains (matrix operations with exact fractions)

## Integration
- [x] Ensure BigRational uses oxinum-int's BigInt/BigUint (not dashu directly)
- [x] Implement rational_to_float / float_to_rational conversion at specified precision (planned 2026-05-29)
  - **Plan ID:** T4 — BigRational↔BigFloat conversion (rational portion)
  - **Goal:** `rational_to_float(&BigRational, prec: u32, mode: RoundingMode) -> OxiNumResult<BigFloat>` and `float_to_rational(&BigFloat) -> BigRational` (exact). Part of the cross-domain T4 plan shared with oxinum-float.
  - **Design:** `rational_to_float`: use `BigFloat::from_bigint` on `r.num()` and `r.den()` (at `prec + guard`), then divide. `float_to_rational`: if `exponent >= 0`, `BigRational::from_parts(±mantissa·2^exp, 1)`; else `BigRational::from_parts(±mantissa, 2^(-exp))`; auto-reduces via N3 invariant.
  - **Files:** `crates/oxinum-rational/src/native/convert.rs` (~150 LoC). Re-export via `native/mod.rs`.
  - **Prerequisites:** T2 (exp + ln for pow), N4a/N4b (BigFloat core), N3 (native BigRational).
  - **Tests:** `rational_to_float(1/3, 100).to_f64() ≈ 0.333`; `float_to_rational(BigFloat::from_f64(1.5)?) == BigRational::from_parts(3.into(), 2u8.into())?`; binary-rational round-trip; general rational agrees to prec-2 ULPs after re-conversion.
  - **Risk:** `pow(0,0)` convention (1) and T4 `log_base(base,base)=1` tolerance.
- [x] Verify compatibility with SciRS2 exact arithmetic requirements (verified 2026-06-03)
  - **Delivered:** `tests/scirs2_rational_compat.rs` — 14 tests proving `RBig::from_parts`, `.numerator()/.denominator()`, `rational_abs`, `rational_reciprocal`, `to_f64`, and the `to_arbitrary_float` division path all match SciRS2's exact usage; includes the `with_precision` pre-scaling step.
- [x] Provide re-export path through oxinum facade crate
