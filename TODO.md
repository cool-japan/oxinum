# OxiNum Project TODO

## Status — v0.1.2 released 2026-06-10

Full native Pure-Rust implementation delivered (~21 000 lines Rust, zero warnings, FFI-audit clean). OxiNum provides: native BigUint/BigInt (schoolbook/Karatsuba/Toom-3/Knuth-D/BPSW), native BigFloat (sqrt/exp/ln/trig/constants via binary-splitting), native BigRational (continued fractions, best-approximation), arbitrary-precision complex (`oxinum-complex`: `CBig` over `DBig` and native `BigComplex` over `BigFloat`), and a unified facade with prelude/constants/convert/parse. The `dashu-*` crate family remains the production backend; native types coexist and are fully validated against dashu via cross-validation.

### Unreleased
- [x] **`oxinum-complex` crate delivered** — `CBig` (decimal-backed, `re`/`im` each a `DBig`) plus ground-up `native::BigComplex` (binary, over `native::BigFloat`): construction, arithmetic, `conj`/`norm_sqr`, `abs`/`arg`, `exp`/`ln`/`sqrt`/`pow`, complex `sin`/`cos`/`tan`/`sinh`/`cosh`/`tanh`, serde + num-traits. Integrated into the `oxinum` facade as `oxinum::CBig`/`Complex` and `oxinum::native::BigComplex`.
- [x] **Fixed** precision-collapse defect in `oxinum-float` `atan`/`atan2` (DBig free functions) — accuracy was capped at ~3e-3 regardless of requested precision; now accurate to full precision.

## Milestones

### M0 -- Skeleton (DONE)
- [x] Workspace compiles clean (`cargo build --workspace --no-default-features`)
- [x] `oxinum-core` re-exports dashu-base `Sign` + defines `OxiNumError`
- [x] `oxinum-int` re-exports `UBig`, `IBig` from dashu-int
- [x] `oxinum-float` re-exports `FBig`, `DBig` from dashu-float
- [x] `oxinum-rational` re-exports `RBig`, `Relaxed` from dashu-ratio
- [x] `oxinum` facade with `pure` (default) feature gating subcrate re-exports
- [x] `deny.toml` bans rug/gmp-mpfr-sys/gmp-sys/mpfr-sys/gmp-mpfr (tree-wide)
- [x] `Dockerfile.ffi-audit` passes in rust:slim (no apt-get)
- [x] `scripts/ffi-audit.sh` passes

## Core Implementation
_Note: Phases 1-7 describe native limb-level math (replacing the dashu backend) and remain future work. The current milestone delivered wrapper-level enrichment on top of dashu (see per-crate TODO.md files for the delivered checklist)._
- [x] Phase 1: Native BigUint/BigInt in oxinum-int with schoolbook and Karatsuba multiplication (~2000 SLOC) (planned 2026-05-28; covered by N1+N2)
- [x] Phase 2: Native BigFloat in oxinum-float with configurable precision and elementary functions (~3000 SLOC) (delivered Round 3: native BigFloat core + sqrt + exp/ln/trig/pow + π/e/ln2 via binary splitting)
- [x] Phase 3: Native BigRational in oxinum-rational with automatic simplification and continued fractions (~1200 SLOC) (planned 2026-05-29; auto-reduction delivered in N3; continued fractions = CF1 this round)
  - **Goal (CF1):** Native continued fractions on BigRational: continued_fraction()->Vec<BigInt>, from_continued_fraction(), convergents(), best_rational_approximation(max_den).
  - **Design:** Euclidean CF expansion (a_i=floor(num/den), recurse on den, num-a_i*den); convergent recurrence h_i=a_i*h_{i-1}+h_{i-2}; semiconvergent best-approx bounded by max_den.
  - **Files:** crates/oxinum-rational/src/native/continued_fraction.rs (new) + native/mod.rs re-export; tests in tests/native_rational.rs.
  - **Tests:** cf(415/93)==[4,2,6,7]; round-trip from_cf(cf(r))==r (200 random); best_approx(π,den≤113)==355/113; dashu-ratio cross-val; negative/integer/1-over-n edges.
