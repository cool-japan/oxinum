# oxinum-float TODO

## Status
Enriched facade over `dashu-float`. Re-exports `FBig`/`DBig`/`Context` and rounding modes, plus convenience functions: elementary math (exp, ln, sqrt, pow) and trigonometric / hyperbolic functions (sin, cos, tan, atan, atan2, sinh, cosh, tanh) computed at configurable precision, and high-precision constants (pi, e, ln2 to 200 digits). All functions take an explicit `precision` argument and run internal arithmetic at guard precision via `with_precision` to avoid accuracy loss. ~530 SLOC production code, 42 tests. Native limb-level float is future work (see root TODO.md).

## Wrapper-level functions (delivered)
- [x] exp(x, precision) -- via dashu binary FBig (validated exp(1) = 2.71828...)
- [x] ln(x, precision) -- DivByZero/Precision error on x <= 0 (validated ln(1) = 0)
- [x] sqrt(x, precision) -- error on x < 0 (validated sqrt(2) = 1.4142135...)
- [x] pow(base, exp, precision) -- via dashu powf (validated 2^10 = 1024)
- [x] sin/cos via Taylor series + argument reduction (25+ correct digits, sin^2+cos^2=1 verified)
- [x] tan via sin/cos quotient (DivByZero on cos=0)
- [x] atan via argument-halving + Taylor (validated atan(1) = pi/4)
- [x] atan2(y, x) with full quadrant handling
- [x] sinh/cosh/tanh via exp-based formulas
- [x] pi / e / ln2 constants (200-digit precomputed, truncated to requested precision)
- [x] sqrt2 constant (computed via sqrt)
- [x] Configurable precision on all transcendental functions

## Native implementation (future work -- not this milestone)
- [x] Implement native `BigFloat` struct: sign + exponent + mantissa + precision (200-300 SLOC)
  - **Plan ID:** N4a — Native `BigFloat` Phase-2 core
  - **Goal:** `oxinum_float::native::BigFloat` (binary base b=2) with explicit precision tracking and post-op rounding. Add/Sub/Neg/Cmp/Display + From<i64>/From<f64>/to_f64 + with_precision + round_to_precision. Mul/Div/Sqrt deferred to N4b.
  - **Design (ultrathink):** Struct `BigFloat { sign: Sign, mantissa: BigUint, exponent: i64, precision: u32 }` representing `(-1)^sign × mantissa × 2^exponent`. Invariants: `precision > 0`; if `mantissa.is_zero()` then canonical zero `{ Positive, 0, 0, precision }`; else `mantissa.bit_length() == precision`. Rounding modes (7): HalfEven, HalfAway, HalfToZero, ToZero, ToInf, ToNegInf, AwayFromZero. Core API: `from_i64(n, prec)`, `from_f64(x, prec) -> OxiNumResult` (Parse error on NaN/Inf), `to_f64`, `with_precision`, `round_to_precision`, `precision()`, `sign()`, `mantissa()`, `exponent()`, `is_zero`, `signum`, `abs`, `neg`. Add/Sub: align exponents with sticky-bit tracking, add/sub signed mantissas, normalize, round to `max(p_a, p_b)`. PartialOrd/Ord: compare signs first, then exponents+mantissas in canonical form. Display: hex-float `0xb…p<exp>` (always exact) plus decimal `Display` cross-validated against dashu.
  - **Files:** `crates/oxinum-float/src/native/mod.rs`, `native/float.rs` (struct + invariants + accessors + Display + round_to_precision), `native/float_add.rs` (Add/Sub/Neg + *Assign + shift-with-sticky), `native/float_convert.rs` (from_i64 / from_f64 / to_f64). `crates/oxinum-float/src/lib.rs` declares `pub mod native;`. Test: `crates/oxinum-float/tests/native_float.rs`.
  - **Prerequisites:** N1 + N2 (DONE).
  - **Tests:** each rounding mode at literal midpoint values (e.g. `0.5` at precision 1, all 7 modes); precision propagation (`add(p10, p20) -> p20`); canonical-zero across precisions; `from_f64(0.5).to_f64() == 0.5`; `from_f64(0.1, 53)` matches exact f64; round-trip 100 random f64s; add commutativity + associativity proptest at fixed precision; `a + 0 == a`; `a + (-a) == 0`; cross-val add/sub vs `dashu_float::DBig` at matching precision over 200 random inputs; Display decimal form matches dashu to N digits as oracle.
  - **Risk:** Banker's rounding at midpoints (classic bug) — per-mode literal-midpoint test matrix. Exp-alignment with sticky-bit tracking — cross-val vs dashu.
