# oxinum-int TODO

## Status
Enriched facade over `dashu-int`. Provides `BigInt`/`BigUint` aliases (`IBig`/`UBig`), the `Gcd`/`ExtendedGcd` traits, radix conversion helpers, and a number-theory module: factorial (balanced product tree), Fibonacci (fast doubling), Lucas numbers, binomial coefficients, extended GCD, modular exponentiation, Miller-Rabin primality, and next_prime. ~290 SLOC production code, 41 tests. Arithmetic is delegated to dashu (native limb math is future work -- see Phase notes in root TODO.md).

## Number-theory functions (wrapper-level, delivered)
- [x] factorial(n) via balanced product tree (validated against 100! = 158 digits)
- [x] fibonacci(n) via fast-doubling (O(log n) multiplications)
- [x] lucas(n) Lucas numbers via fast-doubling identity
- [x] binomial(n, k) via multiplicative formula with cancellation
- [x] extended_gcd returning Bezout coefficients (verified a*x + b*y = gcd)
- [x] mod_pow modular exponentiation (binary method, DivByZero on modulus 0)
- [x] is_prime Miller-Rabin (deterministic witnesses, validated vs Carmichael numbers)
- [x] next_prime via Miller-Rabin
- [x] from_radix / to_radix helpers (2..=36, validated roundtrip)

## Native implementation (future work -- not this milestone)
- [x] Implement native `BigUint` backed by `Vec<u64>` limbs with schoolbook add/sub (300-400 SLOC) (planned 2026-05-28)
  - **Plan ID:** N1 — Native unsigned `BigUint` core
  - **Goal:** A correct, fully-tested native `oxinum_int::native::BigUint` (little-endian `Vec<u64>` limbs, normalized: no trailing zero limbs, canonical zero = empty `Vec`), additive alongside existing dashu re-exports.
  - **Design:** `BigUint` struct + invariants; constructors `zero`/`one`/`from_u64`/`from_le_limbs`; `Clone`/`PartialEq`/`Eq`/`Hash`/`PartialOrd`/`Ord`; carry add via `overflowing_add`; `checked_sub(&self) -> Option<BigUint>` ONLY (no `Sub` op on `BigUint`); `shl`/`shr` by bit count; schoolbook (`u128` accumulation) + **Karatsuba** with `KARATSUBA_THRESHOLD` (~32 limbs); **Knuth Algorithm D** `divrem` (single-limb fast path, D1 normalization so top divisor limb ≥ 2^63, qhat + 2-step correction, multiply-subtract-with-borrow + add-back) plus `checked_divrem`; `bit_length`/`trailing_zeros`/`count_ones`/`test_bit`/`set_bit`/`clear_bit`; bitwise `BitAnd`/`BitOr`/`BitXor` limb-wise (`Not` deferred — needs fixed width); `from_bytes_be`/`from_bytes_le`/`to_bytes_be`/`to_bytes_le`; `from_str_radix`/`to_radix` (2..=36, powers-of-2 via shifts); `Display`/`Debug` via `to_radix(10)`; `Add`/`Mul`/`Div`/`Rem` (+`*Assign`) for owned & borrowed; Div/Rem panic on zero divisor (documented).
  - **Files:** `crates/oxinum-int/src/native/{mod.rs,uint.rs,mul.rs,div.rs,radix.rs,ops_uint.rs}`; declare `pub mod native;` in `crates/oxinum-int/src/lib.rs`; integration test `crates/oxinum-int/tests/native_uint.rs`. **No `unsafe`** (crate-wide `#![forbid(unsafe_code)]`).
  - **Prerequisites:** Item 0 (proptest dev-dep).
  - **Tests (division surface pinned):** unit add/sub/mul/shift/bits/bytes/radix; normalization (no trailing-zero limbs); **division mandatory:** (a) single-limb divisor fast path, (b) power-of-2 divisor cross-checked vs `shr`, (c) explicit Knuth-D normalization edge (top divisor limb ≥ 2^63), (d) cross-validate vs `dashu_int::UBig` over ~1000 random `(num_limbs ∈ 1..=100, den_limbs ∈ 1..=num_limbs)` pairs; proptest add comm/assoc, mul comm/assoc + L/R distributivity, `a == (a/b)*b + a%b` for `b != 0`, `(a<<k)>>k == a`; cross-val add/mul/div/rem/cmp vs dashu `UBig`.
  - **Risk:** Knuth-D qhat correction & borrow handling are the classic bug locus; pinned division tests + dashu cross-val mitigate. If any file nears 2000 lines, split with `splitrs`.
