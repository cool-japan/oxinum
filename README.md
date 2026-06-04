# OxiNum

**Pure-Rust arbitrary-precision math — the COOLJAPAN GMP/MPFR-free replacement.**

[![crates.io](https://img.shields.io/crates/v/oxinum.svg)](https://crates.io/crates/oxinum)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

OxiNum replaces `rug` / GMP / MPFR with a 100 % Pure Rust, Apache-2.0
licensed implementation built on top of the [`dashu`](https://crates.io/crates/dashu)
family of crates, augmented with a fully native Pure-Rust arithmetic core.

**Version 0.1.1 — Released 2026-06-04**

---

## Security Warning — NOT constant-time

> **OxiNum is NOT constant-time.** This is a general-purpose numerics library,
> not a cryptographic bignum replacement. Secret-dependent computations (private
> keys, DH exponents, blinding factors, …) must never be routed through OxiNum.
> Use a constant-time library (e.g. `crypto-bigint`) for that purpose.

---

## Sub-crates

| Crate              | Description                                                         |
|--------------------|---------------------------------------------------------------------|
| `oxinum-core`      | Core traits (`OxiNumTrait`, `OxiSigned`), `OxiNumError`, `RoundingMode` |
| `oxinum-int`       | Arbitrary-precision integers — dashu re-exports + full native BigUint/BigInt |
| `oxinum-float`     | Arbitrary-precision floats — dashu re-exports + native BigFloat with transcendentals |
| `oxinum-rational`  | Exact rationals — dashu re-exports + native BigRational with continued fractions |
| `oxinum-complex`   | Arbitrary-precision complex — `CBig` over `DBig` + native `BigComplex` over `BigFloat` |
| `oxinum`           | Facade crate — prelude, constants (π, e, ln 2), parse helpers       |

---

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
oxinum = "0.1.1"
```

### Integer arithmetic

```rust
use oxinum::{UBig, IBig};

let a = UBig::from(u128::MAX);
let b = UBig::from(u128::MAX);
let c = a + b;                      // 2 * u128::MAX — no overflow
println!("{c}");

let d = IBig::from(-42i64);
println!("{d}");                    // -42
```

### Native arbitrary-precision integers

```rust
use oxinum_int::native::{BigUint, BigInt};

let a = BigUint::from_u64(u64::MAX);
let b = a.clone() * &a;            // Karatsuba/Toom-3 multiplication
let (q, r) = b.divrem(&BigUint::from_u64(1_000_000_007));
println!("remainder = {r}");

// Number theory
use oxinum_int::native::primality::is_probably_prime;
assert!(is_probably_prime(&BigUint::from_u64(999_999_999_999_999_877)));
```

### Arbitrary-precision floats

```rust
use oxinum::FBig;
use dashu_float::round::mode;

// Binary float with 200-bit precision
type F200 = FBig<mode::HalfAway, 2>;
let pi: F200 = F200::from(3u32);
println!("{pi}");
```

### Native high-precision floats

```rust
use oxinum_float::native::{BigFloat, RoundingMode, pi, e_const};

// 200-bit precision π via binary splitting
let pi_200 = pi(200, RoundingMode::HalfEven);

// Transcendental functions
let x = BigFloat::from_f64(0.5, 64, RoundingMode::HalfEven).unwrap();
let sin_x = x.sin(64, RoundingMode::HalfEven);
let cos_x = x.cos(64, RoundingMode::HalfEven);
```

### Decimal floats

```rust
use oxinum::DBig;

let d: DBig = DBig::from(31415926u32);
println!("{d}");                    // 31415926
```

### Rational numbers

```rust
use oxinum::{RBig, Relaxed};

let r = RBig::from(3u32);
println!("{r}");                    // 3
```

### Native exact rationals with continued fractions

```rust
use oxinum_rational::native::BigRational;
use oxinum_int::native::{BigInt, BigUint};

// 415/93 = [4; 2, 6, 7]
let r = BigRational::from_parts(BigInt::from(415i64), BigUint::from_u64(93)).unwrap();
let cf = r.continued_fraction();
assert_eq!(cf, vec![BigInt::from(4i64), BigInt::from(2i64),
                     BigInt::from(6i64), BigInt::from(7i64)]);

// Best rational approximation to π with denominator ≤ 113
let pi_approx = BigRational::best_rational_approximation(&r, &BigUint::from_u64(113));
```

### Complex numbers

```rust
use oxinum::CBig;

// |3 + 4i| = 5 at 20 significant digits
let z = CBig::from_f64(3.0, 4.0).expect("finite parts");
assert_eq!(z.abs(20).expect("magnitude").to_string(), "5");

// exp(iπ) ≈ −1 (Euler's identity)
let i_pi = CBig::from_imag(oxinum::constants::pi(40)); // 0 + π·i
let euler = i_pi.exp(30).expect("exp");
assert!(euler.re().to_string().starts_with("-0.99999999"));
```

### Native binary-base complex numbers

```rust
use oxinum::native::{BigComplex, BigFloat, RoundingMode};

let re = BigFloat::from_i64(3, 64, RoundingMode::HalfEven);
let im = BigFloat::from_i64(4, 64, RoundingMode::HalfEven);
let z = BigComplex::new(re, im);
assert_eq!(z.norm_sqr().to_f64(), 25.0); // |3 + 4i|^2 = 25
```

---

## Feature flags

| Feature      | Default | Description                                          |
|--------------|---------|------------------------------------------------------|
| `pure`       | yes     | Enables oxinum-int / float / rational / complex sub-crates |
| `macros`     | no      | Enables `dashu-macros` literal macros                |
| `serde`      | no      | Serde serialize/deserialize for all native types     |
| `num-traits` | no      | `num-traits` compatibility (Zero, One, Signed, etc.) |
| `rand`       | no      | Random number generation via `rand` crate            |

---

## Implementation status (v0.1.1)

### oxinum-core
- [x] `OxiNumTrait`, `OxiSigned`, `OxiNumError` / `OxiNumResult`
- [x] `RoundingMode` enum (HalfEven, HalfAway, Floor, Ceil, Truncate)
- [x] `Sign` re-export from `dashu-base`
- [x] serde feature gate

### oxinum-int
- [x] `UBig`, `IBig` dashu re-exports
- [x] Native `BigUint` — little-endian `Vec<u64>` limbs, canonical zero = empty Vec
  - [x] Schoolbook multiplication (< 32 limbs)
  - [x] Karatsuba multiplication (32–100 limbs)
  - [x] Toom-Cook-3 multiplication (≥ 100 limbs)
  - [x] Knuth Algorithm D division with qhat correction
  - [x] Binary GCD (Stein's algorithm) + Lehmer GCD
  - [x] Newton integer sqrt and nth-root
  - [x] Bitwise operations, radix I/O (2–36), byte conversions
- [x] Native `BigInt` — signed, canonical zero invariant (`+0 == -0`)
- [x] Number theory: Miller-Rabin + BPSW primality (Jacobi + strong Lucas)
- [x] Sieve of Eratosthenes prime generation
- [x] Montgomery multiplication context for modular exponentiation
- [x] Extended GCD (Bezout coefficients)
- [x] serde, rand, num-traits features

### oxinum-float
- [x] `FBig`, `DBig` dashu re-exports
- [x] Native `BigFloat` — binary-base, explicit precision, post-op rounding
  - [x] `sqrt`, `exp`, `ln`, `pow`
  - [x] `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`
  - [x] High-precision constants: π, e, ln 2 (binary-splitting / AGM)
  - [x] `FloatContext` builder pattern for precision management
  - [x] `total_cmp` for sort-stable total order
- [x] serde, num-traits features

### oxinum-rational
- [x] `RBig`, `Relaxed` dashu re-exports
- [x] Native `BigRational` — automatic GCD reduction, canonical sign/zero
  - [x] Continued-fraction expansion and reconstruction
  - [x] Convergents sequence
  - [x] Semiconvergent best-rational-approximation (bounded denominator)
  - [x] Cross-domain conversions: `BigFloat` ↔ `BigRational` ↔ `BigInt`
- [x] serde, num-traits features

### oxinum-complex
- [x] `CBig` — decimal-backed complex (`re`/`im` each a `DBig`)
  - [x] Construction (`new`/`from_parts`/`from_real`/`from_imag`/`from_f64`), `zero`/`one`/`i`
  - [x] Arithmetic (`+`/`−`/`×`/`÷`, neg), `conj`, `norm_sqr`
  - [x] `abs`, `arg`, `exp`, `ln`, `sqrt`, `pow` (principal branches)
  - [x] Complex `sin`/`cos`/`tan`/`sinh`/`cosh`/`tanh`
  - [x] serde, num-traits features
- [x] `native::BigComplex` — ground-up binary complex over `native::BigFloat`
  - [x] Same surface with explicit precision / `RoundingMode` control

### oxinum (facade)
- [x] Prelude re-exporting all public types
- [x] Constants module (π, e, ln 2)
- [x] Parse helpers
- [x] Integration tests: Machin π formula, exact rational determinant
- [x] Examples: `high_precision_pi`, `exact_rational_linear_solve`

### Quality
- [x] 1749 tests across the workspace, 0 skipped
- [x] Zero clippy warnings (`-D warnings`, all-targets, all-features)
- [x] Zero rustdoc warnings
- [x] `cargo fmt` clean
- [x] `deny.toml` banning GMP/MPFR/rug crates tree-wide
- [x] `Dockerfile.ffi-audit` + `scripts/ffi-audit.sh` — FFI-free verification
- [x] Criterion benchmark harnesses for mul/div/factorial/primality/transcendentals/complex
- [x] Property-based tests with proptest across all arithmetic laws
- [x] ~24 000 lines of Rust (137 source files)

---

## License

Apache-2.0

Copyright 2026 COOLJAPAN OU (Team Kitasan)