- [x] Phase 4: Advanced multiplication algorithms (Toom-Cook-3, eventually Schonhage-Strassen) (~800 SLOC) (planned 2026-05-29; Toom-Cook-3 = TC1 this round; remaining: Schönhage-Strassen — future)
  - **Goal (TC1):** Third mul tier above Karatsuba; mul() dispatches schoolbook<32 ≤ Karatsuba<TOOM3_THRESHOLD(~100 limbs) ≤ Toom-3. O(n^1.465).
  - **Design:** 3-way limb-block split; evaluate operands at {0,1,-1,2,∞} (signed intermediates); recurse for w0..w4; **Bodrato interpolation schedule** (exact ÷3 and ÷2 helpers — never general divrem); recompose. Internal to mul.rs (no public API change).
  - **Files:** crates/oxinum-int/src/native/mul.rs (mul_toom3 + dispatch), native/uint.rs (const TOOM3_THRESHOLD), tests in tests/native_int.rs.
  - **Tests:** cross-val vs Karatsuba at the threshold boundary (98..104 limbs) + adversarial limb patterns (all-u64::MAX, single-bit/power-of-two, len(a)≫len(b), 0/1 operands) + 200 random + proptest a*b==toom3(a,b).
  - **Risk:** interpolation sign/exact-division bugs (classic Toom-3 locus) — follow published Bodrato schedule verbatim; boundary+adversarial cross-val against correct Karatsuba.
- [x] Phase 5: Number theory module -- primality testing, modular arithmetic, prime generation (~800 SLOC) (planned 2026-05-29; modular arith + deterministic MR + sieve + factorial delivered P1/P2; BPSW = BP1 this round completes it)
  - **Goal (BP1):** Upgrade is_probably_prime to BPSW (Miller-Rabin base 2 + strong Lucas, Selfridge params) — no known counterexample, deterministic below 2^64.
  - **Design:** add jacobi(a,n) (quadratic-reciprocity recursion); Selfridge D-search (D=5,-7,9,-11,…) until jacobi(D,n)=-1, with **perfect-square check on n FIRST** (isqrt(n)²==n ⇒ composite) so search terminates; P=1,Q=(1-D)/4; strong Lucas via existing lucas_uv; wire into is_probably_prime (small-n deterministic MR unchanged, large-n MR2+strong-Lucas).
  - **Files:** crates/oxinum-int/src/native/primality.rs (jacobi+selfridge+strong-Lucas+rewire), native/lucas.rs (minor), tests in tests/native_number_theory.rs.
  - **Tests:** perfect-square inputs composite without hanging (1009²,65537²); all n∈[2,10000] match sieve; Carmichael composite; published strong-Lucas pseudoprimes (5459,5777,10877,16109,18971) composite; Mersenne primes prime; jacobi unit table.
- [x] Phase 6: Cross-domain conversions -- IBig <-> FBig <-> RBig, from/to primitives (~300 SLOC)
- [x] Phase 7: serde feature (Pure Rust serialization) (~100 SLOC) (planned 2026-05-29; SD1 — completes serde across all 4 native types; BigUint/BigInt already done)
  - **Goal (SD1):** Serialize/Deserialize on native BigFloat + BigRational behind each crate's serde feature.
  - **Design:** cfg_attr-derive where fields serialize; BigFloat={sign,mantissa,exponent,precision}; BigRational={num,den}; deserialize re-establishes invariants via #[serde(try_from)] shim (no broken values). Pull oxinum-int/serde in both crates' serde feature.
  - **Files:** crates/oxinum-float/src/native/float.rs (+serde shim), crates/oxinum-rational/src/native/rational.rs (+shim), Cargo.toml feature wiring; tests under #[cfg(feature="serde")].
  - **Tests:** JSON round-trip from_str(to_string(x))==x (zero/negative/high-precision π for float; negative/integer/1-over-3 for rational); malformed/invariant-violating input (den=0, non-reduced 2/4) → error or normalized form.

## API Improvements
- [x] Unify error handling across all sub-crates via `oxinum-core::OxiNumError`/`OxiNumResult`
- [x] Dashu-independent `RoundingMode` enum in oxinum-core for precision vocabulary
- [x] Design consistent builder/context pattern for precision management
- [x] Add serde support across all types (planned 2026-05-28; completed: BigUint/BigInt via I1, BigFloat/BigRational via SD1)
- [x] Add rand support for random number generation (planned 2026-05-29)
  - **Plan ID:** R2 — rand integration (root-level tracking)
  - **Covered by:** `oxinum-int`'s R2 plan block. Adds `rand = { version = "0.9", default-features = false }` to workspace `[workspace.dependencies]`, plus `rand` feature flag in `oxinum-int`. Exposes `BigUint::random_bits`, `BigUint::random_in_range`, `BigInt::random_in_range`, and `Distribution` impls.
- [x] Radix conversion helpers (decimal, hex, octal, binary via in_radix)

