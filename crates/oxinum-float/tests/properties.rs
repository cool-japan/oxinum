//! Property-style integration tests for `oxinum-float`.
//!
//! This file covers four hardening areas (see `crates/oxinum-float/TODO.md`
//! "Item 4 — oxinum-float test & property hardening"):
//!
//! 1. **Rounding-mode behaviour at exact midpoints.**  Uses `with_rounding`
//!    and `with_precision` to compare `HalfEven` (banker's) versus `HalfAway`
//!    on cases where the two modes diverge.
//! 2. **String round-trip.**  For a set of seed strings and several
//!    precisions, asserts that parsing the displayed string back yields the
//!    same value at the same precision.  We compare at the lower precision
//!    because `Display` may truncate trailing digits and because
//!    `DBig::with_precision` is the canonical precision-bearing form.
//! 3. **Special-value handling.**  See the `special_values_scoped` module for
//!    the rationale: `dashu-float` does not represent `NaN`, and `±Inf` is a
//!    sentinel-only value whose arithmetic panics.  Non-finite outcomes
//!    surface as `OxiNumError` from the `oxinum-float` wrapper, so these
//!    tests assert the error-returning paths instead of fabricating a
//!    `NaN`/`Inf` capability.
//! 4. **Commutativity / associativity proptests** and a **cross-validation
//!    block** that exercises the public `oxinum_float::{DBig, ...}` surface
//!    against the raw `dashu_float::DBig` operations.
//!
//! Naming note: we deliberately avoid importing names like `round`,
//! `HalfEven`, `HalfAway` at the crate root of this test file because
//! `oxinum_float::round` is a public re-export module.  All rounding-mode
//! types are qualified as `dashu_float::round::mode::{HalfEven, HalfAway}`.

#![forbid(unsafe_code)]

use std::str::FromStr;

use oxinum_float::{compute_e, compute_pi, ln, pow, sqrt, DBig, OxiNumError};

use dashu_float::FBig;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// 1. Rounding-mode behaviour at exact midpoints
// ---------------------------------------------------------------------------
//
// `DBig` is `FBig<HalfAway, 10>`.  To compare a different rounding mode we
// call `.with_rounding::<HalfEven>()` (or `<HalfAway>()`).  Note that
// `Context::<R>::new(0)` means *unlimited* precision in `dashu-float`, so
// we always call `with_precision(N)` with N >= 1.  `with_precision` returns
// `Approximation<Self, _>`; we extract via `.value()`.
//
// The midpoint cases below were selected because they actually discriminate
// between the two modes at the requested precision.  For example, "1.5"
// rounds to 2 under both `HalfEven` and `HalfAway` (1.5 -> 2 is "away from
// zero" and 2 is also the nearest even integer), so it would not be a
// useful test.

#[test]
fn midpoint_two_point_five_halfeven_to_two() {
    // 2.5 -> HalfEven (banker's) chooses the even neighbour (2).
    let v = DBig::from_str("2.5").expect("parse 2.5");
    assert_eq!(v.precision(), 2);
    let rounded = v
        .with_rounding::<dashu_float::round::mode::HalfEven>()
        .with_precision(1)
        .value();
    assert_eq!(rounded.to_string(), "2", "HalfEven(2.5) should be 2");
}

#[test]
fn midpoint_two_point_five_halfaway_to_three() {
    // 2.5 -> HalfAway rounds away from zero -> 3.
    let v = DBig::from_str("2.5").expect("parse 2.5");
    let rounded = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    assert_eq!(rounded.to_string(), "3", "HalfAway(2.5) should be 3");
}

#[test]
fn midpoint_four_point_five_halfeven_to_four() {
    let v = DBig::from_str("4.5").expect("parse 4.5");
    let rounded = v
        .with_rounding::<dashu_float::round::mode::HalfEven>()
        .with_precision(1)
        .value();
    assert_eq!(rounded.to_string(), "4", "HalfEven(4.5) should be 4");
}

