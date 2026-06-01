//! Primality testing for [`BigUint`].
//!
//! # Algorithm
//!
//! [`is_probably_prime`] uses deterministic Miller-Rabin with witness sets
//! whose correctness is proven for specific bounds:
//!
//! - For `n < 3,215,031,751`: witnesses `{2, 3, 5, 7}` — deterministic.
//!   (Pomerance, Selfridge, Wagstaff 1980.)
//! - For `n < 3,317,044,064,679,887,385,961,981` (≈3.3×10²⁴):
//!   witness set from Sorenson & Webster (2017) Journal of Number Theory.
//!   Each sub-range has a proven smallest composite pseudoprime.
//! - For `n ≥ 3.3×10²⁴` (does not fit in u128): uses BPSW
//!   (Miller-Rabin base-2 + strong Lucas with Selfridge parameters).
//!   No counterexample to BPSW is known, and it has been verified for all
//!   n below 2^64.
//!
//! # References
//!
//! - G. Jaeschke, "On strong pseudoprimes to several bases", *Mathematics
//!   of Computation* 61 (1993), pp. 915–926.
//! - J. Sorenson & J. Webster, "Strong pseudoprimes to twelve prime bases",
//!   *Mathematics of Computation* 86 (2017), pp. 985–1003.
//! - C. Pomerance, J. Selfridge, S. Wagstaff, "The pseudoprimes to 25×10⁹",
//!   *Mathematics of Computation* 35 (1980), pp. 1003–1026.
//! - R. Baillie & S. Wagstaff, "Lucas Pseudoprimes", *Mathematics of
//!   Computation* 35 (1980), pp. 1391–1417.
//! - R. Crandall & C. Pomerance, *Prime Numbers: A Computational Perspective*,
//!   2nd ed., Springer 2005, §3.1–3.3.

use super::lucas::is_odd;
use super::mod_arith::{mod_mul, mod_pow};
use super::uint::BigUint;
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// Miller-Rabin internal helpers
// ---------------------------------------------------------------------------

/// Write `n_minus_1 = 2^s * d` with `d` odd. Returns `(d, s)`.
fn factor_out_two(n_minus_1: &BigUint) -> (BigUint, u64) {
    let s = n_minus_1.trailing_zeros();
    let d = n_minus_1.shr_bits(s);
    (d, s)
}

/// Miller-Rabin strong pseudoprime test for witness `a`.
///
/// Returns `Ok(true)` if `n` is probably prime (strong pseudoprime to base `a`),
/// or `Ok(false)` if `n` is definitely composite. Returns `Err` on internal
/// arithmetic error (only if `n == 0`, which the caller prevents).
fn miller_rabin_witness(n: &BigUint, d: &BigUint, s: u64, a: u64) -> OxiNumResult<bool> {
    // a_big = a mod n (a is a small witness, << n for interesting cases)
    let a_big = BigUint::from(a);
    let n_minus_1 = n
        .checked_sub(&BigUint::one())
        .ok_or_else(|| OxiNumError::Domain("miller_rabin_witness: n must be > 1".into()))?;

    // x = a^d mod n
    let mut x = mod_pow(&a_big, d, n)?;

    if x == BigUint::one() || x == n_minus_1 {
        return Ok(true); // probably prime
    }

    // Square up to s-1 times.
    for _ in 0..s.saturating_sub(1) {
        x = mod_pow(&x, &BigUint::from(2u64), n)?;
        if x == n_minus_1 {
            return Ok(true); // probably prime
        }
    }

    Ok(false) // definitely composite
}

// ---------------------------------------------------------------------------
// Threshold-based witness selection
// ---------------------------------------------------------------------------

/// Convert a `BigUint` to `u128` if it fits; otherwise return `None`.
fn biguint_to_u128(n: &BigUint) -> Option<u128> {
    let limbs = n.as_limbs();
    match limbs.len() {
        0 => Some(0),
        1 => Some(limbs[0] as u128),
        2 => Some(((limbs[1] as u128) << 64) | (limbs[0] as u128)),
        _ => None, // doesn't fit in u128
    }
}

