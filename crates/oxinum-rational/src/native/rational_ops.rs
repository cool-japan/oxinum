//! Arithmetic operator impls (`Add`/`Sub`/`Mul`/`Div`/`Rem` + `*Assign`)
//! plus `PartialOrd`/`Ord`/`Hash` for [`BigRational`].
//!
//! Every binary operator provides all four ownership variants:
//! `&T op &T`, `T op T`, `T op &T`, `&T op T`.
//!
//! # Zero-divisor policy
//!
//! Following the convention of [`oxinum_int::native::BigInt`]: `Div` and
//! `Rem` *panic* when the divisor is zero; constructors and `recip` return
//! [`OxiNumError::DivByZero`].

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

use oxinum_int::native::{gcd, BigInt};

use super::rational::BigRational;

// ---------------------------------------------------------------------------
// Internal core helpers
// ---------------------------------------------------------------------------

/// Core `Add` body shared by all four `Add` ownership variants.
///
/// Uses the GCD-optimised cross-multiplication:
///
/// ```text
/// g = gcd(b, d)
/// res_num = a * (d / g) + c * (b / g)
/// res_den = lcm(b, d)   =   (b / g) * d   =   b * (d / g)
/// reduce(res_num, res_den)
/// ```
fn add_core(a: &BigRational, b: &BigRational) -> BigRational {
    // Common-denominator GCD.
    let g = gcd(a.den().clone(), b.den().clone());
    let b_over_g = a.den() / &g;
    let d_over_g = b.den() / &g;
    // Lift both pieces to BigInt for signed cross-multiplication.
    let lhs = &a.num * &BigInt::from(d_over_g.clone());
    let rhs = &b.num * &BigInt::from(b_over_g.clone());
    let num = &lhs + &rhs;
    // lcm(b, d) = (b/g) * d  =  b * (d/g). Pick the second form; both work.
    let den = &b_over_g * b.den();
    BigRational::reduce_unchecked(num, den)
}

/// Core `Sub` body: `a - c = a + (-c)`, but inlined to avoid an extra
/// allocation for the negation.
fn sub_core(a: &BigRational, b: &BigRational) -> BigRational {
    let g = gcd(a.den().clone(), b.den().clone());
    let b_over_g = a.den() / &g;
    let d_over_g = b.den() / &g;
    let lhs = &a.num * &BigInt::from(d_over_g.clone());
    let rhs = &b.num * &BigInt::from(b_over_g.clone());
    let num = &lhs - &rhs;
    let den = &b_over_g * b.den();
    BigRational::reduce_unchecked(num, den)
}

/// Core `Mul` body using the diagonal pre-reduce:
///
/// ```text
/// g1 = gcd(|a|, d)
/// g2 = gcd(|c|, b)
/// res_num = (a / g1) * (c / g2)
/// res_den = (b / g2) * (d / g1)
/// ```
///
/// The result is already reduced (no final gcd needed).
fn mul_core(a: &BigRational, b: &BigRational) -> BigRational {
    // Pre-reduce diagonals.
    let g1 = gcd(a.num.magnitude().clone(), b.den.clone()); // gcd(|a|, d)
    let g2 = gcd(b.num.magnitude().clone(), a.den.clone()); // gcd(|c|, b)

    // a / g1 (preserving sign of a)
    let (sa, ma) = a.num.clone().into_parts();
    let a_red_mag = &ma / &g1;
    let a_red = BigInt::from_parts(sa, a_red_mag);

    // c / g2 (preserving sign of c)
    let (sc, mc) = b.num.clone().into_parts();
    let c_red_mag = &mc / &g2;
    let c_red = BigInt::from_parts(sc, c_red_mag);

    // Denominators.
    let b_red = a.den() / &g2; // b / g2
    let d_red = b.den() / &g1; // d / g1

    let num = &a_red * &c_red;
    let den = &b_red * &d_red;

    // The numerator may now be zero (if either factor was zero). Map to
    // canonical zero in that case; otherwise the diagonal pre-reduce
    // guarantees the result is already in lowest terms.
    if num.is_zero() {
        return BigRational::zero();
    }
    BigRational { num, den }
}

/// Core `Div` body: `(a/b) / (c/d) = (a*d) / (b*c)`. Panics if `b` is zero
/// (zero divisor).
///
/// We reuse the diagonal pre-reduce of `Mul` by first negating-then-swapping
/// the second operand on the fly. The resulting denominator must remain
/// positive: the sign of `c` flows into the numerator instead.
fn div_core(a: &BigRational, b: &BigRational) -> BigRational {
    if b.num.is_zero() {
        panic!("BigRational: division by zero");
    }
    // Build the reciprocal of `b`. `recip` reroutes the sign onto the new
    // numerator, so the denominator-must-be-positive invariant holds.
    // Safety: we just rejected the zero case.
    let recip = b
        .recip()
        .expect("recip(non-zero) cannot fail — zero check above");
    mul_core(a, &recip)
}

// ---------------------------------------------------------------------------
// Macros — wire owned/borrowed variants and *Assign to the *_core helpers.
// ---------------------------------------------------------------------------

macro_rules! impl_binop {
    ($Trait:ident, $method:ident, $core:ident) => {
        impl $Trait<&BigRational> for &BigRational {
            type Output = BigRational;
            #[inline]
            fn $method(self, rhs: &BigRational) -> BigRational {
                $core(self, rhs)
            }
        }

        impl $Trait<BigRational> for BigRational {
            type Output = BigRational;
            #[inline]
            fn $method(self, rhs: BigRational) -> BigRational {
                $core(&self, &rhs)
            }
        }

        impl $Trait<&BigRational> for BigRational {
            type Output = BigRational;
            #[inline]
            fn $method(self, rhs: &BigRational) -> BigRational {
                $core(&self, rhs)
            }
        }

        impl $Trait<BigRational> for &BigRational {
            type Output = BigRational;
            #[inline]
            fn $method(self, rhs: BigRational) -> BigRational {
                $core(self, &rhs)
            }
        }
    };
}