- [x] Implement native `BigInt` as sign + `BigUint` magnitude (100 SLOC) (planned 2026-05-28)
  - **Plan ID:** N2 — Native signed `BigInt` + GCD + roots (dispatched only after N1 `done`)
  - **Goal:** `oxinum_int::native::BigInt` (sign + `BigUint` magnitude) with full signed arithmetic, primitive conversions, binary GCD, integer roots — on top of N1's `BigUint`.
  - **Design:** `BigInt { sign: Sign, mag: BigUint }`. **Canonical zero is the ONLY zero: `{ sign: Positive, mag: BigUint::ZERO }`** — enforced in every constructor and after every op (so `+0 == -0`, `Hash`/`Eq`/`Ord` consistent). `abs`/`signum`/`Neg`. `Add`/`Sub`/`Mul`/`Div`/`Rem`/`Neg` (+`*Assign`) for owned & borrowed (Div/Rem truncate toward zero, document sign-of-remainder). `Ord`/`PartialOrd`/`Hash`. `From<u64/i64/u128/i128/u32/i32/usize/isize>`; `TryFrom<&BigInt>`/`TryFrom<&BigUint>` for those primitives with range check → `OxiNumError::Overflow` on out-of-range. **Stein binary GCD** on `BigUint`; `BigInt` gcd via magnitudes. **Newton** `BigUint::sqrt` (floor) + `nth_root` generalized; `BigInt::nth_root` (error on even root of negative).
  - **Files:** `crates/oxinum-int/src/native/{int.rs,ops_int.rs,convert.rs,gcd.rs,roots.rs}`; extend `native/mod.rs` `pub use`; integration test `crates/oxinum-int/tests/native_int.rs`.
  - **Prerequisites:** **N1 complete** (`BigUint`/div/mul/shift). Item 0 (proptest).
  - **Tests:** signed ops proptest comm/assoc/distrib + `a == (a/b)*b + a%b` incl. negatives; cross-val vs `dashu_int::IBig`; `+0 == -0` canonical-zero test; `TryFrom` boundary tests (i64::MIN/MAX, u64::MAX, etc.); GCD properties (`g | a`, `g | b`, `gcd(a, 0) == |a|`) cross-val vs dashu `Gcd`; sqrt invariant `sqrt(n)^2 ≤ n < (sqrt(n)+1)^2` over random n; nth_root invariant.
  - **Risk:** sign/canonical-zero drift; remainder-sign convention must match dashu for cross-val. Mitigations: explicit invariant + `+0==-0` test.
- [x] Implement Karatsuba multiplication (threshold ~32 limbs) (150-200 SLOC) (planned 2026-05-28; covered by N1)
- [x] Implement Toom-Cook-3 multiplication (threshold ~100 limbs) (200-300 SLOC) (planned 2026-05-29; TC1)
  - **Goal:** Third mul tier above Karatsuba; mul() dispatches schoolbook<32 ≤ Karatsuba<TOOM3_THRESHOLD(~100 limbs) ≤ Toom-3. O(n^1.465).
  - **Design:** 3-way limb-block split; evaluate at {0,1,-1,2,∞} (signed intermediates); recurse for w0..w4; **Bodrato interpolation schedule** (exact ÷3 and ÷2 helpers, never general divrem); recompose. Internal to mul.rs.
  - **Files:** native/mul.rs (mul_toom3 + dispatch), native/uint.rs (const TOOM3_THRESHOLD), tests/native_int.rs.
  - **Tests:** cross-val vs Karatsuba at threshold boundary (98..104 limbs) + adversarial limbs (all-u64::MAX, power-of-two, asymmetric, 0/1) + 200 random + proptest.
  - **Risk:** interpolation sign/exact-div bugs — follow Bodrato verbatim; boundary+adversarial cross-val.