// Deterministic witness sets sourced from the literature.
// Each range has a proven least composite strong pseudoprime (Sorenson 2017).
static WITNESSES_2: &[u64] = &[2];
static WITNESSES_3: &[u64] = &[2, 3];
static WITNESSES_4: &[u64] = &[2, 3, 5];
static WITNESSES_5: &[u64] = &[2, 3, 5, 7];
static WITNESSES_6: &[u64] = &[2, 3, 5, 7, 11];
static WITNESSES_7: &[u64] = &[2, 3, 5, 7, 11, 13];
static WITNESSES_8: &[u64] = &[2, 3, 5, 7, 11, 13, 17];
static WITNESSES_9: &[u64] = &[2, 3, 5, 7, 11, 13, 17, 19, 23];
static WITNESSES_12: &[u64] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

/// Select the deterministic witness set for `n128` based on proven bounds.
///
/// These bounds cover n up to 3,317,044,064,679,887,385,961,981 (≈3.3×10²⁴).
/// For n ≥ this threshold, use BPSW instead.
fn select_witnesses(n128: u128) -> Option<&'static [u64]> {
    // Thresholds from Jaeschke 1993 and Sorenson & Webster 2017.
    if n128 < 2_047 {
        Some(WITNESSES_2)
    } else if n128 < 1_373_653 {
        Some(WITNESSES_3)
    } else if n128 < 25_326_001 {
        Some(WITNESSES_4)
    } else if n128 < 3_215_031_751 {
        Some(WITNESSES_5)
    } else if n128 < 2_152_302_898_747 {
        Some(WITNESSES_6)
    } else if n128 < 3_474_749_660_383 {
        Some(WITNESSES_7)
    } else if n128 < 341_550_071_728_321 {
        Some(WITNESSES_8)
    } else if n128 < 3_825_123_056_546_413_051 {
        Some(WITNESSES_9)
    } else if n128 < 318_665_857_834_031_151_167_461 {
        Some(WITNESSES_12)
    } else if n128 < 3_317_044_064_679_887_385_961_981 {
        // Sorenson & Webster 2017, Theorem 3: {2,3,5,7,11,13,17,19,23,29,31,37,41}
        // is deterministic for all n < 3,317,044,064,679,887,385,961,981.
        Some(&[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41])
    } else {
        // n >= 3.3e24: use BPSW
        None
    }
}

// ---------------------------------------------------------------------------
// Jacobi symbol
// ---------------------------------------------------------------------------