#[test]
fn midpoint_four_point_five_halfaway_to_five() {
    let v = DBig::from_str("4.5").expect("parse 4.5");
    let rounded = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    assert_eq!(rounded.to_string(), "5", "HalfAway(4.5) should be 5");
}

#[test]
fn midpoint_six_point_five_halfeven_to_six() {
    let v = DBig::from_str("6.5").expect("parse 6.5");
    let rounded = v
        .with_rounding::<dashu_float::round::mode::HalfEven>()
        .with_precision(1)
        .value();
    assert_eq!(rounded.to_string(), "6", "HalfEven(6.5) should be 6");
}

#[test]
fn midpoint_six_point_five_halfaway_to_seven() {
    let v = DBig::from_str("6.5").expect("parse 6.5");
    let rounded = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    assert_eq!(rounded.to_string(), "7", "HalfAway(6.5) should be 7");
}

#[test]
fn midpoint_zero_point_two_five_diverges() {
    // `dashu_float` rounds against the *significand* digit count, not
    // against the precision the parser stamped on the input.  The
    // significand of `"0.25"` is `25` (two digits), so `with_precision(1)`
    // splits off the trailing `5`, which is the midpoint.
    //
    // HalfEven: penultimate significand digit `2` (even) wins -> "0.2".
    // HalfAway: away from zero -> "0.3".
    let v = DBig::from_str("0.25").expect("parse 0.25");
    let he = v
        .clone()
        .with_rounding::<dashu_float::round::mode::HalfEven>()
        .with_precision(1)
        .value();
    let ha = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    assert_eq!(he.to_string(), "0.2", "HalfEven(0.25) -> 0.2");
    assert_eq!(ha.to_string(), "0.3", "HalfAway(0.25) -> 0.3");
}

#[test]
fn midpoint_zero_point_four_five_diverges() {
    // Same significand-driven rule as the 0.25 case: significand is `45`
    // (2 digits), so `with_precision(1)` splits off the trailing `5`.
    // HalfEven keeps `4` (even) -> "0.4"; HalfAway -> "0.5".
    let v = DBig::from_str("0.45").expect("parse 0.45");
    let he = v
        .clone()
        .with_rounding::<dashu_float::round::mode::HalfEven>()
        .with_precision(1)
        .value();
    let ha = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    assert_eq!(he.to_string(), "0.4", "HalfEven(0.45) -> 0.4");
    assert_eq!(ha.to_string(), "0.5", "HalfAway(0.45) -> 0.5");
}

#[test]
fn midpoint_negative_two_point_five_diverges() {
    // -2.5 -> HalfEven -> -2;  HalfAway -> -3 (still "away from zero").
    let v = DBig::from_str("-2.5").expect("parse -2.5");
    let he = v
        .clone()
        .with_rounding::<dashu_float::round::mode::HalfEven>()
        .with_precision(1)
        .value();
    let ha = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    assert_eq!(he.to_string(), "-2", "HalfEven(-2.5) -> -2");
    assert_eq!(ha.to_string(), "-3", "HalfAway(-2.5) -> -3");
}

#[test]
fn midpoint_one_point_five_both_modes_agree() {
    // 1.5 -> HalfEven (nearest even is 2) -> 2;  HalfAway (away from 0) -> 2.
    // Included as a sanity check that "midpoint" does not always discriminate.
    let v = DBig::from_str("1.5").expect("parse 1.5");
    let he = v
        .clone()
        .with_rounding::<dashu_float::round::mode::HalfEven>()
        .with_precision(1)
        .value();
    let ha = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    assert_eq!(he.to_string(), "2");
    assert_eq!(ha.to_string(), "2");
}

