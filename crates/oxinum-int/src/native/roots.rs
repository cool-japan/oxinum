//! Integer square root and generalized integer nth root (Newton's method).
//!
//! All routines return the FLOOR of the true root. After Newton iteration
//! converges, a final correction step verifies the floor invariant
//! `x^n <= value < (x+1)^n` and, if necessary, decrements `x` by one to
//! restore it. This guards against the edge cases near perfect-power
//! boundaries that pure Newton can miss by one.

use super::int::BigInt;
use super::uint::BigUint;
use crate::{OxiNumError, OxiNumResult};
use oxinum_core::Sign;

impl BigUint {
    /// Integer square root (floor) via Newton's method.
    ///
    /// Returns the largest `x` such that `x*x <= self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(16).sqrt(), BigUint::from_u64(4));
    /// assert_eq!(BigUint::from_u64(17).sqrt(), BigUint::from_u64(4));
    /// assert_eq!(BigUint::zero().sqrt(), BigUint::zero());
    /// assert_eq!(BigUint::from_u64(1).sqrt(), BigUint::from_u64(1));
    /// ```
    pub fn sqrt(&self) -> BigUint {
        if self.is_zero() {
            return BigUint::zero();
        }
        if self.is_one() {
            return BigUint::one();
        }
        // Initial estimate: x0 = 1 << ceil(bit_length / 2). This guarantees
        // x0^2 >= self (so the Newton iteration is monotonically decreasing).
        let bl = self.bit_length();
        let init_shift = bl.div_ceil(2);
        let mut x = BigUint::one().shl_bits(init_shift);
        loop {
            // x_next = (x + self / x) / 2
            let q = self / &x;
            let sum = &x + &q;
            let next = sum.shr_bits(1);
            if next >= x {
                break;
            }
            x = next;
        }
        // Floor correction: ensure x*x <= self < (x+1)*(x+1). The Newton
        // step above can leave x one too large in rare boundary cases.
        debug_assert!(
            &x * &x <= *self || x.is_zero(),
            "sqrt floor invariant lower"
        );
        while &x * &x > *self {
            // Defensive: should never iterate after Newton convergence, but
            // keep the loop to make the invariant unconditional.
            x = x
                .checked_sub(&BigUint::one())
                .expect("x > 0 because x*x > self > 0");
        }
        debug_assert!({
            let xp1 = &x + &BigUint::one();
            &xp1 * &xp1 > *self
        });
        x
    }

    /// Integer `n`-th root (floor) via Newton's method generalized.
    ///
    /// Returns the largest `x` such that `x^n <= self`. `n` must be `>= 1`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiNumError::Precision`] if `n == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigUint;
    /// assert_eq!(BigUint::from_u64(27).nth_root(3).unwrap(), BigUint::from_u64(3));
    /// assert_eq!(BigUint::from_u64(28).nth_root(3).unwrap(), BigUint::from_u64(3));
    /// assert_eq!(BigUint::from_u64(81).nth_root(4).unwrap(), BigUint::from_u64(3));
    /// ```
    pub fn nth_root(&self, n: u32) -> OxiNumResult<BigUint> {
        if n == 0 {
            return Err(OxiNumError::Precision("zeroth root is undefined".into()));
        }
        if n == 1 {
            return Ok(self.clone());
        }
        if n == 2 {
            return Ok(self.sqrt());
        }
        if self.is_zero() {
            return Ok(BigUint::zero());
        }
        if self.is_one() {
            return Ok(BigUint::one());
        }
        // Initial estimate: x0 = 1 << ceil(bit_length / n). For n >= 2 and
        // value > 1, this gives x0 >= true root (Newton converges
        // monotonically downward).
        let bl = self.bit_length();
        let n64 = n as u64;
        let init_shift = bl.div_ceil(n64);
        let mut x = BigUint::one().shl_bits(init_shift);
        let n_big = BigUint::from_u64(n64);
        let nm1 = n - 1;
        let nm1_big = BigUint::from_u64((n - 1) as u64);
        // Newton: x_next = ((n-1) * x + value / x^(n-1)) / n
        loop {
            let xnm1 = x.pow(nm1);
            let q = self / &xnm1;
            let lhs = &nm1_big * &x;
            let sum = &lhs + &q;
            let next = &sum / &n_big;
            if next >= x {
                break;
            }
            x = next;
        }
        // Floor correction: ensure x^n <= self.
        while x.pow(n) > *self {
            x = x
                .checked_sub(&BigUint::one())
                .expect("x > 0 because x^n > self > 0");
        }
        // Defensive (debug-only): confirm x^n <= self < (x+1)^n.
        debug_assert!(x.pow(n) <= *self);
        debug_assert!({
            let xp1 = &x + &BigUint::one();
            xp1.pow(n) > *self
        });
        Ok(x)
    }
}