/// Compute the Jacobi symbol `(a/n)` for small integer `a` and odd positive `n > 1`.
///
/// Returns 0 if `gcd(a, n) > 1`, 1 or -1 otherwise. Uses the iterative
/// algorithm based on quadratic reciprocity (Crandall & Pomerance 2005, §2.3).
///
/// `n` must be odd and greater than 1. Passing even or zero `n` returns 0.
pub(crate) fn jacobi(a: i64, n: &BigUint) -> i32 {
    if n.is_zero() || n == &BigUint::one() {
        return 0;
    }

    // n must be odd
    if !is_odd(n) {
        return 0;
    }

    // Handle negative a:
    // (−a / n) = (−1 / n) * (a / n)
    // (−1 / n) = (−1)^((n−1)/2) = 1 if n ≡ 1 (mod 4), −1 if n ≡ 3 (mod 4).
    let mut result: i32 = 1;
    let mut a = a;
    if a < 0 {
        a = -a;
        // n mod 4: check LSB and second bit
        let n_mod4 = n.as_limbs().first().copied().unwrap_or(0) & 3;
        if n_mod4 == 3 {
            result = -result;
        }
    }

    if a == 0 {
        // (0 / n) = 0 for n > 1
        return 0;
    }

    // Now a > 0. We compute jacobi(a, n) iteratively.
    // Reduce a mod n to start (for small a, a < n typically in our usage).
    // We convert n to u64 only if it fits, otherwise we know a < n already
    // since a is i64 and n is BigUint.
    let n_limbs = n.as_limbs();
    let mut a_reduced: u64 = if n_limbs.len() == 1 {
        a as u64 % n_limbs[0]
    } else {
        // n > 2^64 >> a (since a fits in i64), so a mod n = a
        a as u64
    };

    // n_val tracks the current modulus. For large n we keep a BigUint.
    // But since our a starts small and each step reduces, we can track
    // both as u64 after the initial reduction (since a < n always shrinks n
    // enough eventually). We use a 2-phase approach:
    // Phase 1: while n is big (> 1 limb), a is u64 < n, so we work with
    //          a as u64 and n as BigUint. Each QR step sets new_a = n mod a
    //          (which gives a u64 result since a is u64), then swaps.
    // Phase 2: both fit in u64.

    // Phase 1: big n
    let mut n_big = n.clone();

    // Factor out 2s from a_reduced
    if a_reduced == 0 {
        return 0;
    }

    let s2 = a_reduced.trailing_zeros();
    a_reduced >>= s2;

    if s2 > 0 {
        // (2^s2 / n): jacobi(2, n)^s2
        // jacobi(2, n) = 1 if n ≡ 1 or 7 (mod 8), −1 if n ≡ 3 or 5 (mod 8)
        let n_mod8 = n_big.as_limbs().first().copied().unwrap_or(0) & 7;
        let sym2: i32 = if n_mod8 == 1 || n_mod8 == 7 { 1 } else { -1 };
        // sym2^s2: negative only if s2 is odd and sym2 == -1
        if sym2 == -1 && s2 % 2 == 1 {
            result = -result;
        }
    }

    if a_reduced == 1 {
        return result;
    }

    // Iterative Jacobi via quadratic reciprocity
    // Invariant: jacobi(original_a, original_n) = result * jacobi(a_reduced, n_big)
    // where a_reduced is odd, 3 <= a_reduced < n_big.
    loop {
        // Apply QR: jacobi(a, n) = jacobi(n mod a, a) * flip
        // where flip = -1 if both a ≡ 3 (mod 4) AND n ≡ 3 (mod 4).
        let a_mod4 = a_reduced & 3;
        let n_mod4 = n_big.as_limbs().first().copied().unwrap_or(0) & 3;
        if a_mod4 == 3 && n_mod4 == 3 {
            result = -result;
        }

        // new_a = n_big mod a_reduced (a_reduced fits in u64)
        let a_bu = BigUint::from(a_reduced);
        let (_, rem) = super::div::divrem(&n_big, &a_bu);
        // rem fits in u64 since divisor a_reduced is u64
        let new_a_raw = rem.as_limbs().first().copied().unwrap_or(0);

        n_big = a_bu; // now n_big = old a_reduced (fits in u64)
        a_reduced = new_a_raw;

        if a_reduced == 0 {
            // gcd(original_a, original_n) > 1
            return 0;
        }

        // Factor out 2s from new a
        let s2 = a_reduced.trailing_zeros();
        a_reduced >>= s2;

        if s2 > 0 {
            // jacobi(2, n_big)^s2
            let nb_mod8 = n_big.as_limbs().first().copied().unwrap_or(0) & 7;
            let sym2: i32 = if nb_mod8 == 1 || nb_mod8 == 7 { 1 } else { -1 };
            if sym2 == -1 && s2 % 2 == 1 {
                result = -result;
            }
        }

        if a_reduced == 1 {
            return result;
        }

        // At this point n_big <= old a_reduced (a u64), so n_big fits in u64 too.
        // We can stay in this loop since n_big is now a small BigUint.
    }
}

// ---------------------------------------------------------------------------
// Selfridge parameters
// ---------------------------------------------------------------------------