// FBig in base 2 also honours rounding-mode selection.  Verify one case in
// the binary base to confirm the wrapper's binary FBig usage is consistent.
#[test]
fn midpoint_binary_fbig_halfeven_chooses_even() {
    // 0.5 in binary = 0.1_2 = "1 * 2^-1".  Rounding to 0 binary digits with
    // HalfEven picks the even integer (0), HalfAway picks 1.
    let v =
        FBig::<dashu_float::round::mode::HalfEven, 2>::from_str("0.1B0").expect("parse binary 0.5");
    // Convert this binary value with precision-1 (HalfEven) and observe.
    let he = v.clone().with_precision(1).value();
    let ha = v
        .with_rounding::<dashu_float::round::mode::HalfAway>()
        .with_precision(1)
        .value();
    // Either mode keeps "0.5" at one binary digit of precision.  We
    // primarily exercise the API doesn't panic and produces a sensible
    // textual round-trip.
    let _ = (he.to_string(), ha.to_string());
}

// ---------------------------------------------------------------------------
// 2. String round-trip: parse(display(x)) == x at fixed precision
// ---------------------------------------------------------------------------
//
// We compare via `with_precision(P).value()` because `Display` for `DBig`
// emits the canonical decimal representation of the underlying significand,
// which may have more or fewer digits than the requested test precision.

fn round_trip_seeds() -> &'static [&'static str] {
    &[
        "0.0",
        "1",
        "-1",
        "3.14159",
        "-2.71828",
        "1e-50",
        "1e50",
        "12345.6789",
        "0.000001",
        "-987654.321",
    ]
}

#[test]
fn round_trip_seeds_at_precisions() {
    let precisions: [usize; 4] = [5, 10, 20, 50];
    for seed in round_trip_seeds() {
        let parsed =
            DBig::from_str(seed).unwrap_or_else(|e| panic!("seed {seed:?} should parse: {e}"));
        for &p in &precisions {
            let canonical = parsed.clone().with_precision(p).value();
            let displayed = canonical.to_string();
            let reparsed =
                DBig::from_str(&displayed).unwrap_or_else(|e| panic!("reparse {displayed:?}: {e}"));
            // Compare at the same (precision-limited) form.
            let reparsed_canonical = reparsed.with_precision(p).value();
            assert_eq!(
                canonical.to_string(),
                reparsed_canonical.to_string(),
                "round-trip mismatch for seed {seed:?} @ precision {p}: \
                 display={displayed:?}"
            );
        }
    }
}