impl BigInt {
    /// Integer `n`-th root for signed values.
    ///
    /// - `n == 0`: error ([`OxiNumError::Precision`]).
    /// - Negative self with even `n`: error (no real even root of a negative
    ///   integer).
    /// - Negative self with odd `n`: returns the unique negative root.
    /// - Otherwise: floor of the positive real root.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    /// assert_eq!(BigInt::from(-8i64).nth_root(3).unwrap(), BigInt::from(-2i64));
    /// assert_eq!(BigInt::from(27i64).nth_root(3).unwrap(), BigInt::from(3i64));
    /// assert!(BigInt::from(-4i64).nth_root(2).is_err());
    /// assert!(BigInt::from(10i64).nth_root(0).is_err());
    /// ```
    pub fn nth_root(&self, n: u32) -> OxiNumResult<BigInt> {
        if n == 0 {
            return Err(OxiNumError::Precision("zeroth root is undefined".into()));
        }
        if self.sign() == Sign::Negative && !self.magnitude().is_zero() {
            if n % 2 == 0 {
                return Err(OxiNumError::Precision(
                    format!("even ({n}-th) root of a negative integer is not a real number").into(),
                ));
            }
            let root_mag = self.magnitude().nth_root(n)?;
            return Ok(BigInt::from_parts(Sign::Negative, root_mag));
        }
        // Positive (or zero) path.
        let root_mag = self.magnitude().nth_root(n)?;
        Ok(BigInt::from_parts(Sign::Positive, root_mag))
    }

    /// Integer square root for signed values. Errors for negative inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_int::native::BigInt;
    /// assert_eq!(BigInt::from(49i64).sqrt().unwrap(), BigInt::from(7i64));
    /// assert!(BigInt::from(-1i64).sqrt().is_err());
    /// ```
    pub fn sqrt(&self) -> OxiNumResult<BigInt> {
        self.nth_root(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_perfect_squares() {
        for k in 0u64..40 {
            let n = BigUint::from_u64(k * k);
            assert_eq!(n.sqrt(), BigUint::from_u64(k));
        }
    }

    #[test]
    fn sqrt_non_perfect() {
        // 17 -> 4 (4^2 = 16, 5^2 = 25)
        assert_eq!(BigUint::from_u64(17).sqrt(), BigUint::from_u64(4));
        assert_eq!(BigUint::from_u64(99).sqrt(), BigUint::from_u64(9));
    }

    #[test]
    fn nth_root_basics() {
        // cube root
        assert_eq!(
            BigUint::from_u64(27).nth_root(3).expect("ok"),
            BigUint::from_u64(3)
        );
        assert_eq!(
            BigUint::from_u64(28).nth_root(3).expect("ok"),
            BigUint::from_u64(3)
        );
        // 4th root of 16 = 2
        assert_eq!(
            BigUint::from_u64(16).nth_root(4).expect("ok"),
            BigUint::from_u64(2)
        );
    }

    #[test]
    fn nth_root_signed_negative_odd() {
        assert_eq!(
            BigInt::from(-8i64).nth_root(3).expect("ok"),
            BigInt::from(-2i64)
        );
        assert_eq!(
            BigInt::from(-1000i64).nth_root(3).expect("ok"),
            BigInt::from(-10i64)
        );
    }

    #[test]
    fn nth_root_signed_negative_even_errors() {
        assert!(BigInt::from(-4i64).nth_root(2).is_err());
        assert!(BigInt::from(-1024i64).nth_root(4).is_err());
    }

    #[test]
    fn nth_root_zero_argument_errors() {
        assert!(BigUint::from_u64(10).nth_root(0).is_err());
        assert!(BigInt::from(10i64).nth_root(0).is_err());
    }
}