## Testing
- [x] Validate functions against reference values (pi/e/ln2, sin/cos, factorial, Fibonacci, primality)
- [x] Property-based testing with proptest for arithmetic laws (planned 2026-05-28; proptest infrastructure wired by Item 0; arithmetic-law proptests added in N1+N2)
- [x] Benchmark suite comparing native vs dashu performance at each phase (planned 2026-05-29; BM1 — criterion harness; also covers "Establish benchmark baselines", "Benchmark Karatsuba crossover", and facade 1000-digit-pi / factorial(1000) benches)
  - **Goal (BM1):** Criterion benches: mul (schoolbook→Karatsuba→Toom-3 crossover) vs dashu, div (Knuth-D vs Newton), factorial(1000), pi(1000 digits), exp/ln at prec 1000, primality (MR vs BPSW).
  - **Design:** criterion (latest) as workspace dev-dep; [[bench]] harness=false per crate under benches/. Gate = compiles clean (cargo bench --no-run) + clippy --all-targets + fast smoke; not full timing.
  - **Files:** crates/oxinum-int/benches/{mul,div,factorial,primality}.rs, crates/oxinum-float/benches/{constants,transcendentals}.rs, Cargo.toml; facade may host end-to-end π/factorial bench.
  - **Risk:** keep criterion dev-dependencies only (pure Rust); re-run ffi-audit.
- [x] Fuzz testing for FromStr parsers
- [x] Float precision assertions (200-digit constants; sin/cos to 25+ digits)

## Performance
- [x] Establish benchmark baselines against dashu, num-bigint (rug intentionally excluded — banned by deny.toml / Pure-Rust policy: GMP/MPFR C deps)
- [x] Optimize limb layout for cache-friendly access patterns (delivered 2026-06-03)
  - **Delivered:** `normalize()` rewritten with `rposition`+`truncate` (O(1) bulk removal vs repeated `pop()`); `add_ref` and `checked_sub` restructured into two sequential passes (overlap + tail) eliminating per-iteration branch; `shl_bits` pre-allocates exact capacity; `from_le_limbs_with_capacity` and `compact()` methods added; 10 new pinned tests verify correctness of optimizations.
- [x] Investigate SIMD-accelerated limb arithmetic (feature-gated) (SIMD ops delivered in oxinum-int as `simd_ops.rs` + optional `simd` feature with nightly `portable_simd` — see `crates/oxinum-int/TODO.md` for details)
- [x] Benchmark Karatsuba crossover point vs schoolbook

## Integration
- [x] Coordinate with SciRS2 for numeric type requirements (verified 2026-06-03)
  - **Delivered:** `scirs2-core/src/numeric/arbitrary_precision.rs` already imports and uses `oxinum_int`, `oxinum_float`, `oxinum_rational`, `oxinum_complex` directly. Compatibility verification tests added: `crates/oxinum-core/tests/scirs2_trait_hierarchy_compat.rs`, `crates/oxinum-int/tests/scirs2_int_compat.rs`, `crates/oxinum-float/tests/scirs2_float_compat.rs`, `crates/oxinum-rational/tests/scirs2_rational_compat.rs`. All 1749 tests pass, zero warnings.
- [ ] Coordinate with OxiBLAS for arbitrary-precision matrix support
  - **Blocked (2026-06-03):** OxiBLAS (`~/work/oxiblas`) currently has no `oxinum` dependency. All numeric operations in OxiBLAS use `f64`/`f32`. Coordination requires: (1) OxiBLAS team adds `oxinum` as an optional feature-gated dep, (2) design of `ArbitraryMatrix<T>` where `T: OxiNum` trait. This is a cross-project decision outside the scope of this run.
- [x] Ensure smooth migration path from dashu re-exports to native types (verified 2026-06-03)
  - **Delivered:** `crates/oxinum/tests/scirs2_facade_compat.rs` contains `mod dashu_drop_in` proving that projects using dashu directly can switch to `oxinum` re-exports (IBig, UBig, DBig, RBig) with no behavioral change. The `oxinum::native::*` namespace provides native replacements under a parallel API. See the facade rustdoc for the migration guide.
- [x] Worked examples: high-precision pi, exact rational linear solve (planned 2026-05-28; covered by Item 5)

## Planned (2026-05-28)