- [x] Implement schoolbook division with Knuth Algorithm D (200-250 SLOC) (planned 2026-05-28; covered by N1; division test surface pinned)
- [x] Implement Newton's division for large dividends (150-200 SLOC) (planned 2026-05-29; ND1)
  - **Goal:** Sub-quadratic division for big-by-big; keep single-limb + Knuth-D for small/medium, add Newton-reciprocal path above NEWTON_DIV_THRESHOLD.
  - **Design:** normalize divisor; Newton reciprocal x_{i+1}=x_i*(2-d*x_i) (quadratic, double precision/step); q̂=(u*x)>>shift; **explicit ±1/±2 correction loop**; rem=u-q̂*d. Dispatch in checked_divrem; fall back to Knuth-D.
  - **Files:** native/div.rs (div_newton + dispatch), tests/native_int.rs.
  - **Tests:** Euclidean invariant u==(u/d)*d+u%d with 0≤rem<d (random + just-above-threshold); cross-val vs Knuth-D at boundary; adversarial (d=power-of-two, d one bit below u, u exact multiple of d).
  - **Risk:** quotient off-by-±1–2 — explicit correction loop + Knuth-D boundary cross-val.
- [x] Implement binary GCD algorithm (Stein's algorithm) (80-100 SLOC) (planned 2026-05-28; covered by N2)
- [x] Implement Lehmer's GCD on `BigUint` (replace Stein binary GCD as default) (180-250 SLOC) (planned 2026-05-28)
  - **Plan ID:** N6 — Lehmer's GCD on BigUint (Phase 4)
  - **Goal:** Replace the current Stein binary GCD as the default `oxinum_int::native::gcd` with Lehmer's matrix-step algorithm — asymptotically faster for large multi-limb operands. Preserve Stein as `gcd_binary` for cross-validation and benchmarking.
  - **Design (ultrathink — Lehmer is subtle):** When both `a` and `b` are multi-limb, the quotient sequence is mostly determined by the top words. Algorithm: (1) form `(ah, bh)` = top 64 bits of `(a, b)` (offset such that larger has top bit at 63); (2) inner Euclidean loop on `(ah, bh)` accumulating 2×2 transformation matrix `[[A,B],[C,D]]`; (3) continue while single-precision quotients provably match multi-precision (overflow condition: `(bh - C >= D)` and `(ah - A >= B)`); (4) apply matrix to full `(a, b)`: `(new_a, new_b) = (A*a - B*b, D*b - C*a)`; (5) loop until single-limb, finish single-word. Matrix entries `(A,B,C,D)` in `u64` with `i128` overflow detection. Fallback: one Stein/Euclidean step if inner loop accepts no steps. Crossover heuristic: `min(a.limbs, b.limbs) <= 2` → direct binary.
  - **API:** `pub fn gcd(a: &BigUint, b: &BigUint) -> BigUint` (Lehmer, new default); `pub fn gcd_binary(a: &BigUint, b: &BigUint) -> BigUint` (Stein, preserved); `gcd_int(IBig, IBig)` continues to delegate via `gcd` on magnitudes.
  - **Files:** `crates/oxinum-int/src/native/gcd.rs` (extend — rename current impl to `gcd_binary`, add `gcd_lehmer`, new `gcd` dispatch). New `native/gcd_lehmer.rs` if main file crosses 250 lines. Extend `tests/native_uint.rs` and `tests/native_int.rs` with Lehmer cross-validation blocks.
  - **Prerequisites:** N1 (BigUint with division + shifts + bit_length).
  - **Tests:** `gcd(0,0)==0`, `gcd(a,0)==a`, `gcd(0,b)==b`, `gcd(a,a)==a`; 300 random `(a,b)` with `a.limbs, b.limbs ∈ 1..=64` cross-val `gcd_lehmer == gcd_binary == dashu_int::ubig::gcd`; edges `gcd(2^256, 2^512)`, `gcd(F_n, F_{n+1})==1`, hand-picked remainders; proptest `gcd | a`, `gcd | b`, `gcd >= 1` for non-both-zero.
  - **Risk:** Overflow-detection condition is canonical bug locus. Mitigated by extensive cross-val against binary GCD + dashu as third oracle.
- [x] Implement extended Lehmer GCD, mod_pow/mod_mul/mod_inv, and Montgomery context (planned 2026-05-29)
  - **Plan ID:** P1 — Modular arithmetic + extended Lehmer GCD + Montgomery
  - **Goal:** `gcd_extended(a,b)->(g,x,y)` (Bezout), `mod_pow/mod_mul/mod_inv`, and `MontgomeryContext` for repeated mod-mul under the same odd modulus.
  - **Design (ultrathink):** gcd_extended: half-Lehmer extended variant tracking Bezout coefficients (BigInt) alongside (BigUint) values; same sandwich-bound condition as N6's Lehmer; single Euclidean step fallback below crossover (`min(a.limbs, b.limbs) ≤ 2`). Returns `(BigUint, BigInt, BigInt)`. mod_inv: `(g,x,_)=gcd_extended(a,m)`; if `g!=1` return `None`; else `x mod m`. mod_mul: `(a*b)%m` via N1's divrem. mod_pow: square-and-multiply on exp bits. Montgomery: `R=2^(⌈bits(m)/64⌉·64)`; precompute `r_mod_m`, `r_squared`, `m_prime=(-m^{-1} mod 2^64)` via Hensel lift; REDC for `mont_mul`; `to_mont`/`from_mont`; `ctx.pow` via Montgomery-form ladder. `MontgomeryContext::new(m)` errors on even m via `OxiNumError::Domain`.
  - **Files:** `crates/oxinum-int/src/native/ext_gcd.rs` (~300 LoC), `native/mod_arith.rs` (~400 LoC), `native/montgomery.rs` (~350 LoC). Re-export via `native/mod.rs`. Tests in `crates/oxinum-int/tests/native_mod_arith.rs`.
  - **Prerequisites:** N1 (BigUint mul/divrem/shifts), N6 (Lehmer GCD structure).
  - **Tests:** Bezout identity `a·x+b·y==g` for 200 random pairs; mod_inv round-trip `(a·mod_inv(a,m))%m==1`; Fermat's little theorem for primes {7,13,101,65537}; cross-val mod_pow vs dashu over 200 random; Montgomery vs schoolbook for 100 random odd m; Montgomery rejects even modulus.
  - **Risk:** m_prime Hensel-lift correctness (canonical REDC bug). Mitigated by cross-val + explicit unit tests for m=7,13,65537.
- [x] Implement prime sieve, deterministic Miller-Rabin+BPSW primality, Lucas U/V, prime-swing factorial (planned 2026-05-29)
  - **Plan ID:** P2 — Number theory: sieve + Miller-Rabin + BPSW + Lucas U/V + prime-swing factorial
  - **Goal:** `prime_sieve(limit)`, `is_probably_prime(&BigUint) -> bool`, `factorial(n: u64) -> BigUint`, `lucas_uv(n, p, q, m) -> (BigInt, BigInt)`.
  - **Design (ultrathink):** Sieve: bit-packed Eratosthenes; segmented above 10^8; returns `Vec<u64>`. Miller-Rabin: deterministic witness sets per Sorenson 2017 (up to 3.3×10^24); BPSW (MR base 2 + strong Lucas) above. Jacobi symbol via quadratic reciprocity (~80 LoC). Lucas U/V via binary expansion of n tracking (U_k, V_k, Q^k) triples mod m with halving identities. BPSW: Selfridge-D param selection (`min D∈{5,-7,9,-11,…}` with Jacobi(D|n)=-1). Prime-swing factorial (Luschny): `n! = swing(n) · (n/2)!² · …`; swing(n) via prime sieve + Legendre exponent formula; O(M(n)log²n).
  - **Files:** `crates/oxinum-int/src/native/sieve.rs` (~250 LoC), `native/primality.rs` (~400 LoC), `native/lucas.rs` (~250 LoC), `native/factorial.rs` (~250 LoC). Re-export via `native/mod.rs`. Tests in `crates/oxinum-int/tests/native_number_theory.rs`.
  - **Prerequisites:** P1 (mod_pow for Miller-Rabin).
  - **Tests:** `prime_sieve(100)` = 25 primes; sieve(1000)=168, sieve(10000)=1229; Miller-Rabin matches sieve for n∈[2,10000]; Carmichael composites 561,1105,1729 correctly identified; Mersenne primes 2^p-1 for p∈{7,13,17,19,31,61}; Lucas identity `U_{m+n}=U_m·V_n+U_n·V_m`; factorial(0)=1, factorial(10)=3628800, factorial(20)=2432902008176640000; prime-swing vs naive cross-val for n∈{0..200}.
  - **Risk:** Miller-Rabin witness-set thresholds — table reproduced from Sorenson 2017 with inline citations. BPSW has no known counterexample below 2^64.
- [x] Implement integer square root (Newton's method) (60-80 SLOC) -- dashu provides `sqrt` today (planned 2026-05-28; covered by N2)
- [x] Implement integer nth root (Newton's method generalized) (80-100 SLOC) (planned 2026-05-28; covered by N2)
- [x] Implement bit manipulation: bit_length, trailing_zeros, count_ones, test_bit, set_bit, clear_bit (100 SLOC) (planned 2026-05-28; covered by N1)
- [x] Implement bitwise AND, OR, XOR, NOT, shifts on BigInt/BigUint (150-200 SLOC) (planned 2026-05-29; BW1 — BigUint AND/OR/XOR/shifts already exist from N1; this adds signed two's-complement + NOT + fmt traits)
  - **Goal:** BigInt BitAnd/BitOr/BitXor/Not/Shl/Shr under infinite-precision two's-complement; BigUint Not intentionally absent (unbounded). Plus fmt::{LowerHex,UpperHex,Octal,Binary} on BigUint+BigInt.
  - **Design:** !x=-(x)-1; binary ops via two's-complement limb windows (max(len)+1), apply limb-wise, re-detect sign from top bit, convert back; arithmetic right shift sign-extends (floor div by 2^n). fmt delegates to to_radix(16/8/2) + {:#} alt-prefix + sign.
  - **Files:** native/bitwise.rs (new), native/radix.rs (four fmt impls), native/mod.rs re-export, tests/native_int.rs.
  - **Tests:** !BigInt(0)==-1, !BigInt(5)==-6; (-1)&0xFF==0xFF; (-8)>>1==-4; De Morgan over mixed signs; i128 cross-val in range; {:x}/{:#b}/{:o} formatting + negative-BigInt hex convention.
  - **Risk:** two's-complement sign re-detection — i128 cross-val + De Morgan proptests.
- [x] Implement `TryFrom<BigInt>` for primitive types with overflow detection (80 SLOC) (planned 2026-05-28; covered by N2)

## API Improvements
- [x] Implement `std::ops::{Add, Sub, Mul, Div, Rem, Neg}` for owned and borrowed operands (planned 2026-05-28; covered by N1 (BigUint: Add/Mul/Div/Rem, no Sub by design) + N2 (BigInt: full set incl. Sub/Neg))
- [x] Implement `AddAssign, SubAssign, MulAssign, DivAssign, RemAssign` (planned 2026-05-28; covered by N1 + N2 in parallel with their ops)
- [x] Implement `PartialOrd, Ord, Hash` for BigInt and BigUint (planned 2026-05-28; covered by N1 (BigUint) + N2 (BigInt))
- [x] Implement `num_traits::Zero`, `One`, `Num`, `Signed`, `Unsigned` compatibility (planned 2026-05-28; covered by I1 — feature flag opened, trait impls deferred to follow-up)
- [x] Implement num_traits::{Zero,One,Num,Signed,Unsigned,ConstZero,ConstOne} on native BigUint+BigInt (planned 2026-05-29)
  - **Plan ID:** N7 — num_traits impls (BigUint + BigInt portion)
  - **Goal:** `num_traits::{Zero, One, Num, Signed, Unsigned, ConstZero, ConstOne}` on `native::BigUint` and `native::BigInt`, gated behind `num-traits` feature.
  - **Design:** `crates/oxinum-int/src/native/num_traits_impl.rs` (~300 LoC). BigUint: `Zero`/`One`/`Num`/`Unsigned`/`ConstZero`/`ConstOne`. `Num::from_str_radix` delegates to N1's `from_str_radix`; `FromStrRadixErr = String`. BigInt: `Zero`/`One`/`Num`/`Signed`/`ConstZero`/`ConstOne`. `Signed::abs/signum/is_positive/is_negative` delegate to inherent methods. All gated `#[cfg(feature = "num-traits")]`.
  - **Files:** `crates/oxinum-int/src/native/num_traits_impl.rs`, re-export from `native/mod.rs`.
  - **Prerequisites:** num-traits feature flag (already in Cargo.toml from I1).
  - **Tests:** `BigUint::zero().is_zero()==true`; `BigInt::from(-5).signum()==-BigInt::one()`; `Num::from_str_radix("ff",16)==Ok(BigUint::from(255u64))`; generic sum test `fn sum<T: Zero+Add>(xs: Vec<T>) -> T`.
  - **Risk:** `Num::FromStrRadixErr` type — use `String` as the simplest viable error.
- [x] Implement `serde::Serialize` / `Deserialize` behind `serde` feature (planned 2026-05-28)
  - **Plan ID:** I1 — Integer wrapper polish
  - **Goal:** Close gaps on native `BigInt`/`BigUint`: `serde` feature, two's-complement signed byte serialization, num_traits feature gating.
  - **Design:** `crates/oxinum-int/Cargo.toml` adds `[features]` `serde = ["dep:serde", "oxinum-core/serde"]` and `num-traits = ["dep:num-traits"]` with optional deps. serde: derive `Serialize`/`Deserialize` on `BigUint` (as `Vec<u64>` limbs) and `BigInt` (as `(Sign, BigUint)`); pipeline test for JSON round-trip. Signed bytes (BigInt — already have unsigned on BigUint): `to_signed_bytes_be(&self) -> Vec<u8>` (two's-complement, BE, minimal length, sign-extended), `to_signed_bytes_le`, `from_signed_bytes_be(&[u8]) -> BigInt`, `from_signed_bytes_le`. num_traits feature flag opens the door; impls deferred.
  - **Files:** `crates/oxinum-int/Cargo.toml`, `crates/oxinum-int/src/native/mod.rs` (re-export `bytes_signed::*`), `crates/oxinum-int/src/native/bytes_signed.rs` (new, <300 lines).
  - **Prerequisites:** N1 + N2 (DONE — uses existing `to_bytes_be/le` + `Sign` flip).
  - **Tests:** signed bytes round-trip for `i64::MIN`/`i64::MAX`, `-1`, `0`, `2^200`, `-2^200`; minimal-length encoding (1 → `[1]`, -1 → `[0xFF]`); serde JSON round-trip for BigUint and BigInt.
  - **Risk:** Two's-complement encoding boundary at exact powers of 2 (e.g. `-128` → `[0x80]` not `[0xFF, 0x80]`). Mitigated by explicit unit tests for `-1`, `-128`, `-129`, `-2^63`, `2^63 - 1`.
- [x] Implement rand integration: random_bits, random_in_range, Distribution impls (planned 2026-05-29)
  - **Plan ID:** R2 — rand integration for BigInt/BigUint
  - **Goal:** rand-compatible random generation for native BigUint/BigInt. Required by future randomized Miller-Rabin rounds (cryptographic-strength primality) and RNG-driven number theory.
  - **Design:** Add `rand = { version = "0.9", default-features = false }` to workspace Cargo.toml. `oxinum-int/Cargo.toml` adds `rand = ["dep:rand"]` feature. API: `BigUint::random_bits<R: RngCore>(rng, n_bits: u64) -> Self` (uniform in [0, 2^n_bits)); `BigUint::random_in_range<R: RngCore>(rng, low, high) -> Self` (rejection-sampling, ~2 draws expected); `BigInt::random_in_range`; `impl Distribution<BigUint> for Uniform<&BigUint>`; `impl Distribution<BigUint> for StandardUniform` (256-bit default, documented). All under `#[cfg(feature = "rand")]`.
  - **Files:** `crates/oxinum-int/src/native/rand_impl.rs` (~300 LoC), workspace+crate Cargo.toml, re-export via `native/mod.rs`. Tests in `crates/oxinum-int/tests/native_rand.rs`.
  - **Prerequisites:** none.
  - **Tests:** `random_bits(64) < 2^64` over 1000 trials; `random_in_range(100,200)` in [100,200) over 10000 trials; uniformity smoke 10000 draws in [0,100); determinism with same StdRng seed; BigInt negative range mix.
  - **Risk:** rand 0.9 API changes vs 0.8 — verify at impl time (Distribution::sample, StandardUniform naming).
- [x] Add `BigUint::from_bytes_be`, `from_bytes_le`, `to_bytes_be`, `to_bytes_le` (planned 2026-05-28; covered by N1)
- [x] Add `BigInt::to_signed_bytes_be`, `to_signed_bytes_le`, `from_signed_bytes_be`, `from_signed_bytes_le` (planned 2026-05-28; covered by I1)

## Testing
- [x] Arithmetic identity / commutativity tests (add, mul)
- [x] Commutativity / associativity / distributivity property tests (proptest) (planned 2026-05-28; covered by N1 + N2 — proptest wired by Item 0)
- [x] Division round-trip: (a / b) * b + (a % b) == a
- [x] GCD properties: Bezout identity (a*x + b*y = gcd)
- [x] Primality test against known primes (Mersenne M_17) and Carmichael numbers
- [x] Factorial correctness (20! exact, 100! digit count)
- [x] Fibonacci fast-doubling correctness (F_50 exact)
- [x] Binomial symmetry property tests
- [x] Cross-validate results against dashu-int for regression detection (broader) (planned 2026-05-28; covered by N1 + N2 — random-input cross-val per algorithm)

## Performance
- [x] Benchmark Karatsuba crossover point (schoolbook vs Karatsuba vs Toom-Cook)
- [x] Benchmark modular exponentiation vs dashu baseline
- [x] Benchmark GCD (binary vs Lehmer) for various input sizes
- [x] Profile memory allocation patterns in multiplication chains (planned 2026-06-03)
  - **Goal:** A report bench (`benches/alloc_profile.rs`, `harness=false`, plain `fn main`) that counts allocations in native big-integer multiplication across all three tiers (schoolbook / Karatsuba / Toom-3), for a single multiply and a 16-step chained product, printing every `AllocStats` field.
  - **Design:** file-local `CountingAlloc` `#[global_allocator]` in the bench binary (legal; bench binaries do NOT inherit the library's `#![forbid(unsafe_code)]`). Atomics track alloc/dealloc calls, cumulative bytes, live bytes, peak bytes; helpers `reset()/snapshot()/measure(||…)`. Tier sizes: {8,64,200,1000} limbs (schoolbook <32, Karatsuba 32..~100, Toom-3 ≥100; large Toom-3 case). Also extends `benches/mul.rs` and `benches/div.rs` with competitive baseline groups (oxinum native vs dashu `UBig::from_words` vs num-bigint `from_bytes_le`) at tiers {8,32,100,400} and divrem sizes spanning both algorithms.
  - **Files:** new `crates/oxinum-int/benches/alloc_profile.rs`; edit `crates/oxinum-int/benches/{mul.rs,div.rs}`; `crates/oxinum-int/Cargo.toml` adds `num-bigint = "0.4.6"` (dev-only, Pure Rust, latest stable) and `[[bench]] name="alloc_profile" harness=false`. dashu-int already a normal dep — no dev-dep needed.
  - **Prerequisites:** none (all mul tiers + divrem already implemented).
  - **Tests:** `cargo bench -p oxinum-int --no-run`; alloc_profile binary runs and prints.
  - **Risk:** closure unused-input warnings → use `|bch, _|`; `UBig::from_words` relies on Word=u64 (64-bit targets only — the only supported targets).
- [x] Consider SIMD-accelerated limb arithmetic via `std::simd` (nightly feature gate) (planned 2026-06-03)
  - **Goal:** `native::BigUint` gains public `BitAnd`/`BitOr`/`BitXor` ops (+ assign variants), and `shl_bits`/`shr_bits` gain a SIMD inner kernel. A `simd` Cargo feature opts in; `build.rs` activates `portable_simd` only on nightly, keeping stable CI green with a bit-identical scalar fallback.
  - **Design:** `build.rs` channel-detector emits `cargo:rustc-cfg=oxinum_simd` only when `simd` feature is on **and** compiler is nightly; `#![cfg_attr(oxinum_simd, feature(portable_simd))]` in `lib.rs` (above existing `#![forbid(unsafe_code)]`). New `src/native/simd_ops.rs`: `and_limbs/or_limbs/xor_limbs(a,b)→Vec<u64>` (AND: min-length, OR/XOR: max-length + normalize); `shl_within/shr_within(limbs, bit_offset: u32)` SIMD inner kernels (shl: `out[i]=(a[i]<<s)|(a[i-1]>>(64-s))`; shr: mirror), via two overlapping `Simd<u64,4>` windows + scalar boundary lanes. `#![forbid(unsafe_code)]` stays. `native::BigUint` gains bitwise impls in `bitwise.rs` calling `simd_ops`. `shl_bits`/`shr_bits` delegate sub-limb branch. Note: multiply/add deliberately excluded — `portable_simd` lacks u64→u128 widening multiply; documented.
  - **Files:** new `build.rs`, `src/native/simd_ops.rs`, `benches/bitops.rs`; modify `src/lib.rs`, `src/native/mod.rs`, `src/native/uint.rs`, `src/native/bitwise.rs`, `Cargo.toml`, `TODO.md`.
  - **Prerequisites:** native BigUint bitwise ops (AND/OR/XOR on `BigUint`) don't yet exist → implement as part of this item.
  - **Tests:** AND/OR/XOR vs dashu `UBig` oracle (proptest + edge cases: unequal lengths, `a^a==0`, `a&0==0`, `a|0==a`, normalization, zero). `shl_bits`/`shr_bits` vs dashu `<<`/`>>` proptest. Round-trip `(x<<k)>>k==x`. Same tests run on stable (scalar path) and nightly (SIMD path) — bit-identical results guaranteed.
  - **Risk:** `--all-features` on stable kept silent via cfg-decoupling (gate on `oxinum_simd` cfg, not the `simd` feature); `unexpected_cfgs` silenced by unconditional `rustc-check-cfg`; XOR/AND trailing-zero normalization via explicit `normalize()`; shift boundary handled with scalar fallback for boundary lanes.

## Integration
- [x] Ensure oxinum-rational uses oxinum-int's BigInt/BigUint for numerator/denominator
- [x] Ensure oxinum-float uses oxinum-int's BigUint for mantissa storage
- [x] Verify API compatibility with SciRS2 integer needs (matrix determinants, polynomial roots) (verified 2026-06-03)
  - **Delivered:** `tests/scirs2_int_compat.rs` — 20 tests proving `IBig`, `UBig`, `is_prime`, `ibig_from_radix`, `factorial`, `binomial`, `Gcd`, `mod_pow` match SciRS2's exact call sites and return correct results; includes Fermat's little theorem, Carmichael composites, and the full `mod_pow` helper path.
- [x] Provide re-export path through oxinum facade crate
