//! Continued-fraction support for the native [`BigRational`].
//!
//! Every rational number has a *finite* simple continued-fraction expansion
//!
//! ```text
//! r = a0 + 1/(a1 + 1/(a2 + ... + 1/a_n))   =   [a0; a1, a2, ..., a_n]
//! ```
//!
//! where the leading term `a0` may be negative (it is `floor(r)`) and every
//! subsequent term is a positive integer (`a_i >= 1`).  This module provides:
//!
//! - [`BigRational::continued_fraction`] — the canonical expansion.
//! - [`BigRational::from_continued_fraction`] — reconstruction (inverse).
//! - [`BigRational::convergents`] — the successive best rational truncations.
//! - [`BigRational::best_rational_approximation`] — the genuinely closest
//!   rational whose denominator does not exceed a bound (convergents *plus*
//!   the half-rule semiconvergent at the truncation point).
//!
//! # Floor vs. truncation
//!
//! The native [`BigInt`] division truncates toward zero, while the continued
//! fraction needs `a0 = floor(r)` (toward negative infinity).  Because the
//! denominator is a strictly positive [`BigUint`], the only place the two
//! conventions diverge is the leading term of a negative rational; we adjust
//! it explicitly (see `floor_div` below).
//!
//! # Examples
//!
//! ```
//! use oxinum_rational::native::BigRational;
//! use oxinum_int::native::{BigInt, BigUint};
//!
//! // 415/93 = [4; 2, 6, 7]
//! let r = BigRational::from_parts(BigInt::from(415i64), BigUint::from_u64(93))
//!     .expect("non-zero denominator");
//! let cf = r.continued_fraction();
//! assert_eq!(
//!     cf,
//!     vec![
//!         BigInt::from(4i64),
//!         BigInt::from(2i64),
//!         BigInt::from(6i64),
//!         BigInt::from(7i64),
//!     ]
//! );
//! ```

use oxinum_core::{OxiNumError, OxiNumResult};
use oxinum_int::native::{divrem_int, BigInt, BigUint};

use super::BigRational;

// ---------------------------------------------------------------------------
// Internal: floor division by a positive denominator
// ---------------------------------------------------------------------------

/// Compute `(floor(num / den), num - floor(num/den) * den)` for a signed
/// numerator over a **strictly positive** denominator `den` (lifted into a
/// `BigInt`).
///
/// Native `BigInt` division truncates toward zero; the continued-fraction
/// algorithm needs floor (toward negative infinity).  When `num` is negative
/// and the truncating remainder is non-zero, we step the quotient down by one
/// and fold the denominator back into the remainder so that the returned
/// remainder satisfies `0 <= remainder < den`.
fn floor_div(num: &BigInt, den: &BigInt) -> (BigInt, BigInt) {
    let (mut q, mut r) = divrem_int(num, den);
    // `den > 0` by invariant, so the only correction needed is when the
    // truncating remainder came out negative (i.e. `num` was negative).
    if r.is_negative() {
        q = &q - &BigInt::one();
        r = &r + den;
    }
    (q, r)
}

// ---------------------------------------------------------------------------
// Public continued-fraction API
// ---------------------------------------------------------------------------

impl BigRational {
    /// Compute the canonical simple continued-fraction expansion
    /// `[a0; a1, a2, ...]`.
    ///
    /// The expansion is always finite for a rational.  The leading term `a0`
    /// equals `floor(self)` and may be negative; every later term is `>= 1`.
    /// For an integer the result is the single-element vector `[self]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    ///
    /// // 1/2 = [0; 2]
    /// let half = BigRational::from_parts(BigInt::from(1i64), BigUint::from_u64(2))
    ///     .expect("non-zero denominator");
    /// assert_eq!(half.continued_fraction(), vec![BigInt::ZERO, BigInt::from(2i64)]);
    /// ```
    pub fn continued_fraction(&self) -> Vec<BigInt> {
        let mut coeffs = Vec::new();

        // Work with a signed numerator over a signed (positive) denominator.
        // The denominator is `> 0` by the BigRational invariant.
        let mut num = self.num.clone();
        let mut den = BigInt::from(self.den.clone());

        loop {
            let (a, r) = floor_div(&num, &den);
            coeffs.push(a);
            if r.is_zero() {
                break;
            }
            // Next step: expand `den / r` (both now strictly positive).
            num = den;
            den = r;
        }
        coeffs
    }