Plan file: `lexical-baking-patterson.md` (this run's approved plan).

### N1 — Native unsigned `BigUint` core (oxinum-int)
- **Goal:** Correct, fully-tested native `oxinum_int::native::BigUint` (little-endian `Vec<u64>` limbs, normalized; canonical zero = empty Vec), additive alongside existing dashu re-exports.
- **Design:** struct + invariants; constructors; `Clone`/`Eq`/`Hash`/`Ord`; carry add via `overflowing_add`; `checked_sub -> Option<BigUint>` only (no `Sub` op); shifts; schoolbook + **Karatsuba** mul (~32-limb threshold); **Knuth Algorithm D** divrem with single-limb fast path + D1 normalization + qhat correction; `bit_length`/`trailing_zeros`/`count_ones`/`test_bit`/`set_bit`/`clear_bit`; bitwise AND/OR/XOR limb-wise (NOT deferred — needs fixed width); `from_bytes_*`/`to_bytes_*`; radix 2..=36; `Display`/`Debug`; `Add`/`Mul`/`Div`/`Rem` (+`*Assign`) owned & borrowed.
- **Files:** `crates/oxinum-int/src/native/{mod.rs,uint.rs,mul.rs,div.rs,radix.rs,ops_uint.rs}`; declare `pub mod native;` in `lib.rs`; integration test `crates/oxinum-int/tests/native_uint.rs`. No `unsafe`.
- **Prerequisites:** Item 0 (proptest dev-dep).
- **Tests:** unit + division surface pinned: (a) single-limb fast path; (b) power-of-2 divisor vs `shr`; (c) Knuth-D normalization edge (top divisor limb ≥ 2^63); (d) ~1000-pair cross-val vs `dashu_int::UBig`; proptest comm/assoc/distrib + `a == (a/b)*b + a%b` + `(a<<k)>>k == a`.
- **Risk:** Knuth-D qhat correction is the classic bug locus; pinned tests + dashu cross-val mitigate.

### N2 — Native signed `BigInt` + GCD + roots (oxinum-int) — *after N1 `done`*
- **Goal:** `oxinum_int::native::BigInt` = sign + `BigUint` magnitude, full signed arithmetic, primitive conversions, binary GCD, integer roots, on top of N1.
- **Design:** **canonical zero = `{ sign: Positive, mag: BigUint::ZERO }` enforced everywhere** (so `+0==-0`, `Hash`/`Eq`/`Ord` consistent); `Add`/`Sub`/`Mul`/`Div`/`Rem`/`Neg` (+`*Assign`) owned & borrowed (Div/Rem truncate toward zero); `From<u64/i64/u128/i128/u32/i32/usize/isize>`; `TryFrom<&BigInt>`/`TryFrom<&BigUint>` for the same with range check → `OxiNumError::Overflow`; **Stein binary GCD** on `BigUint`; **Newton** integer `sqrt` (floor) + `nth_root` on `BigUint`; `BigInt::nth_root` (error on even root of negative).
- **Files:** `crates/oxinum-int/src/native/{int.rs,ops_int.rs,convert.rs,gcd.rs,roots.rs}`; extend `native/mod.rs`; integration test `crates/oxinum-int/tests/native_int.rs`.
- **Prerequisites:** N1 done; Item 0 (proptest).
- **Tests:** signed proptest comm/assoc/distrib + `a == (a/b)*b + a%b` w/ negatives; `+0 == -0`; `TryFrom` boundary tests; GCD properties + cross-val vs dashu `Gcd`; sqrt invariant `sqrt(n)^2 ≤ n < (sqrt(n)+1)^2`.
- **Risk:** sign/canonical-zero drift; remainder-sign convention must match dashu for cross-val.

### Item 5 — facade integration tests + worked examples
- **Goal:** End-to-end demos through the public facade.
- **Design:** integration tests for pi via Machin's formula `π/4 = 4·atan(1/5) − atan(1/239)` (assert against `oxinum::constants::pi`) and exact 3×3 rational determinant; worked examples `high_precision_pi.rs` and `exact_rational_linear_solve.rs` (Cramer's rule over `RBig`).
- **Files:** `crates/oxinum/tests/{machin_pi.rs,rational_determinant.rs}`; `crates/oxinum/examples/{high_precision_pi.rs,exact_rational_linear_solve.rs}`.
- **Prerequisites:** none beyond existing facade API.
- **Tests:** the two integration tests + `cargo build --examples -p oxinum`.
- **Risk:** Machin precision/guard-digit tuning — assert to ~30 digits at guard precision 50.

### Item 0 — Prereq: workspace dependency wiring
Add `proptest = "1"` to `[workspace.dependencies]` and `[dev-dependencies] proptest = { workspace = true }` in oxinum-core, oxinum-int, oxinum-float, oxinum-rational, oxinum. Add `serde = { version = "<latest 1.x>", features = ["derive"], optional = true }` to `[workspace.dependencies]`; oxinum-core gets `serde = { workspace = true, optional = true }` in `[dependencies]` and `[features] serde = ["dep:serde"]`. Run solo first to prevent races on root manifest.