macro_rules! impl_assign {
    ($Trait:ident, $method:ident, $core:ident) => {
        impl $Trait<&BigRational> for BigRational {
            #[inline]
            fn $method(&mut self, rhs: &BigRational) {
                *self = $core(&*self, rhs);
            }
        }

        impl $Trait<BigRational> for BigRational {
            #[inline]
            fn $method(&mut self, rhs: BigRational) {
                *self = $core(&*self, &rhs);
            }
        }
    };
}

// Add / Sub / Mul / Div
impl_binop!(Add, add, add_core);
impl_binop!(Sub, sub, sub_core);
impl_binop!(Mul, mul, mul_core);
impl_binop!(Div, div, div_core);

impl_assign!(AddAssign, add_assign, add_core);
impl_assign!(SubAssign, sub_assign, sub_core);
impl_assign!(MulAssign, mul_assign, mul_core);
impl_assign!(DivAssign, div_assign, div_core);

// ---------------------------------------------------------------------------
// Rem (trait completeness)
// ---------------------------------------------------------------------------

/// `Rem` for `BigRational`: panics on zero divisor; returns zero otherwise.
///
/// Rationals form a field, so `a / b` is exact and `a mod b == 0` whenever
/// `b != 0`. Provided for trait-bound completeness (e.g. `num_traits::Num`).
fn rem_core(_a: &BigRational, b: &BigRational) -> BigRational {
    if b.num.is_zero() {
        panic!("BigRational: remainder with zero divisor");
    }
    BigRational::zero()
}

impl_binop!(Rem, rem, rem_core);
impl_assign!(RemAssign, rem_assign, rem_core);

// ---------------------------------------------------------------------------
// Hash, PartialOrd, Ord
// ---------------------------------------------------------------------------

impl Hash for BigRational {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Canonical form makes `(num, den)` uniquely identifying.
        self.num.hash(state);
        self.den.hash(state);
    }
}

impl Ord for BigRational {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_impl(other)
    }
}

impl PartialOrd for BigRational {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oxinum_int::native::BigUint;

    fn r(n: i64, d: u64) -> BigRational {
        BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d))
            .expect("non-zero denominator")
    }

    #[test]
    fn add_unlike_denominators() {
        // 1/2 + 1/3 = 5/6
        assert_eq!(r(1, 2) + r(1, 3), r(5, 6));
    }

    #[test]
    fn add_cancels_to_zero() {
        // -3/4 + 3/4 = 0
        let sum = r(-3, 4) + r(3, 4);
        assert!(sum.is_zero());
        assert_eq!(sum.to_string(), "0");
    }

    #[test]
    fn sub_unlike_denominators() {
        // 1 - 1/3 = 2/3
        let one = BigRational::one();
        assert_eq!(one - r(1, 3), r(2, 3));
    }

    #[test]
    fn mul_reduces_to_half() {
        // 2/3 * 3/4 = 1/2
        assert_eq!(r(2, 3) * r(3, 4), r(1, 2));
    }

    #[test]
    fn mul_by_zero_is_zero() {
        let p = r(7, 9) * BigRational::zero();
        assert!(p.is_zero());
    }

    #[test]
    fn div_basic() {
        // (1/2) / (1/3) = 3/2
        assert_eq!(r(1, 2) / r(1, 3), r(3, 2));
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_by_zero_panics() {
        let _ = r(1, 2) / BigRational::zero();
    }

    #[test]
    fn rem_returns_zero() {
        // 7/3 % 1/2 = 0 (rationals form a field)
        let rem = r(7, 3) % r(1, 2);
        assert!(rem.is_zero());
    }

    #[test]
    #[should_panic(expected = "remainder with zero divisor")]
    fn rem_by_zero_panics() {
        let _ = r(1, 2) % BigRational::zero();
    }

    #[test]
    fn assign_ops_idempotent() {
        let mut a = r(1, 2);
        a += r(1, 3);
        assert_eq!(a, r(5, 6));
        a -= r(1, 3);
        assert_eq!(a, r(1, 2));
        a *= r(2, 1);
        assert_eq!(a, BigRational::one());
        a /= r(2, 1);
        assert_eq!(a, r(1, 2));
    }

    #[test]
    fn ord_basic() {
        let half = r(1, 2);
        let third = r(1, 3);
        let neg_half = r(-1, 2);
        assert!(third < half);
        assert!(neg_half < third);
        assert!(neg_half < half);
        assert_eq!(half.cmp(&half), Ordering::Equal);
    }

    #[test]
    fn ord_zero_handled() {
        assert!(r(-1, 100) < BigRational::zero());
        assert!(BigRational::zero() < r(1, 1_000_000));
    }

    #[test]
    fn hash_matches_eq() {
        use std::collections::hash_map::DefaultHasher;
        let a = r(6, 4); // reduces to 3/2
        let b = r(3, 2);
        assert_eq!(a, b);
        let mut ha = DefaultHasher::new();
        a.hash(&mut ha);
        let mut hb = DefaultHasher::new();
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn borrowed_owned_variants_consistent() {
        // Smoke-test all four ownership flavours of Add.
        let a = r(1, 2);
        let b = r(1, 3);
        let target = r(5, 6);
        assert_eq!(&a + &b, target);
        assert_eq!(a.clone() + b.clone(), target);
        assert_eq!(a.clone() + &b, target);
        assert_eq!(&a + b.clone(), target);
    }
}
