//! Lucas sequences U_n and V_n modulo m.
//!
//! The Lucas sequences of the first and second kind are defined by the
//! recurrences:
//!
//! - U_0 = 0, U_1 = 1, U_{k+2} = P · U_{k+1} − Q · U_k
//! - V_0 = 2, V_1 = P, V_{k+2} = P · V_{k+1} − Q · V_k
//!
//! where P, Q are integer parameters and D = P² − 4Q is the discriminant.
//!
//! # Algorithm
//!
//! Uses the binary-expansion ladder with doubling formulas:
//!
//! - U_{2k} = U_k · V_k (mod m)
//! - V_{2k} = V_k² − 2·Q^k (mod m)
//! - Q^{2k} = (Q^k)² (mod m)
//!
//! and the increment step (requires m to be odd so that 2 is invertible):
//!
//! - U_{2k+1} = (P · U_{2k} + V_{2k}) / 2 (mod m)
//! - V_{2k+1} = (D · U_{2k} + P · V_{2k}) / 2 (mod m)
//! - Q^{2k+1} = Q^{2k} · Q (mod m)
//!
//! where D = P² − 4Q, and "/2 mod m" is multiplication by the modular
//! inverse of 2 (which exists when m is odd).

use super::div::divrem;
use super::mod_arith::mod_mul;
use super::uint::BigUint;
use oxinum_core::{OxiNumError, OxiNumResult};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute `(U_n mod m, V_n mod m)` for Lucas parameters P and Q.
///
/// `n` must be a non-negative integer. `m` must be a positive odd number
/// (oddness is required because the halving step "/2 mod m" requires 2 to
/// be invertible).
///
/// Returns `Err(OxiNumError::DivByZero)` if `m == 0`, or
/// `Err(OxiNumError::Domain)` if `m` is even.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::{lucas_uv, BigUint};
///
/// // Lucas sequence with P=1, Q=-1 gives Fibonacci numbers for U_n.
/// let m = BigUint::from(1_000_000_007u64); // odd prime modulus required
/// let (u, _v) = lucas_uv(&BigUint::from(7u64), 1, -1, &m).expect("lucas");
/// assert_eq!(u, BigUint::from(13u64)); // Fib(7) = 13
/// ```
pub fn lucas_uv(n: &BigUint, p: i64, q: i64, m: &BigUint) -> OxiNumResult<(BigUint, BigUint)> {
    if m.is_zero() {
        return Err(OxiNumError::DivByZero);
    }
    if !is_odd(m) {
        return Err(OxiNumError::Domain("lucas_uv: modulus must be odd".into()));
    }
    lucas_uv_mod(n, p, q, m)
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Internal implementation of the Lucas binary ladder.
fn lucas_uv_mod(n: &BigUint, p: i64, q: i64, m: &BigUint) -> OxiNumResult<(BigUint, BigUint)> {
    // Base cases.
    if n.is_zero() {
        // (U_0, V_0) = (0, 2)
        return Ok((BigUint::zero(), mod_reduce_u64(2, m)));
    }

    let bits = n.bit_length();
    if bits == 1 && n.test_bit(0) {
        // n == 1: (U_1, V_1) = (1, P)
        return Ok((BigUint::one(), mod_reduce_signed(p, m)));
    }

    // Discriminant D = P^2 - 4Q (can be negative for standard Lucas params).
    // P and Q fit in i64, so P^2 and 4Q fit in i128.
    let d_val: i128 = (p as i128) * (p as i128) - 4 * (q as i128);

    // Initialize state triple (u, v, qk) = (U_1, V_1, Q^1).
    // The leading '1' bit of n is implicit; we process remaining bits below.
    let mut u = BigUint::one(); // U_1 = 1
    let mut v = mod_reduce_signed(p, m); // V_1 = P
    let mut qk = mod_reduce_signed(q, m); // Q^1 = Q

    for i in (0..bits - 1).rev() {
        // --- Doubling step: (U_k, V_k, Q^k) → (U_{2k}, V_{2k}, Q^{2k}) ---
        // U_{2k} = U_k * V_k mod m
        let new_u = mod_mul(&u, &v, m)?;
        // V_{2k} = V_k^2 - 2*Q^k mod m
        let v_sq = mod_mul(&v, &v, m)?;
        let two_qk = mod_mul(&BigUint::from(2u64), &qk, m)?;
        let new_v = modsub(&v_sq, &two_qk, m);
        // Q^{2k} = (Q^k)^2 mod m
        let new_qk = mod_mul(&qk, &qk, m)?;

        u = new_u;
        v = new_v;
        qk = new_qk;

        // --- Increment step (if current bit is 1): (U_{2k+1}, V_{2k+1}, Q^{2k+1}) ---
        if n.test_bit(i) {
            let p_mod = mod_reduce_signed(p, m);
            let d_mod = mod_reduce_i128(d_val, m);

            // U_{2k+1} = (P*U_{2k} + V_{2k}) / 2 mod m
            let pu = mod_mul(&p_mod, &u, m)?;
            let raw_u = mod_add(&pu, &v, m);
            let new_u2 = moddiv2(raw_u, m);

            // V_{2k+1} = (D*U_{2k} + P*V_{2k}) / 2 mod m
            let du = mod_mul(&d_mod, &u, m)?;
            let pv = mod_mul(&p_mod, &v, m)?;
            let raw_v = mod_add(&du, &pv, m);
            let new_v2 = moddiv2(raw_v, m);

            // Q^{2k+1} = Q^{2k} * Q mod m
            let q_mod = mod_reduce_signed(q, m);
            let new_qk2 = mod_mul(&qk, &q_mod, m)?;

            u = new_u2;
            v = new_v2;
            qk = new_qk2;
        }
    }

    Ok((u, v))
}

// ---------------------------------------------------------------------------
// Arithmetic helpers
// ---------------------------------------------------------------------------

/// Reduce a signed i64 value modulo m (result in [0, m)).
fn mod_reduce_signed(x: i64, m: &BigUint) -> BigUint {
    if x >= 0 {
        let bx = BigUint::from(x as u64);
        let (_, rem) = divrem(&bx, m);
        rem
    } else {
        // x < 0: compute (m - (|x| mod m)) mod m
        let bx = BigUint::from(x.unsigned_abs());
        let (_, rem) = divrem(&bx, m);
        if rem.is_zero() {
            BigUint::zero()
        } else {
            m.checked_sub(&rem).unwrap_or_else(BigUint::zero)
        }
    }
}

/// Reduce a signed i128 value modulo m (result in [0, m)).
fn mod_reduce_i128(x: i128, m: &BigUint) -> BigUint {
    if x >= 0 {
        let bx = BigUint::from(x as u128);
        let (_, rem) = divrem(&bx, m);
        rem
    } else {
        let bx = BigUint::from(x.unsigned_abs());
        let (_, rem) = divrem(&bx, m);
        if rem.is_zero() {
            BigUint::zero()
        } else {
            m.checked_sub(&rem).unwrap_or_else(BigUint::zero)
        }
    }
}

/// Reduce a u64 value modulo m.
fn mod_reduce_u64(x: u64, m: &BigUint) -> BigUint {
    let bx = BigUint::from(x);
    let (_, rem) = divrem(&bx, m);
    rem
}

/// `(a - b) mod m` where a, b are already in [0, m).
fn modsub(a: &BigUint, b: &BigUint, m: &BigUint) -> BigUint {
    if a >= b {
        let r = a.checked_sub(b).unwrap_or_else(BigUint::zero);
        let (_, rem) = divrem(&r, m);
        rem
    } else {
        // a < b: result = (a + m - b) mod m = m - (b - a)
        let diff = b.checked_sub(a).unwrap_or_else(BigUint::zero);
        if diff.is_zero() {
            return BigUint::zero();
        }
        m.checked_sub(&diff).unwrap_or_else(BigUint::zero)
    }
}

/// `(a + b) mod m` — a and b must already be < m, or this may give a result
/// in [0, 2m) which we then reduce once.
fn mod_add(a: &BigUint, b: &BigUint, m: &BigUint) -> BigUint {
    let sum = BigUint::add_ref(a, b);
    if &sum >= m {
        sum.checked_sub(m).unwrap_or_else(BigUint::zero)
    } else {
        sum
    }
}

/// Divide by 2 modulo m for odd m.
///
/// If `a` is even, result is `a >> 1` (since `a` and `m` are both in `[0, m)`,
/// and `m` is odd, `a` even ⇒ `a/2` is an integer in `[0, m/2)`).
/// If `a` is odd, result is `(a + m) >> 1` (since `a + m` is even when m is odd).
fn moddiv2(a: BigUint, m: &BigUint) -> BigUint {
    let a_mod = {
        let (_, r) = divrem(&a, m);
        r
    };
    if !is_odd(&a_mod) {
        // even: a / 2
        a_mod.shr_bits(1)
    } else {
        // odd: (a + m) / 2
        let sum = BigUint::add_ref(&a_mod, m);
        sum.shr_bits(1)
    }
}

/// Returns true if `n` is odd (LSB is 1).
#[inline]
pub(crate) fn is_odd(n: &BigUint) -> bool {
    n.as_limbs().first().copied().unwrap_or(0) & 1 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bu(n: u64) -> BigUint {
        BigUint::from(n)
    }

    #[test]
    fn lucas_base_cases() {
        let m = bu(1_000_000_007);
        // U_0 = 0, V_0 = 2
        let (u, v) = lucas_uv(&bu(0), 1, -1, &m).expect("lucas 0");
        assert_eq!(u, bu(0));
        assert_eq!(v, bu(2));

        // U_1 = 1, V_1 = P = 1
        let (u, v) = lucas_uv(&bu(1), 1, -1, &m).expect("lucas 1");
        assert_eq!(u, bu(1));
        assert_eq!(v, bu(1));
    }

    #[test]
    fn lucas_fibonacci_u_sequence() {
        // P=1, Q=-1: U_n is the Fibonacci sequence.
        // Fib: 0,1,1,2,3,5,8,13,21,34,55,...
        let expected_u = [0u64, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
        let m = bu(1_000_000_007);
        for (n, &eu) in expected_u.iter().enumerate() {
            let (u, _v) = lucas_uv(&bu(n as u64), 1, -1, &m).expect("lucas Fib");
            assert_eq!(u, bu(eu), "U_{} should be Fib_{}", n, n);
        }
    }

    #[test]
    fn lucas_fibonacci_v_sequence() {
        // P=1, Q=-1: V_n is the Lucas number sequence: 2,1,3,4,7,11,18,29,...
        let expected_v = [2u64, 1, 3, 4, 7, 11, 18, 29, 47, 76];
        let m = bu(1_000_000_007);
        for (n, &ev) in expected_v.iter().enumerate() {
            let (_u, v) = lucas_uv(&bu(n as u64), 1, -1, &m).expect("lucas V");
            assert_eq!(v, bu(ev), "V_{} should be Lucas_{}", n, n);
        }
    }

    #[test]
    fn lucas_modular_reduction() {
        // Fib(12) = 144; 144 mod 101 = 43. Use 101 (prime, odd) as modulus.
        let m = bu(101);
        let (u, _) = lucas_uv(&bu(12), 1, -1, &m).expect("lucas mod");
        assert_eq!(u, bu(144 % 101)); // 144 mod 101 = 43
    }

    #[test]
    fn lucas_rejects_even_modulus() {
        let m = bu(100);
        assert!(lucas_uv(&bu(5), 1, -1, &m).is_err());
    }

    #[test]
    fn lucas_rejects_zero_modulus() {
        assert!(lucas_uv(&bu(5), 1, -1, &BigUint::zero()).is_err());
    }
}
