# oxinum-int — Arbitrary-precision integers for OxiNum

[![Crates.io](https://img.shields.io/crates/v/oxinum-int.svg)](https://crates.io/crates/oxinum-int)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`oxinum-int` is the big-integer layer of the OxiNum arbitrary-precision numeric tower. It provides the `BigInt` / `BigUint` type aliases over `dashu-int`'s `IBig` / `UBig`, together with a focused library of number-theory routines: factorial, Fibonacci (fast doubling), Lucas numbers, binomial coefficients, modular exponentiation, Miller–Rabin primality testing, next-prime search, extended GCD, and arbitrary-radix string conversion.

In addition to the `dashu`-backed aliases, the crate ships a ground-up, **Pure Rust** native big-integer implementation in the [`native`](#native-module) module — a little-endian `Vec<u64>`-limb `BigUint`/`BigInt` with explicit Karatsuba/Newton thresholds, Montgomery multiplication, a prime sieve, and more. The whole crate is `#![forbid(unsafe_code)]` and free of any GMP/MPFR or other C/C++/Fortran dependency.

## Installation

```toml
[dependencies]
oxinum-int = "0.1.0"

# Optional capabilities:
oxinum-int = { version = "0.1.0", features = ["serde", "num-traits", "rand"] }
```

## Quick Start

```rust
use oxinum_int::{factorial, fibonacci, binomial, is_prime, mod_pow, BigUint};

// Number theory over arbitrary-precision integers.
assert_eq!(factorial(20).to_string(), "2432902008176640000");
assert_eq!(fibonacci(50).to_string(), "12586269025");
assert_eq!(binomial(10, 3), BigUint::from(120u32));

// Probabilistic primality (deterministic for small inputs).
assert!(is_prime(&BigUint::from(131071u32), 0));   // Mersenne prime M17
assert!(!is_prime(&BigUint::from(8_388_607u32), 0)); // M23 is composite

// Modular exponentiation: 2^10 mod 1000 = 24
let r = mod_pow(&BigUint::from(2u32), &BigUint::from(10u32), &BigUint::from(1000u32))?;
assert_eq!(r, BigUint::from(24u32));
# Ok::<(), oxinum_int::OxiNumError>(())
```

## API Overview

### Type aliases & core re-exports

| Item | Description |
|------|-------------|
| `BigUint` | Alias for `dashu_int::UBig` — arbitrary-precision **unsigned** integer. |
| `BigInt` | Alias for `dashu_int::IBig` — arbitrary-precision **signed** integer. |
| `UBig`, `IBig` | The underlying `dashu-int` types, re-exported. |
| `Gcd` | The `dashu_int::ops::Gcd` operation trait, re-exported so callers can write `.gcd(&other)`. |
| `Sign` | Re-exported from `oxinum-core` (`Sign::Positive` / `Sign::Negative`). |
| `OxiNumError`, `OxiNumResult` | Re-exported from `oxinum-core`. |

### Number-theory functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `factorial` | `fn(u32) -> UBig` | `n!` via a balanced product tree. |
| `fibonacci` | `fn(u32) -> UBig` | `F(n)` via the fast-doubling method (`O(log n)` multiplications). |
| `lucas` | `fn(u32) -> UBig` | Lucas number `L(n)` (`L(0) = 2, L(1) = 1`). |
| `binomial` | `fn(u32, u32) -> UBig` | Binomial coefficient `C(n, k)` via the multiplicative formula (returns 0 if `k > n`). |
| `extended_gcd` | `fn(&IBig, &IBig) -> (IBig, IBig, IBig)` | Returns `(gcd, x, y)` with `a·x + b·y = gcd`. |
| `mod_pow` | `fn(&UBig, &UBig, &UBig) -> OxiNumResult<UBig>` | `base^exp mod modulus` via binary exponentiation. Errors with `DivByZero` if `modulus == 0`. |
| `is_prime` | `fn(&UBig, u32) -> bool` | Miller–Rabin primality test. `witnesses = 0` uses a deterministic witness set (correct to ≈ 3.3 × 10²⁴) after small-prime trial division. |
| `next_prime` | `fn(&UBig) -> UBig` | Smallest prime strictly greater than `n`. |

### Radix conversion & predicates

Because of the orphan rule, `FromRadix` / `ToRadix` are exposed as free functions over the foreign `UBig` / `IBig` types:

| Function | Signature | Description |
|----------|-----------|-------------|
| `ubig_from_radix` | `fn(&str, u32) -> OxiNumResult<UBig>` | Parse a `UBig` in radix `2..=36`. |
| `ibig_from_radix` | `fn(&str, u32) -> OxiNumResult<IBig>` | Parse an `IBig` in radix `2..=36`. |
| `ubig_to_radix` | `fn(&UBig, u32) -> OxiNumResult<String>` | Format a `UBig` in radix `2..=36`. |
| `ibig_to_radix` | `fn(&IBig, u32) -> OxiNumResult<String>` | Format an `IBig` in radix `2..=36`. |
| `ubig_is_zero` / `ubig_is_one` | `fn(&UBig) -> bool` | Zero / one predicates for `UBig`. |
| `ibig_is_zero` / `ibig_is_one` | `fn(&IBig) -> bool` | Zero / one predicates for `IBig`. |
| `ibig_signum` | `fn(&IBig) -> Sign` | Sign of an `IBig` (zero is `Positive`). |
| `ibig_abs` | `fn(&IBig) -> IBig` | Absolute value of an `IBig`. |

All radix functions return `OxiNumError::InvalidRadix` when the radix is outside `2..=36`, and `OxiNumError::Parse` on invalid digits.

### `native` module

A ground-up Pure Rust integer core, intentionally **not** re-exported at the crate root (to avoid clashing with the `dashu`-backed `BigUint`/`BigInt` aliases). Access it as `oxinum_int::native::*`.

| Item | Kind | Description |
|------|------|-------------|
| `BigUint` | struct | Little-endian `Vec<u64>`-limb unsigned big integer (normalized, canonical empty-vec zero). |
| `BigInt` | struct | Signed wrapper around the native `BigUint`. |
| `factorial` | fn | Native factorial. |
| `gcd`, `gcd_binary`, `gcd_int` | fn | GCD variants (binary/Stein and signed). |
| `gcd_extended`, `mod_inv` | fn | Extended GCD and modular inverse. |
| `mod_mul`, `mod_pow` | fn | Native modular multiply / exponentiation. |
| `MontgomeryContext` | struct | Montgomery-form modular multiplication context. |
| `divrem`, `checked_divrem`, `divrem_int` | fn | Quotient/remainder division primitives. |
| `is_probably_prime` | fn | Native Miller–Rabin test. |
| `prime_sieve` | fn | Sieve of primes. |
| `KARATSUBA_THRESHOLD`, `NEWTON_DIV_THRESHOLD` | const | Algorithm cross-over thresholds. |
| `BigUintBits` | struct | *(feature `rand`)* uniform random-bit generation support. |
| `ParseBigIntError`, `ParseBigUintError` | struct | *(feature `num-traits`)* parse-error types. |

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | off | `serde` derives (enables `oxinum-core/serde`). |
| `num-traits` | off | `num-traits` integration for the native types. |
| `rand` | off | Random big-integer generation for the native types. |

## Cross-references

- [`oxinum-core`](https://crates.io/crates/oxinum-core) — shared traits, errors, and `RoundingMode`.
- [`oxinum-float`](https://crates.io/crates/oxinum-float) — arbitrary-precision floats / decimals.
- [`oxinum-rational`](https://crates.io/crates/oxinum-rational) — exact rationals (built on the integer core).
- [`oxinum`](https://crates.io/crates/oxinum) — the façade re-exporting all number-theory functions and types.

## License

Apache-2.0 — COOLJAPAN OU (Team Kitasan)