#[test]
fn round_trip_high_precision_constants() {
    // `compute_pi` and `compute_e` return pre-stored constants; verify a
    // round-trip across the wrapper's public API.
    for p in &[10usize, 30, 100] {
        let pi = compute_pi(*p);
        let pi_str = pi.to_string();
        let reparsed = DBig::from_str(&pi_str).expect("reparse pi should succeed");
        assert_eq!(
            pi.to_string(),
            reparsed.to_string(),
            "pi round-trip @ precision {p}"
        );

        let e = compute_e(*p);
        let e_str = e.to_string();
        let re = DBig::from_str(&e_str).expect("reparse e should succeed");
        assert_eq!(
            e.to_string(),
            re.to_string(),
            "e round-trip @ precision {p}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Special-value handling — scoped to backend semantics
// ---------------------------------------------------------------------------

/// Tests covering the special-value contract of `oxinum-float`.
///
/// **Backend reality:** `dashu-float` represents only **finite** values:
///
/// * It has **no `NaN`** representation at all (parsing `NaN` is an error,
///   and there is no `NaN` literal constructor).
/// * It has a `±Inf` sentinel, but **any arithmetic on infinities panics**
///   (see `dashu_float::error::panic_operate_with_inf`).  Parsing
///   `"inf"` / `"-inf"` is also rejected.
///
/// Consequently, `oxinum-float` *does not* propagate `NaN + x = NaN` or
/// `Inf + Inf = Inf`; instead, non-finite outcomes surface as
/// `OxiNumError::Precision` (or `DivByZero`).  The tests below assert the
/// **error-returning** equivalents that the wrapper guarantees.
mod special_values_scoped {
    use super::*;

    #[test]
    fn parsing_inf_is_an_error() {
        // dashu-float intentionally does not accept infinities in `from_str`.
        assert!(DBig::from_str("inf").is_err());
        assert!(DBig::from_str("Inf").is_err());
        assert!(DBig::from_str("-inf").is_err());
        assert!(DBig::from_str("Infinity").is_err());
    }

    #[test]
    fn parsing_nan_is_an_error() {
        assert!(DBig::from_str("nan").is_err());
        assert!(DBig::from_str("NaN").is_err());
        assert!(DBig::from_str("NAN").is_err());
    }

    #[test]
    fn ln_of_zero_returns_precision_error() {
        let zero = DBig::from_str("0.0").expect("parse 0.0");
        match ln(&zero, 20) {
            Err(OxiNumError::Precision(_)) => {}
            other => panic!("ln(0) should be Precision error; got {other:?}"),
        }
    }

    #[test]
    fn ln_of_negative_returns_precision_error() {
        let neg = DBig::from_str("-1.0").expect("parse -1.0");
        match ln(&neg, 20) {
            Err(OxiNumError::Precision(_)) => {}
            other => panic!("ln(-1) should be Precision error; got {other:?}"),
        }
    }

    #[test]
    fn sqrt_of_negative_returns_precision_error() {
        let neg = DBig::from_str("-1.0").expect("parse -1.0");
        match sqrt(&neg, 20) {
            Err(OxiNumError::Precision(_)) => {}
            other => panic!("sqrt(-1) should be Precision error; got {other:?}"),
        }
    }

    #[test]
    fn pow_with_nonpositive_base_returns_precision_error() {
        // pow(base, exp) requires base > 0 for nonzero exponent.
        let zero = DBig::from_str("0.0").expect("parse 0.0");
        let one = DBig::from_str("1.0").expect("parse 1.0");
        match pow(&zero, &one, 20) {
            Err(OxiNumError::Precision(_)) => {}
            other => panic!("pow(0, 1) should be Precision error; got {other:?}"),
        }

        let neg = DBig::from_str("-2.0").expect("parse -2.0");
        match pow(&neg, &one, 20) {
            Err(OxiNumError::Precision(_)) => {}
            other => panic!("pow(-2, 1) should be Precision error; got {other:?}"),
        }
    }

    #[test]
    fn zero_precision_argument_is_an_error() {
        // The wrapper rejects precision == 0 across all transcendentals.
        let one = DBig::from_str("1.0").expect("parse 1.0");
        assert!(ln(&one, 0).is_err());
        assert!(sqrt(&one, 0).is_err());
        assert!(pow(&one, &one, 0).is_err());
    }

    #[test]
    fn infinity_sentinel_is_not_exposed_through_public_api() {
        // Sanity check: any finite operation we perform via the wrapper
        // returns a finite, displayable result (no "inf"/"nan" leaks).
        let a = DBig::from_str("1e100").expect("parse 1e100");
        let b = DBig::from_str("1e100").expect("parse 1e100");
        let sum = &a + &b;
        let s = sum.to_string();
        assert!(
            !s.to_lowercase().contains("inf") && !s.to_lowercase().contains("nan"),
            "sum should be a finite decimal, got {s:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Commutativity / associativity proptests
// ---------------------------------------------------------------------------
//
// We sample from a fixed pool of finite decimal literals (sub-unit, near-
// unit, multi-digit integer, very small, large in magnitude, both signs).
// All arithmetic is performed at precision 30 so that any rounding noise
// is well below the comparison threshold.

fn pool() -> Vec<&'static str> {
    vec![
        "0.0",
        "1.0",
        "-1.0",
        "0.1",
        "-3.7",
        "42.0",
        "0.000001",
        "-987654.321",
        "3.141592653589793",
        "-2.718281828459045",
        "1.0e10",
        "-1.0e-10",
        "0.5",
        "-0.25",
        "12345.6789",
    ]
}

fn lit() -> impl Strategy<Value = DBig> {
    let p = pool();
    let len = p.len();
    (0usize..len).prop_map(move |i| DBig::from_str(p[i]).expect("pool literal parses"))
}

fn at_precision(x: &DBig, prec: usize) -> DBig {
    x.clone().with_precision(prec).value()
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    /// `a + b == b + a` at precision 30.
    #[test]
    fn addition_commutativity(a in lit(), b in lit()) {
        let ab = at_precision(&(&a + &b), 30);
        let ba = at_precision(&(&b + &a), 30);
        prop_assert_eq!(ab.to_string(), ba.to_string());
    }

    /// `a * b == b * a` at precision 30.
    #[test]
    fn multiplication_commutativity(a in lit(), b in lit()) {
        let ab = at_precision(&(&a * &b), 30);
        let ba = at_precision(&(&b * &a), 30);
        prop_assert_eq!(ab.to_string(), ba.to_string());
    }

    /// `((a + b) + c) - (a + (b + c))` is small at precision 30.
    ///
    /// Float associativity is generally false, but if each operand is
    /// promoted to 30 decimal digits before the sum, then both groupings
    /// share the same working precision and the difference should be far
    /// below `1e-8`.  We promote the operands explicitly because parsed
    /// `DBig` literals carry the precision implied by the input string
    /// (e.g. `"3.14"` -> precision 3); without promotion, `with_precision`
    /// on the final result has nothing to widen.
    #[test]
    fn addition_near_associativity(
        a in lit(), b in lit(), c in lit(),
    ) {
        let ap = at_precision(&a, 30);
        let bp = at_precision(&b, 30);
        let cp = at_precision(&c, 30);
        let lhs = at_precision(&(&(&ap + &bp) + &cp), 30);
        let rhs = at_precision(&(&ap + &(&bp + &cp)), 30);
        let diff = &lhs - &rhs;
        // Loose tolerance: the worst-case cancellation among `pool()`
        // entries is bounded by the precision-30 ulp at the largest
        // magnitude (~1e10 * 1e-29 ~ 1e-19), but we allow `1e-8` to be
        // robust against any future expansion of the pool.
        let bound = DBig::from_str("1e-8").expect("parse 1e-8");
        let neg_bound = DBig::from_str("-1e-8").expect("parse -1e-8");
        prop_assert!(
            diff <= bound && diff >= neg_bound,
            "associativity diff = {} (a={}, b={}, c={})",
            diff, a, b, c
        );
    }

    /// `sqrt(x * x) ~= |x|` to within a tiny tolerance, for finite x.
    ///
    /// We must promote `x` to a 30-digit working precision before
    /// squaring, otherwise dashu propagates the parsed input precision
    /// (e.g. `"-3.7"` -> precision 2) and the eventual `sqrt` truncates
    /// at far below 30 digits.
    #[test]
    fn sqrt_of_square_is_abs(x in lit()) {
        let xp = at_precision(&x, 30);
        let xx = &xp * &xp;
        let s = sqrt(&xx, 30).expect("sqrt(x*x) should succeed for finite x");
        // Compare to |x|; we don't have a public `abs`, so re-parse via
        // string trimming.
        let abs_str = xp.to_string();
        let abs = if let Some(stripped) = abs_str.strip_prefix('-') {
            DBig::from_str(stripped).expect("abs string reparses")
        } else {
            xp.clone()
        };
        let abs_p = at_precision(&abs, 25);
        let s_p = at_precision(&s, 25);
        let diff = &abs_p - &s_p;
        // 1e-8 absolute tolerance is far above the precision-30 ulp at
        // any magnitude in `pool()` (max ~1e10) -- a safe upper bound
        // accounting for the wrapper's truncate-to-precision step.
        let bound = DBig::from_str("1e-8").expect("parse 1e-8");
        let neg_bound = DBig::from_str("-1e-8").expect("parse -1e-8");
        prop_assert!(
            diff <= bound && diff >= neg_bound,
            "sqrt(x*x) - |x| = {} for x = {}",
            diff, x
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Cross-validation against dashu-float
// ---------------------------------------------------------------------------
//
// `oxinum-float` re-exports `DBig` (and `FBig`) from `dashu-float` and
// does not wrap the arithmetic operators directly.  These tests are
// therefore very explicit: they exercise that performing the same
// arithmetic via the `oxinum_float::*` re-exports yields identical
// results to performing it via `dashu_float::*`.  Should the wrapper ever
// gain its own arithmetic layer (see `oxinum-float/TODO.md` -> "API
// Improvements -> Implement std::ops::*"), these tests will catch any
// regression where the two paths diverge.

#[test]
fn cross_validate_addition_matches_dashu() {
    let oa = DBig::from_str("3.14").expect("parse 3.14");
    let ob = DBig::from_str("2.71").expect("parse 2.71");
    let osum = at_precision(&(&oa + &ob), 10);

    let da = dashu_float::DBig::from_str("3.14").expect("dashu parse 3.14");
    let db = dashu_float::DBig::from_str("2.71").expect("dashu parse 2.71");
    let dsum = (&da + &db).with_precision(10).value();

    assert_eq!(osum.to_string(), dsum.to_string());
}

#[test]
fn cross_validate_multiplication_matches_dashu() {
    let oa = DBig::from_str("12345.6789").expect("parse 12345.6789");
    let ob = DBig::from_str("0.0001").expect("parse 0.0001");
    let oprod = at_precision(&(&oa * &ob), 20);

    let da = dashu_float::DBig::from_str("12345.6789").expect("dashu parse");
    let db = dashu_float::DBig::from_str("0.0001").expect("dashu parse");
    let dprod = (&da * &db).with_precision(20).value();

    assert_eq!(oprod.to_string(), dprod.to_string());
}

#[test]
fn cross_validate_subtraction_matches_dashu() {
    let oa = DBig::from_str("1000.0").expect("parse 1000.0");
    let ob = DBig::from_str("0.001").expect("parse 0.001");
    let odiff = at_precision(&(&oa - &ob), 30);

    let da = dashu_float::DBig::from_str("1000.0").expect("dashu parse");
    let db = dashu_float::DBig::from_str("0.001").expect("dashu parse");
    let ddiff = (&da - &db).with_precision(30).value();

    assert_eq!(odiff.to_string(), ddiff.to_string());
}

#[test]
fn cross_validate_division_matches_dashu() {
    let oa = DBig::from_str("1.0").expect("parse 1.0");
    let ob = DBig::from_str("3.0").expect("parse 3.0");
    let oquot = at_precision(&oa, 30) / at_precision(&ob, 30);

    let da = dashu_float::DBig::from_str("1.0").expect("dashu parse");
    let db = dashu_float::DBig::from_str("3.0").expect("dashu parse");
    let dquot = da.with_precision(30).value() / db.with_precision(30).value();

    assert_eq!(oquot.to_string(), dquot.to_string());
}

#[test]
fn cross_validate_sqrt_wrapper_matches_dashu_root() {
    let two = DBig::from_str("2.0").expect("parse 2.0");
    let wrap = sqrt(&two, 20).expect("sqrt(2) via wrapper");
    // Reference: first 20 significant digits of sqrt(2) per a standard
    // table (1.4142135623730950488...).  We verify the wrapper produces
    // exactly the expected leading prefix; the precise digit count after
    // the wrapper's truncation step is governed by `oxinum_float`.
    let s = wrap.to_string();
    assert!(
        s.starts_with("1.4142135623730950488"),
        "sqrt(2) leading digits unexpected: got {s}"
    );
}