/// Find Selfridge parameters (D, P=1, Q=(1−D)/4) for the strong Lucas test.
///
/// Searches D = 5, −7, 9, −11, ... until `jacobi(D, n) == −1`.
///
/// Returns `None` if `n` is a perfect square (D-search would not terminate),
/// or if `jacobi(D, n) == 0` (meaning `gcd(D, n) > 1`, hence composite).
///
/// # Preconditions
///
/// `n` must be odd and greater than 3. The caller is responsible for filtering
/// small n, even n, and perfect-square n.
pub(crate) fn selfridge_params(n: &BigUint) -> Option<(i64, i64, i64)> {
    // Perfect-square guard: if n = k^2 then jacobi(D, n) is never -1 for any D,
    // causing an infinite loop. Check first.
    let sqrt_n = n.sqrt();
    let sq = &sqrt_n * &sqrt_n;
    if &sq == n {
        return None; // perfect square => composite
    }

    // D-search: 5, −7, 9, −11, 13, −15, ...
    // Step k (0-based): d_abs = 5 + 2k, sign = (−1)^k
    let mut step: i64 = 0;
    loop {
        let d_abs: i64 = 5 + 2 * step;
        let d_val: i64 = if step % 2 == 0 { d_abs } else { -d_abs };

        let j = jacobi(d_val, n);
        if j == -1 {
            // P = 1, Q = (1 − D) / 4
            let q: i64 = (1 - d_val) / 4;
            return Some((d_val, 1, q));
        }
        if j == 0 {
            // gcd(|D|, n) > 1. Two sub-cases:
            //   (a) n divides |D|: n is a small prime that happens to equal a
            //       multiple of the current |D|. Skip to the next D — this D
            //       is degenerate (discriminant vanishes mod n) and yields no
            //       information. This handles n=5 (D=5), n=11 (D=-11), etc.
            //   (b) |D| < n and gcd(D,n) > 1: gcd is a proper non-trivial
            //       factor of n, so n is composite.
            let d_abs = d_val.unsigned_abs();
            let n_limbs = n.as_limbs();
            // Check if n (a single limb) divides d_abs
            let n_divides_d = n_limbs.len() == 1 && d_abs % n_limbs[0] == 0;
            if !n_divides_d {
                // gcd(|D|, n) is a proper factor of n → composite
                return None;
            }
            // else: n | |D|, this D is degenerate — continue to the next D
        }

        step += 1;

        // Safety guard: for non-square n, the search always terminates quickly
        // (usually within the first few steps). If we've gone far, something is wrong.
        if step > 500 {
            // Treat as composite / unknown
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// Strong Lucas probable prime test
// ---------------------------------------------------------------------------

/// Strong Lucas probable prime test with given parameters P, Q.
///
/// Computes `n + 1 = 2^s * d` (d odd), then uses [`lucas_uv`] to evaluate
/// `(U_d, V_d) mod n`. Returns `true` if n is a strong Lucas probable prime
/// for these parameters (i.e., `U_d ≡ 0` or some `V_{d·2^r} ≡ 0`).
///
/// # Preconditions
///
/// `n` must be odd and at least 5. The caller ensures this.
fn strong_lucas_probable_prime(n: &BigUint, p: i64, q: i64) -> bool {
    // n + 1 = 2^s * d, d odd
    let n_plus_1 = BigUint::add_ref(n, &BigUint::one());
    let (d, s) = factor_out_two(&n_plus_1);

    // Compute (U_d mod n, V_d mod n) via the binary Lucas ladder.
    let uv = match super::lucas::lucas_uv(&d, p, q, n) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let (u_d, mut v_r) = uv;

    // Condition 1: U_d ≡ 0 (mod n)
    if u_d.is_zero() {
        return true;
    }

    // Compute Q^d mod n independently (lucas_uv does not return Q^n).
    // We need Q^d to apply the doubling identity V_{2k} = V_k^2 − 2·Q^k.
    // Reduce Q to [0, n) first.
    let q_reduced = if q >= 0 {
        let bq = BigUint::from(q as u64);
        let (_, rem) = super::div::divrem(&bq, n);
        rem
    } else {
        // q < 0: compute n - (|q| mod n)
        let bq = BigUint::from(q.unsigned_abs());
        let (_, rem) = super::div::divrem(&bq, n);
        if rem.is_zero() {
            BigUint::zero()
        } else {
            n.checked_sub(&rem).unwrap_or_else(BigUint::zero)
        }
    };

    // Q^d mod n
    let mut qk = match mod_pow(&q_reduced, &d, n) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Condition 2: V_{d·2^r} ≡ 0 (mod n) for some r in 0..s-1
    // Check r=0 first (V_d), then double up to r=s-1.
    for _r in 0..s {
        // Check V_{d·2^r} ≡ 0 (mod n)
        if v_r.is_zero() {
            return true;
        }

        if _r == s - 1 {
            break; // No need to compute the next doubling
        }

        // Doubling: V_{2k} = V_k^2 − 2·Q^k (mod n)
        let v_sq = match mod_mul(&v_r, &v_r, n) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let two_qk = match mod_mul(&BigUint::from(2u64), &qk, n) {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Subtraction mod n
        v_r = if v_sq >= two_qk {
            v_sq.checked_sub(&two_qk).unwrap_or_else(BigUint::zero)
        } else {
            // v_sq < two_qk: result = (v_sq + n - two_qk)
            let sum = BigUint::add_ref(&v_sq, n);
            sum.checked_sub(&two_qk).unwrap_or_else(BigUint::zero)
        };

        // Q^{2k} = (Q^k)^2 (mod n)
        qk = match mod_mul(&qk, &qk, n) {
            Ok(v) => v,
            Err(_) => return false,
        };
    }

    false // composite
}

// ---------------------------------------------------------------------------
// BPSW composite test
// ---------------------------------------------------------------------------

/// Baillie-PSW probable prime test: Miller-Rabin base 2 + strong Lucas.
///
/// This is `pub(crate)` so that the in-module tests can directly validate
/// the implementation against ground truth on small n.
///
/// Returns `true` if `n` is a BPSW probable prime (no BPSW counterexample
/// is known to date), `false` if `n` is definitely composite.
///
/// # Preconditions
///
/// `n` must be an odd integer ≥ 5. The caller is responsible for handling
/// n < 5, even n, and perfect powers as needed.
pub(crate) fn bpsw_probable_prime(n: &BigUint) -> bool {
    // Step 1: Miller-Rabin base 2
    let n_minus_1 = match n.checked_sub(&BigUint::one()) {
        Some(v) => v,
        None => return false,
    };
    let (d, s) = factor_out_two(&n_minus_1);

    match miller_rabin_witness(n, &d, s, 2) {
        Ok(true) => {} // passes MR base 2, continue to Lucas
        Ok(false) => return false,
        Err(_) => return false,
    }

    // Step 2: Strong Lucas test with Selfridge parameters
    match selfridge_params(n) {
        None => false, // perfect square or jacobi=0 => composite
        Some((_d_val, p, q)) => strong_lucas_probable_prime(n, p, q),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Test whether `n` is probably prime.
///
/// Uses deterministic Miller-Rabin with proven witness sets for
/// `n < 3.3×10²⁴`. For larger `n`, uses BPSW (Miller-Rabin base-2 +
/// strong Lucas with Selfridge parameters). No counterexample to BPSW is
/// known; it has been verified for all n below 2^64.
///
/// Returns `false` for `n < 2` and for even `n > 2`.
/// Returns `true` for `n = 2` and `n = 3`.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{is_probably_prime, BigUint};
///
/// assert!(is_probably_prime(&BigUint::from(97u64)));
/// assert!(!is_probably_prime(&BigUint::from(561u64))); // Carmichael
/// assert!(!is_probably_prime(&BigUint::from(4u64)));
/// ```
pub fn is_probably_prime(n: &BigUint) -> bool {
    // --- Trivial cases ---
    if n < &BigUint::from(2u64) {
        return false;
    }
    if n == &BigUint::from(2u64) || n == &BigUint::from(3u64) {
        return true;
    }
    // Even numbers > 2 are composite.
    if !is_odd(n) {
        return false;
    }

    // --- Select witness set based on size ---
    match biguint_to_u128(n) {
        Some(n128) => match select_witnesses(n128) {
            Some(witnesses) => {
                // Deterministic range: use proven Miller-Rabin witness set.
                let n_minus_1 = match n.checked_sub(&BigUint::one()) {
                    Some(v) => v,
                    None => return false,
                };
                let (d, s) = factor_out_two(&n_minus_1);
                for &a in witnesses {
                    match miller_rabin_witness(n, &d, s, a) {
                        Ok(true) => continue,
                        Ok(false) => return false,
                        Err(_) => return false,
                    }
                }
                true
            }
            None => {
                // n >= 3.3e24 but fits in u128: use BPSW.
                bpsw_probable_prime(n)
            }
        },
        None => {
            // n > u128::MAX: use BPSW.
            bpsw_probable_prime(n)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::mod_arith::mod_pow;
    use crate::native::sieve::prime_sieve;

    fn bu(n: u64) -> BigUint {
        BigUint::from(n)
    }

    // -----------------------------------------------------------------------
    // Jacobi symbol tests
    // -----------------------------------------------------------------------

    #[test]
    fn jacobi_corner_cases() {
        // jacobi(0, n) = 0 for n > 1
        assert_eq!(jacobi(0, &bu(7)), 0);
        assert_eq!(jacobi(0, &bu(15)), 0);
        // jacobi(1, n) = 1 for all odd n > 1
        assert_eq!(jacobi(1, &bu(5)), 1);
        assert_eq!(jacobi(1, &bu(7)), 1);
        assert_eq!(jacobi(1, &bu(99)), 1);
    }

    #[test]
    fn jacobi_minus1() {
        // jacobi(-1, n) = 1 if n ≡ 1 (mod 4), -1 if n ≡ 3 (mod 4)
        assert_eq!(jacobi(-1, &bu(5)), 1); // 5 ≡ 1 mod 4
        assert_eq!(jacobi(-1, &bu(13)), 1); // 13 ≡ 1 mod 4
        assert_eq!(jacobi(-1, &bu(7)), -1); // 7 ≡ 3 mod 4
        assert_eq!(jacobi(-1, &bu(11)), -1); // 11 ≡ 3 mod 4
    }

    #[test]
    fn jacobi_matches_legendre_for_primes() {
        // For odd prime p and a in [1, p-1]:
        // jacobi(a, p) = Legendre(a, p) = a^((p-1)/2) mod p (1 or p-1=-1)
        for &p in &[5u64, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47] {
            for a in 1u64..p {
                let exp = (p - 1) / 2;
                let leg_raw = mod_pow(&bu(a), &bu(exp), &bu(p)).expect("mod_pow");
                // leg_raw is 1 or p-1 (i.e., -1 mod p)
                let expected: i32 = if leg_raw == bu(1) { 1 } else { -1 };
                let got = jacobi(a as i64, &bu(p));
                assert_eq!(
                    got, expected,
                    "jacobi({a}, {p}) = {got} but Legendre = {expected}"
                );
            }
        }
    }

    #[test]
    fn jacobi_multiplicativity() {
        // jacobi(a, m*n) = jacobi(a, m) * jacobi(a, n) for odd m, n.
        let cases = [(3i64, 5u64, 7u64), (5, 9, 11), (7, 13, 15), (11, 21, 25)];
        for (a, m, n) in cases {
            // m*n
            let mn = bu(m * n);
            if !is_odd(&mn) {
                continue;
            }
            let j_mn = jacobi(a, &mn);
            let j_m = jacobi(a, &bu(m));
            let j_n = jacobi(a, &bu(n));
            // Note: m and n must both be odd for multiplicativity to hold
            if is_odd(&bu(m)) && is_odd(&bu(n)) && m > 1 && n > 1 {
                assert_eq!(
                    j_mn,
                    j_m * j_n,
                    "jacobi({a}, {}) = {j_mn} but jacobi({a},{m})*jacobi({a},{n}) = {}",
                    m * n,
                    j_m * j_n
                );
            }
        }
    }

    #[test]
    fn jacobi_known_values() {
        // Reference table values
        assert_eq!(jacobi(2, &bu(7)), 1); // 7 ≡ 7 mod 8
        assert_eq!(jacobi(2, &bu(5)), -1); // 5 ≡ 5 mod 8
        assert_eq!(jacobi(2, &bu(17)), 1); // 17 ≡ 1 mod 8
        assert_eq!(jacobi(2, &bu(11)), -1); // 11 ≡ 3 mod 8
        assert_eq!(jacobi(3, &bu(5)), -1); // 3 is non-residue mod 5
        assert_eq!(jacobi(3, &bu(7)), -1); // 3 is non-residue mod 7
        assert_eq!(jacobi(4, &bu(7)), 1); // 4 = 2^2, perfect square
        assert_eq!(jacobi(9, &bu(35)), 1); // 9 = 3^2
                                           // gcd(6, 9) = 3 > 1 => jacobi(6, 9) = 0
        assert_eq!(jacobi(6, &bu(9)), 0);
    }

    // -----------------------------------------------------------------------
    // Selfridge parameters tests
    // -----------------------------------------------------------------------

    #[test]
    fn selfridge_perfect_squares_return_none() {
        // Perfect squares must return None (otherwise D-search doesn't terminate)
        let squares = [
            1009u64 * 1009,
            65537u64 * 65537,
            999983u64 * 999983,
            4u64,  // 2^2
            9u64,  // 3^2
            49u64, // 7^2
        ];
        for &sq in &squares {
            let n = bu(sq);
            // Only test odd perfect squares (even ones are filtered before selfridge)
            if is_odd(&n) && n > bu(3) {
                assert!(
                    selfridge_params(&n).is_none(),
                    "selfridge_params({sq}) should be None (perfect square)"
                );
            }
        }
    }

    #[test]
    fn selfridge_primes_return_some() {
        for &p in &[5u64, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47] {
            let (d_val, p_val, q_val) = selfridge_params(&bu(p))
                .unwrap_or_else(|| panic!("selfridge_params({p}) returned None"));
            // Verify P=1, Q=(1-D)/4
            assert_eq!(p_val, 1, "P must be 1 for prime {p}");
            assert_eq!(q_val, (1 - d_val) / 4, "Q mismatch for prime {p}");
            // Verify jacobi(D, p) = -1
            assert_eq!(
                jacobi(d_val, &bu(p)),
                -1,
                "jacobi(D={d_val}, {p}) must be -1"
            );
        }
    }

    // -----------------------------------------------------------------------
    // BPSW direct tests (exercises BPSW on small n with known ground truth)
    // -----------------------------------------------------------------------

    #[test]
    fn bpsw_matches_sieve_for_odd_n() {
        // Direct test of bpsw_probable_prime against sieve ground truth.
        // This ensures Lucas is correct even for small n.
        let sieve_primes = prime_sieve(10_000);
        for n in (5u64..10_000).step_by(2) {
            // Only odd n >= 5
            let expected = sieve_primes.binary_search(&n).is_ok();
            let got = bpsw_probable_prime(&bu(n));
            assert_eq!(
                got, expected,
                "bpsw_probable_prime({n}) = {got} but expected {expected}"
            );
        }
    }

    #[test]
    fn bpsw_base2_strong_pseudoprimes_are_composite() {
        // These are base-2 strong pseudoprimes (OEIS A001262):
        // they PASS Miller-Rabin base 2 but are caught by the strong Lucas test.
        // This specifically validates the Lucas half of BPSW.
        for &n in &[2047u64, 3277, 4033, 4681, 8321, 15841, 29341, 42799, 49141] {
            let result = bpsw_probable_prime(&bu(n));
            assert!(
                !result,
                "base-2 strong pseudoprime {n} should fail BPSW (Lucas should catch it)"
            );
        }
    }

    #[test]
    fn bpsw_strong_lucas_pseudoprimes_are_composite() {
        // Strong Lucas pseudoprimes (OEIS A217255): pass Lucas but fail MR base 2.
        // MR-base-2 catches these, validating the MR half of BPSW.
        for &n in &[5459u64, 5777, 10877, 16109, 18971] {
            assert!(
                !bpsw_probable_prime(&bu(n)),
                "strong Lucas pseudoprime {n} should fail BPSW (MR-base-2 catches it)"
            );
        }
    }

    // -----------------------------------------------------------------------
    // is_probably_prime tests
    // -----------------------------------------------------------------------

    #[test]
    fn primality_trivial() {
        assert!(!is_probably_prime(&bu(0)));
        assert!(!is_probably_prime(&bu(1)));
        assert!(is_probably_prime(&bu(2)));
        assert!(is_probably_prime(&bu(3)));
        assert!(!is_probably_prime(&bu(4)));
        assert!(is_probably_prime(&bu(5)));
    }

    #[test]
    fn primality_matches_sieve_up_to_10000() {
        let sieve_primes = prime_sieve(10_000);
        for n in 2u64..10_000 {
            let expected = sieve_primes.binary_search(&n).is_ok();
            let got = is_probably_prime(&bu(n));
            assert_eq!(got, expected, "primality mismatch at n={}", n);
        }
    }

    #[test]
    fn carmichael_numbers_are_composite() {
        // Classical Carmichael numbers that fool single-base Miller-Rabin.
        for &n in &[561u64, 1105, 1729, 2465, 2821, 6601, 8911, 10585] {
            assert!(
                !is_probably_prime(&bu(n)),
                "Carmichael {} was incorrectly identified as prime",
                n
            );
        }
    }

    #[test]
    fn mersenne_primes() {
        // 2^p - 1 for known Mersenne prime exponents.
        for p in [7u32, 13, 17, 19, 31] {
            let m = BigUint::from(2u64)
                .pow(p)
                .checked_sub(&BigUint::one())
                .expect("2^p > 1");
            assert!(is_probably_prime(&m), "2^{}-1 should be prime", p);
        }
    }

    #[test]
    fn mersenne_composite() {
        // 2^11 - 1 = 2047 = 23 × 89 (composite).
        let m2047 = bu(2047);
        assert!(!is_probably_prime(&m2047));
    }

    #[test]
    fn large_known_prime() {
        // 2^31 - 1 = 2147483647 (a well-known Mersenne prime M_31).
        let m31 = BigUint::from(2u64)
            .pow(31)
            .checked_sub(&BigUint::one())
            .expect("2^31 > 1");
        assert!(is_probably_prime(&m31));
    }

    #[test]
    fn bpsw_large_mersenne_primes() {
        // Mersenne primes large enough to exercise BPSW path or high witness counts.
        // 2^61 - 1 = 2305843009213693951 (M_61, a known Mersenne prime).
        let m61 = BigUint::from(2u64)
            .pow(61)
            .checked_sub(&BigUint::one())
            .expect("2^61 > 1");
        assert!(is_probably_prime(&m61), "M_61 should be prime");
    }

    #[test]
    fn bpsw_perfect_square_is_composite() {
        // These must NOT hang and must return false.
        let n1 = bu(1009) * bu(1009); // 1009^2 = 1018081 (odd perfect square)
        let n2 = bu(65537) * bu(65537);
        let n3 = bu(999983) * bu(999983); // 999983 is prime
        assert!(!is_probably_prime(&n1), "1009^2 should be composite");
        assert!(!is_probably_prime(&n2), "65537^2 should be composite");
        assert!(!is_probably_prime(&n3), "999983^2 should be composite");
    }
}