- [x] Implement IEEE 754 rounding modes natively (100-150 SLOC) (planned 2026-05-28; covered by N4a)
- [x] Implement addition/subtraction/multiplication with correct rounding (550-700 SLOC) (planned 2026-05-28; add/sub covered by N4a, mul covered by N4b)
- [x] Implement division via Newton-Raphson reciprocal iteration (200-250 SLOC) (planned 2026-05-28; covered by N4b — implemented via rigorous integer-division-on-scaled-mantissa fallback per task's "simpler reference division" provision; advisor recommended this over Newton-Raphson to avoid f64-seed-at-extreme-exponents edge cases)
  - **Plan ID:** N4b — Native `BigFloat` mul / div / sqrt
  - **Goal:** Extend the N4a core with multiplication, division via Newton-Raphson reciprocal, square root via Newton iteration. Depends on N4a.
  - **Design (ultrathink):** Mul: multiply mantissas (uses N1's Karatsuba), sum exponents (check i64 overflow → Overflow error), round to `max(p_a, p_b)`. Div: Newton-Raphson reciprocal — compute `r ≈ 1/b` at `target_prec + 8 guard bits` from f64 seed `1.0 / b_top52_to_f64`, iterate `r' = r * (2 - b*r)` for `ceil(log2(target_prec/52))` iterations (≤ 30 safety cap); then `a/b = a * r` rounded to target. DivByZero error on `b.mantissa.is_zero()`. Sqrt: ensure effective exponent is even (shift mantissa left 1 bit if odd, decrement exponent), then `sqrt(m * 2^(2k)) = sqrt(m) * 2^k` using N2 `BigUint::sqrt` for floor sqrt at scaled mantissa (scale by `2^(target_prec*2)`), divide exponent by 2. Negative input → Domain error.
  - **Files:** `crates/oxinum-float/src/native/float_mul.rs`, `native/float_div.rs`, `native/float_sqrt.rs`; extend `tests/native_float.rs`.
  - **Prerequisites:** N4a complete.
  - **Tests:** Mul commutativity + associativity at fixed precision; `1 * a == a`; `0 * a == 0`; cross-val vs dashu over 200 random pairs. Div: `(a / b) * b ≈ a` to `target_prec - guard` ULPs; `a / a == 1` for non-zero; `OxiNumError::DivByZero` on `a / 0`; cross-val over 100 pairs. Sqrt: `sqrt(x)^2 ≈ x`; `sqrt(4) == 2`; `sqrt(2)^2 ≈ 2.0` to 50 digits; `OxiNumError::Domain` on `sqrt(-1)`.
  - **Risk:** Newton iteration count — too few → wrong; too many → slow. Precision-driven iteration count + safety bound + low-precision convergence cross-check.
- [x] Implement native square root via Newton's method (100-150 SLOC) (planned 2026-05-28; covered by N4b — leverages `BigUint::sqrt`'s Newton iteration on a scaled mantissa, with parity-fixup so the result exponent halves cleanly)
- [x] Implement BigFloat constants (π via Chudnovsky+BS, e via 1/n! BS, ln 2 via Machin atanh) + binary-splitting engine (planned 2026-05-29)
  - **Plan ID:** T1 — BigFloat constants + binary-splitting engine
  - **Goal:** `oxinum_float::native::constants::{pi, e_const, ln2}` at arbitrary precision via Chudnovsky+binary splitting (π), 1/n! binary splitting (e), Machin-like atanh sums (ln 2). Ship a reusable `native::binary_splitting` engine consumed by T2's exp Taylor and T3's trig Taylor.
  - **Design (ultrathink):** Binary-splitting engine in `native/binary_splitting.rs`: generic D&C evaluator for series `Σ a(k)·P(k)/Q(k)`. API: `pub trait BSSeries { fn p_q_a(k: u64) -> (BigInt, BigInt, BigInt); }`, `pub fn binary_split<S: BSSeries>(lo: u64, hi: u64) -> (BigInt, BigInt, BigInt, BigInt)`. π via Chudnovsky (14.18 digits/term, `N = ⌈n·log10(2)/14.18⌉+4` terms, compute `sqrt(640320³)` via BigFloat::sqrt). e via 1/k! BS series with `P(k)=1, Q(k)=k`. ln2 via Hwang identity `14·atanh(1/31)+10·atanh(1/49)+6·atanh(1/161)`, BS each atanh. Caching: `OnceLock<RwLock<Option<BigFloat>>>` storing highest-precision computed; reuse if `cached_prec >= target+16`, else recompute at `target+32` guard.
  - **Files:** `crates/oxinum-float/src/native/binary_splitting.rs` (~300 LoC), `native/constants.rs` (~450 LoC), extend `native/mod.rs`. Test: `crates/oxinum-float/tests/native_constants.rs`.
  - **Prerequisites:** N4a (BigFloat core), N4b (BigFloat sqrt for π's `12/sqrt(640320³)`).
  - **Tests:** `pi(50)` matches first 50 decimal digits; `e_const(50)` matches; `ln2(50)` matches; cache reuse; cross-val vs dashu at prec 100+500; BS combine-rule algebraic identity.
  - **Risk:** Chudnovsky guard bits — mitigated by `target+32` guard and `round_to_precision`. Cache invalidation: write-locks RwLock briefly during recompute.
- [x] Implement exp(x) and ln(x) natively via Taylor+binary-splitting+arg-reduction (planned 2026-05-29)
  - **Plan ID:** T2 — BigFloat `exp` + `ln`
  - **Goal:** `BigFloat::exp(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<Self>` and `BigFloat::ln(&self, prec: u32, mode: RoundingMode) -> OxiNumResult<Self>`.
  - **Design (ultrathink):** exp: special cases (exp(0)=1, overflow/underflow checks); arg-reduction `e^x = (e^(x/2^k))^(2^k)` choosing `k = max(0, ⌈log₂(|x|)⌉ + prec/64)` so `|x/2^k| ≤ 2^{-prec/64}`; Taylor sum via T1's BS engine; square back k times. ln: special cases (ln(1)=0, ln(0)→Domain, ln(neg)→Domain); mantissa decomposition `x=2^k·m`, `ln(x)=k·ln2+ln(m)` using T1's `ln2`; Newton-Raphson `y_{i+1} = y_i - 1 + m/exp(y_i)` with precision-doubling schedule, f64 seed, ≤30 iters cap.
  - **Files:** `crates/oxinum-float/src/native/exp.rs` (~350 LoC), `native/ln.rs` (~300 LoC). Tests in `crates/oxinum-float/tests/native_transcendentals.rs`.
  - **Prerequisites:** T1 (constants + BS), C1 (Domain variant), N4a+N4b.
  - **Tests:** `exp(0)==1`; `exp(1)≈e_const`; `exp(x)*exp(-x)≈1` over 200 random x∈[-50,50]; `ln(1)==0`; `ln(e_const)≈1`; round-trip exp(ln(x))≈x; ln(0)/ln(-1)→Domain; cross-val vs dashu exp/ln over 200 pairs.
  - **Risk:** Newton convergence ringing at low initial precision — precision-doubling schedule mitigates.
- [x] Implement sin/cos/tan/atan/atan2 natively (planned 2026-05-29)
  - **Plan ID:** T3 — BigFloat trig
  - **Goal:** `BigFloat::sin/cos/tan/atan/atan2` at arbitrary precision.
  - **Design (ultrathink):** sin/cos: mod-2π reduction with dynamic guard `g=⌈log₂(|x|)⌉`; octant reduction to `[-π/4,π/4]`; Taylor via BS engine. tan = sin/cos (Domain on cos=0). atan: range reduction (`|x|>1` → `sign·π/2-atan(1/x)`); half-angle acceleration `atan(x)=2·atan(x/(1+sqrt(1+x²)))` applied until `|x|<2^{-prec/64}`; Taylor via BS. atan2: 8-quadrant table.
  - **Files:** `crates/oxinum-float/src/native/trig.rs` (~500 LoC), `native/atan.rs` (~300 LoC). Tests in `native_transcendentals.rs`.
  - **Prerequisites:** T1 (π + BS).
  - **Tests:** `sin(0)==0`, `cos(0)==1`; Pythagorean identity over 200 random x; Machin verification `π=16·atan(1/5)-4·atan(1/239)` to 50 digits; atan2 8-quadrant table; cross-val vs dashu.
  - **Risk:** Catastrophic cancellation in arg reduction for huge |x| — mitigated by dynamic guard bits.
- [x] Implement special values: NaN, +Inf, -Inf, +0, -0 with IEEE 754 semantics (100-150 SLOC) (planned 2026-05-29)
  - **Goal:** Add `FloatClass { Finite, Infinite, Nan }` tag to `BigFloat`; wire full IEEE-754 Additive model — operators produce/propagate NaN/±Inf (never panic), checked methods keep `OxiNumError` finite-domain contract. Single canonical zero (no signed −0).
  - **Design:** `FloatClass` enum in float.rs; `class` as first BigFloat field; `nan/infinity/neg_infinity` constructors; `is_zero` redefined to require `Finite && mantissa.is_zero()`; remove `Eq`/`Ord`, rewrite `PartialEq`/`PartialOrd`, add `cmp_finite` + inherent `total_cmp`; new `nonfinite.rs` with `nonfinite_binop`/`nonfinite_propagate` helpers; operators stop panicking (`div_or_panic→div_ieee`); transcendental guards return `Ok(NaN/Inf)` for non-finite inputs; serde `BigFloatRepr` gains `#[serde(default)] class`; Display/format_ext emit `NaN`/`inf`/`-inf`.
  - **Files:** float.rs, nonfinite.rs (new), mod.rs, float_add.rs, float_mul.rs, float_div.rs, float_sqrt.rs, float_ln.rs, ln_agm.rs, float_exp.rs, trig.rs, atan.rs, pow.rs, float_convert.rs, format_ext.rs, serde_impl.rs.
  - **Prerequisites:** none.
  - **Tests:** new native_nonfinite.rs; rewrite div/rem `#[should_panic]` tests; serde NaN/Inf round-trip.
  - **Risk:** `is_zero()` redefinition is load-bearing (NaN/Inf share mantissa=0 with zero); dropping Eq/Ord proven safe by workspace blast-radius grep (no dependents).
- [x] Implement engineering-notation string formatting (150-200 SLOC) (planned 2026-05-29; EF1)
  - **Goal:** BigFloat::to_engineering_string(digits) — decimal exponent a multiple of 3, mantissa ∈ [1,1000); plus scientific helper.
  - **Design:** reuse existing decimal-conversion path (float_convert.rs); compute decimal exponent, snap down to nearest multiple of 3, shift mantissa digits.
  - **Files:** crates/oxinum-float/src/native/format_ext.rs (new) + native/mod.rs re-export; tests in tests/native_float.rs (or native_format.rs).
  - **Tests:** 12345→"12.345e3"; 0.0001234→"123.4e-6"; exponent always ≡0 mod 3.
  - **Risk:** decimal rounding at digit boundaries — reuse tested decimal path, assert exp≡0 mod 3.
- [x] Implement hex float parsing (150-200 SLOC) (planned 2026-05-29; EF1 — paired with hex formatting)
  - **Goal:** BigFloat::from_hex_float(&str) and to_hex_string() — C99 %a-style ±0x1.<hex-frac>p±<binexp>.
  - **Design:** BigFloat is binary (mantissa·2^exp) so hex is near-direct: emit leading bit, group remaining mantissa bits into hex nibbles, p-exponent = binary exponent; parsing inverts (validate 0x prefix, hex digits, optional '.', mandatory p; build mantissa/exponent; malformed → OxiNumError::Domain/Parse). NO unwrap() in parser.
  - **Files:** crates/oxinum-float/src/native/format_ext.rs (shared with eng-notation); tests in tests/native_float.rs.
  - **Tests:** round-trip from_hex_float(to_hex_string(x))==x exactly (binary-exact) over 200 random incl. negative/zero/subnormal-magnitude; from_hex_float("0x1.8p3")==12.0; malformed → Err.
  - **Risk:** hex path is binary-exact (no rounding) — round-trip must be bit-identical.

## API Improvements
- [x] Implement `std::ops::{Add, Sub, Mul, Div, Neg}` for owned and borrowed operands (planned 2026-05-28; covered via dashu `DBig` re-export — wrapper verification only)
- [x] Implement `PartialOrd` (NaN-aware, IEEE 754 totalOrder optional) (planned 2026-05-28; covered via dashu re-export; NaN-aware caveat documented since dashu has no NaN representation)
- [x] Implement `Display` with precision control (format specifier support) (planned 2026-05-28; covered by F1)
- [x] Implement `serde::Serialize` / `Deserialize` behind `serde` feature (planned 2026-05-28)
  - **Plan ID:** F1 — Float wrapper polish
  - **Goal:** Close gaps on `DBig` wrapper: `serde` feature, `with_precision`/`epsilon`/`ulp` helpers, document Hash-not-applicable.
  - **Design:** `crates/oxinum-float/Cargo.toml` adds `[features]` `serde = ["dep:serde", "dashu/serde"]` with optional `serde` dep. Public helpers (free fns): `with_precision(&DBig, u32) -> DBig` (delegate to dashu method), `epsilon(prec: u32) -> DBig` (smallest positive at given decimal precision), `ulp(&DBig) -> DBig` (unit in last place at x's precision), `signum(&DBig) -> i32` (if not already). Rustdoc on lib.rs preamble notes Hash not impl per IEEE 754.
  - **Files:** `crates/oxinum-float/Cargo.toml`, `crates/oxinum-float/src/lib.rs` (extend with helpers + module-level doc) or new `src/precision.rs`.
  - **Prerequisites:** none.
  - **Tests:** `epsilon(10)` matches expected literal; `with_precision(x, 100)` yields `precision() == 100`; `ulp(1.0)` matches `epsilon(prec)`; serde JSON round-trip for `DBig` with `--features serde`.
  - **Risk:** dashu `Context` / `with_precision` API shape — verify at impl time.
- [x] Add `BigFloat::with_precision(prec)` constructor (planned 2026-05-28; covered by F1 as `with_precision` free fn)
- [x] Add `BigFloat::epsilon(prec)` returning smallest representable value at given precision (planned 2026-05-28; covered by F1)
- [x] Add `BigFloat::ulp()` returning unit in the last place (planned 2026-05-28; covered by F1)
- [x] Implement num_traits::{Zero,One,Num,Signed} on native BigFloat (planned 2026-05-29)
  - **Plan ID:** N7 — num_traits impls (BigFloat portion)
  - **Goal:** `num_traits::{Zero, One, Num, Signed}` on `native::BigFloat`, gated behind `num-traits` feature.
  - **Design:** `crates/oxinum-float/src/native/num_traits_impl.rs` (~250 LoC). `Zero::zero()` → `BigFloat::zero(prec)` at default prec 53; `One::one()` → `BigFloat::from_i64(1, 53)`; `Num::from_str_radix` → parse via `from_f64(s.parse::<f64>()?, prec)`; `Signed::{abs,signum,is_positive,is_negative}` delegate to inherent methods. `num_traits::Float` deferred (needs NaN/Inf). All gated `#[cfg(feature = "num-traits")]`.
  - **Files:** `crates/oxinum-float/src/native/num_traits_impl.rs`, re-export from `native/mod.rs`.
  - **Prerequisites:** num-traits feature flag (verify it is in oxinum-float/Cargo.toml; add if missing).
  - **Tests:** `BigFloat::zero(53).is_zero() == true`; generic sum test; serde compatibility.
  - **Risk:** `Num::FromStrRadixErr` type — use `String` wrapper to keep it simple.
- [x] Implement `num_traits::Float` trait where applicable (planned 2026-05-29)
  - **Goal:** Implement applicable num_traits float traits: `FloatConst` (PI, E, LN_2, SQRT_2, …, 16 methods) and `TotalOrder` (total_cmp). Document why `Float`/`FloatCore`/`Real` are inapplicable: (1) `Copy` supertrait — BigFloat is heap-backed (Vec<u64>), non-Copy; (2) ill-defined `max_value`/`min_value`/`min_positive_value`/`integer_decode` for unbounded precision.
  - **Design:** `impl FloatConst` — 16 required methods at `DEFAULT_PREC = 53`, derived from `constants::pi/e_const/ln2` + `sqrt`/`ln`/`div_ref`/`from_i64`; `.expect()` for documented-infallible constant generation. `impl num_traits::float::TotalOrder` delegates to inherent `total_cmp`. Module-doc rewrite.
  - **Files:** num_traits_impl.rs.
  - **Prerequisites:** NF1 (float:64) — needs `total_cmp` + `FloatClass` predicates.
  - **Tests:** `FloatConst::PI()` > 0; `TotalOrder::total_cmp(nan,nan)==Equal`; `total_cmp(+inf,nan)==Less`.
  - **Risk:** `.expect()` is the only production `.expect` in this crate — intentional documented-infallible exception.

## Testing
- [x] Verify elementary function accuracy against reference values (pi, e, ln2, sqrt2, sin/cos(0.7))
- [x] Test rounding modes produce correct results for midpoint cases (Item 4, 2026-05-28; `crates/oxinum-float/tests/properties.rs`)
- [x] Test special value propagation: NaN + x = NaN, Inf + Inf = Inf, Inf - Inf = NaN (Item 4, 2026-05-28; **scoped to backend** -- `dashu-float` represents no `NaN` and only a sentinel `±Inf` whose arithmetic panics, so the wrapper's non-finite outcomes surface as `OxiNumError`. The `special_values_scoped` module in `crates/oxinum-float/tests/properties.rs` documents this and asserts the error-returning equivalents: `ln(0)`, `ln(-1)`, `sqrt(-1)`, `pow(0, 1)`, `pow(-2, 1)` -> `OxiNumError::Precision`; also asserts that `from_str("inf"|"nan")` are parse errors.)
- [x] Test trig identity: sin^2(x) + cos^2(x) = 1 to 25+ digits
- [x] Test string round-trip: parse(display(x)) == x for various precisions (Item 4, 2026-05-28; `crates/oxinum-float/tests/properties.rs`)
- [x] Property tests: addition commutativity, multiplication commutativity (Item 4, 2026-05-28; proptest in `crates/oxinum-float/tests/properties.rs`)
- [x] Cross-validate against dashu-float for regression detection (Item 4, 2026-05-28; `crates/oxinum-float/tests/properties.rs`)

## Performance
- [x] Benchmark AGM-based ln vs Taylor series for various precisions
- [x] Benchmark Chudnovsky pi vs Machin-like formulas (Chudnovsky IS the implementation used in pi(); bench_pi covers 100/500/1000-bit (prec ≈ 30/150/301 decimal digits) precisions side-by-side with e_const and ln2 in constants.rs; 3322-bit case ≈ 1000 decimal digits added)
- [ ] Profile allocation patterns in exp/sin/cos computation chains
- [x] Benchmark against dashu-float for equivalent operations (BM1 already covered pi/exp/ln at prec 1000 with native vs dashu)
- [ ] Consider binary splitting parallelization for constant computation

## Integration
- [x] Implement BigFloat pow/log_base + BigRational↔BigFloat conversion at specified precision (planned 2026-05-29)
  - **Plan ID:** T4 — BigFloat pow + log + BigRational↔BigFloat conversion
  - **Goal:** `BigFloat::pow(&self, exp: &Self, prec, mode)`, `BigFloat::log(&self, base: &Self, prec, mode)`, `rational_to_float(&BigRational, prec, mode) -> BigFloat`, `float_to_rational(&BigFloat) -> BigRational` (exact).
  - **Design:** pow: integer fast-path via repeated squaring on mantissa when `y.exponent >= 0`; else `exp(y*ln(x))` requiring `x>0`. `pow(0,0)=1` by convention. log_base: `ln(x)/ln(base)`. rational_to_float: convert num/den via `BigFloat::from_bigint`, then divide. float_to_rational: `exponent >= 0` → `num = mantissa*2^exp, den=1`; else `num = mantissa, den = 2^(-exp)`.
  - **Files:** `crates/oxinum-float/src/native/pow.rs` (~250 LoC); extend `float_convert.rs` with `from_bigint`. Tests in `native_transcendentals.rs`.
  - **Prerequisites:** T2 (exp + ln), N4a/N4b.
  - **Tests:** `pow(2,10)==1024`; `pow(x,0)==1`; `pow(x,y)*pow(x,-y)≈1`; `log_base(100,10)≈2`; rational↔float round-trip; `float_to_rational(1.5)==3/2`.
  - **Risk:** `pow(0,0)` convention documented explicitly.
- [ ] Verify compatibility with SciRS2 floating-point needs (matrix decomposition, optimization)
- [x] Provide re-export path through oxinum facade crate
- [x] Ensure Context type works with oxinum facade's round module