    /// Reconstruct a [`BigRational`] from its continued-fraction coefficients.
    ///
    /// This is the inverse of [`continued_fraction`](Self::continued_fraction):
    /// folding from the tail, `result = a_last`, then for each earlier term
    /// `result = a_i + 1/result`.
    ///
    /// # Errors
    ///
    /// - [`OxiNumError::Parse`] when `coeffs` is empty.
    /// - [`OxiNumError::DivByZero`] when an interior partial result is zero
    ///   (only reachable for hand-constructed, non-canonical inputs such as
    ///   `[0, 0]`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    ///
    /// // [3; 7, 16] = 355/113
    /// let cf = vec![BigInt::from(3i64), BigInt::from(7i64), BigInt::from(16i64)];
    /// let r = BigRational::from_continued_fraction(&cf).expect("non-empty");
    /// let expected = BigRational::from_parts(BigInt::from(355i64), BigUint::from_u64(113))
    ///     .expect("non-zero denominator");
    /// assert_eq!(r, expected);
    /// ```
    pub fn from_continued_fraction(coeffs: &[BigInt]) -> OxiNumResult<Self> {
        let (last, rest) = coeffs
            .split_last()
            .ok_or_else(|| OxiNumError::Parse("empty continued fraction".into()))?;

        let mut result = BigRational::from_integer(last.clone());
        // Fold the remaining coefficients in reverse: result = a_i + 1/result.
        for coeff in rest.iter().rev() {
            if result.is_zero() {
                return Err(OxiNumError::DivByZero);
            }
            let reciprocal = result.recip()?;
            result = BigRational::from_integer(coeff.clone()) + reciprocal;
        }
        Ok(result)
    }

    /// Return the successive convergents `h_i / k_i` of the continued-fraction
    /// expansion.
    ///
    /// The convergents are the best rational approximations to `self` with
    /// monotonically increasing denominators; the final convergent equals
    /// `self` exactly.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    ///
    /// let r = BigRational::from_parts(BigInt::from(355i64), BigUint::from_u64(113))
    ///     .expect("non-zero denominator");
    /// let convs = r.convergents();
    /// // The convergents of 355/113 are 3, 22/7, 355/113.
    /// assert_eq!(convs.len(), 3);
    /// assert_eq!(*convs.last().expect("non-empty"), r);
    /// ```
    pub fn convergents(&self) -> Vec<BigRational> {
        let coeffs = self.continued_fraction();

        // Convergent recurrence:
        //   h_{-1} = 1, h_{-2} = 0
        //   k_{-1} = 0, k_{-2} = 1
        //   h_i = a_i * h_{i-1} + h_{i-2}
        //   k_i = a_i * k_{i-1} + k_{i-2}
        let mut h_prev2 = BigInt::ZERO; // h_{-2}
        let mut h_prev1 = BigInt::one(); // h_{-1}
        let mut k_prev2 = BigInt::one(); // k_{-2}
        let mut k_prev1 = BigInt::ZERO; // k_{-1}

        let mut out = Vec::with_capacity(coeffs.len());
        for a in &coeffs {
            let h = &(a * &h_prev1) + &h_prev2;
            let k = &(a * &k_prev1) + &k_prev2;

            out.push(convergent_from_signed(&h, &k));

            h_prev2 = h_prev1;
            h_prev1 = h;
            k_prev2 = k_prev1;
            k_prev1 = k;
        }
        out
    }

    /// Find the genuinely best rational approximation to `self` whose
    /// denominator does not exceed `max_den`.
    ///
    /// This walks the convergents while their denominator stays within
    /// `max_den`, then, at the truncation point, also considers the half-rule
    /// *semiconvergent* `(t·h_i + h_{i-1}) / (t·k_i + k_{i-1})` with
    /// `t = floor((max_den - k_{i-1}) / k_i)`.  Among the candidate(s) it
    /// returns the one closest to `self`, breaking ties toward the smaller
    /// denominator.
    ///
    /// When `max_den` is zero the result is the integer part `floor(self)`
    /// (the only "denominator <= 0" interpretation that yields a value).
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_rational::native::BigRational;
    /// use oxinum_int::native::{BigInt, BigUint};
    ///
    /// // Best approximation of 355/113 with denominator <= 100 is 311/99.
    /// let pi = BigRational::from_parts(BigInt::from(355i64), BigUint::from_u64(113))
    ///     .expect("non-zero denominator");
    /// let approx = pi.best_rational_approximation(&BigUint::from_u64(100));
    /// let expected = BigRational::from_parts(BigInt::from(311i64), BigUint::from_u64(99))
    ///     .expect("non-zero denominator");
    /// assert_eq!(approx, expected);
    /// ```
    pub fn best_rational_approximation(&self, max_den: &BigUint) -> BigRational {
        // Degenerate bound: the best "denominator <= 0" rational is the floor.
        if max_den.is_zero() {
            let coeffs = self.continued_fraction();
            // `continued_fraction` always returns at least one coefficient
            // (the floor); fall back to zero only defensively.
            return match coeffs.first() {
                Some(a0) => BigRational::from_integer(a0.clone()),
                None => BigRational::zero(),
            };
        }

        let coeffs = self.continued_fraction();
        let max_den_i = BigInt::from(max_den.clone());

        // Convergent recurrence state.
        let mut h_prev2 = BigInt::ZERO; // h_{i-2}
        let mut h_prev1 = BigInt::one(); // h_{i-1}
        let mut k_prev2 = BigInt::one(); // k_{i-2}
        let mut k_prev1 = BigInt::ZERO; // k_{i-1}

        // Best full convergent found so far whose denominator is <= max_den.
        // Seeded with the floor term, which always has denominator 1.
        let mut best = match coeffs.first() {
            Some(a0) => BigRational::from_integer(a0.clone()),
            None => return BigRational::zero(),
        };
        let mut have_best = false;

        for a in &coeffs {
            let h = &(a * &h_prev1) + &h_prev2;
            let k = &(a * &k_prev1) + &k_prev2;

            if k > max_den_i {
                // Truncation point. `(h_prev1, k_prev1)` is the last full
                // convergent within bound (h_{i-1}/k_{i-1}); `(h_prev2,
                // k_prev2)` is h_{i-2}/k_{i-2}. The semiconvergent uses
                // t = floor((max_den - k_{i-2}) / k_{i-1}).
                //
                // `k_prev1 >= 1` here: a convergent only exceeds the bound at
                // index >= 1 (the index-0 denominator is 1 <= max_den), so the
                // previous denominator is a genuine positive convergent
                // denominator.
                let (t, _) = floor_div(&(&max_den_i - &k_prev2), &k_prev1);
                let semi_h = &(&t * &h_prev1) + &h_prev2;
                let semi_k = &(&t * &k_prev1) + &k_prev2;

                let semi = convergent_from_signed(&semi_h, &semi_k);
                best = pick_closer(self, best, have_best, semi);
                break;
            }

            // `k` is within bounds: this convergent is a candidate.
            best = convergent_from_signed(&h, &k);
            have_best = true;

            h_prev2 = h_prev1;
            h_prev1 = h;
            k_prev2 = k_prev1;
            k_prev1 = k;
        }

        best
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a [`BigRational`] from a signed numerator `h` and a signed,
/// guaranteed-positive denominator `k`.
///
/// In the convergent recurrence the denominators `k_i` are always strictly
/// positive integers, and consecutive `(h_i, k_i)` are coprime, so this avoids
/// the fallible [`BigRational::from_parts`] path entirely (no `Result`, no
/// `unwrap`).  The sign always lives on the numerator.
fn convergent_from_signed(h: &BigInt, k: &BigInt) -> BigRational {
    // `k > 0` by construction; split into (sign, magnitude) and drop the sign.
    let (_k_sign, k_mag) = k.clone().into_parts();
    // `reduce_unchecked` requires `den != 0`; `k_mag >= 1` holds.
    BigRational::reduce_unchecked(h.clone(), k_mag)
}

/// Return whichever of `current` / `candidate` is closer to `target`.
///
/// When `have_current` is false the `candidate` is taken unconditionally.  On
/// an exact tie the smaller denominator wins (both are already reduced, so the
/// denominator comparison is canonical).
fn pick_closer(
    target: &BigRational,
    current: BigRational,
    have_current: bool,
    candidate: BigRational,
) -> BigRational {
    if !have_current {
        return candidate;
    }
    let err_current = (&current - target).abs();
    let err_candidate = (&candidate - target).abs();
    match err_candidate.cmp(&err_current) {
        core::cmp::Ordering::Less => candidate,
        core::cmp::Ordering::Greater => current,
        core::cmp::Ordering::Equal => {
            if candidate.den() < current.den() {
                candidate
            } else {
                current
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn br(n: i64, d: u64) -> BigRational {
        BigRational::from_parts(BigInt::from(n), BigUint::from_u64(d)).expect("br")
    }

    fn ints(vals: &[i64]) -> Vec<BigInt> {
        vals.iter().map(|&v| BigInt::from(v)).collect()
    }

    #[test]
    fn cf_classic_vector() {
        assert_eq!(br(415, 93).continued_fraction(), ints(&[4, 2, 6, 7]));
    }

    #[test]
    fn cf_pi_convergent() {
        assert_eq!(br(355, 113).continued_fraction(), ints(&[3, 7, 16]));
    }

    #[test]
    fn cf_negative_uses_floor() {
        // -415/93 = [-5; 1, 1, 6, 7] (floor convention, NOT truncation).
        assert_eq!(br(-415, 93).continued_fraction(), ints(&[-5, 1, 1, 6, 7]));
    }

    #[test]
    fn cf_integer_single_term() {
        assert_eq!(br(5, 1).continued_fraction(), ints(&[5]));
        assert_eq!(br(-7, 1).continued_fraction(), ints(&[-7]));
        assert_eq!(BigRational::zero().continued_fraction(), ints(&[0]));
    }

    #[test]
    fn cf_unit_fraction() {
        assert_eq!(br(1, 7).continued_fraction(), ints(&[0, 7]));
    }

    #[test]
    fn from_cf_roundtrip_classic() {
        let r = br(415, 93);
        let back = BigRational::from_continued_fraction(&r.continued_fraction()).expect("ok");
        assert_eq!(back, r);
    }

    #[test]
    fn from_cf_empty_errors() {
        assert_eq!(
            BigRational::from_continued_fraction(&[]),
            Err(OxiNumError::Parse("empty continued fraction".into()))
        );
    }

    #[test]
    fn from_cf_zero_chain_div_by_zero() {
        // [0, 0]: result starts at 0, then 0 + 1/0 -> DivByZero.
        assert_eq!(
            BigRational::from_continued_fraction(&ints(&[0, 0])),
            Err(OxiNumError::DivByZero)
        );
    }

    #[test]
    fn convergents_last_equals_self() {
        let r = br(355, 113);
        let convs = r.convergents();
        assert_eq!(*convs.last().expect("non-empty"), r);
    }

    #[test]
    fn convergents_strictly_improve() {
        let r = br(415, 93);
        let convs = r.convergents();
        let mut prev_err: Option<BigRational> = None;
        for c in &convs {
            let err = (c - &r).abs();
            if let Some(p) = prev_err {
                assert!(err < p, "convergent errors must strictly decrease");
            }
            prev_err = Some(err);
        }
    }

    #[test]
    fn best_approx_semiconvergent() {
        // The discriminating case: 311/99 (semiconvergent), not 22/7.
        assert_eq!(
            br(355, 113).best_rational_approximation(&BigUint::from_u64(100)),
            br(311, 99)
        );
    }

    #[test]
    fn best_approx_exact_when_bound_allows() {
        assert_eq!(
            br(355, 113).best_rational_approximation(&BigUint::from_u64(113)),
            br(355, 113)
        );
    }

    #[test]
    fn best_approx_convergent_bounds() {
        assert_eq!(
            br(355, 113).best_rational_approximation(&BigUint::from_u64(10)),
            br(22, 7)
        );
        assert_eq!(
            br(355, 113).best_rational_approximation(&BigUint::from_u64(7)),
            br(22, 7)
        );
    }

    #[test]
    fn best_approx_denom_zero_is_floor() {
        assert_eq!(
            br(7, 2).best_rational_approximation(&BigUint::ZERO),
            BigRational::from_i64(3)
        );
        assert_eq!(
            br(-7, 2).best_rational_approximation(&BigUint::ZERO),
            BigRational::from_i64(-4)
        );
    }

    #[test]
    fn floor_div_matches_floor_semantics() {
        // -7 / 3 = floor(-2.333) = -3 remainder 2.
        let (q, r) = floor_div(&BigInt::from(-7i64), &BigInt::from(3i64));
        assert_eq!(q, BigInt::from(-3i64));
        assert_eq!(r, BigInt::from(2i64));
        // 7 / 3 = 2 remainder 1.
        let (q, r) = floor_div(&BigInt::from(7i64), &BigInt::from(3i64));
        assert_eq!(q, BigInt::from(2i64));
        assert_eq!(r, BigInt::from(1i64));
    }
}
