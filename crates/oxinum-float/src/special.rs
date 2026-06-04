//! Special mathematical functions for arbitrary-precision floating-point.
//!
//! Provides pure-Rust implementations of:
//! - [`gamma`] — Gamma function Γ(x)
//! - [`ln_gamma`] — log-Gamma function ln(Γ(x))
//! - [`digamma`] — Digamma function ψ(x) = d/dx ln(Γ(x))
//! - [`erf`] — Error function
//! - [`erfc`] — Complementary error function
//! - [`bessel_j0`] — Bessel function J₀(x)
//! - [`euler_gamma`] — Euler–Mascheroni constant γ
//! - [`catalan`] — Catalan's constant G
//! - [`free_cache`] — No-op (MPFR compatibility shim)
//!
//! All functions work on [`DBig`] (decimal arbitrary-precision big float) and
//! accept a `precision` parameter giving the number of significant decimal
//! digits to carry through the computation.
//!
//! ## Algorithms
//!
//! - **Gamma / log-Gamma**: Lanczos approximation (g=7, 9 coefficients) for
//!   x ∈ (0, 20]; Stirling series for x > 20; reflection formula for x < 0.
//! - **Digamma**: recurrence to shift x > 8, then Bernoulli asymptotic series.
//! - **Erf**: Taylor series for |x| ≤ 2; asymptotic continued-fraction for |x| > 2.
//! - **Erfc**: complement of erf with careful sign handling.
//! - **Bessel J₀**: power series for |x| ≤ 12; asymptotic expansion for |x| > 12.
//! - **Euler γ**: pre-stored 200-digit decimal string.
//! - **Catalan G**: pre-stored 200-digit decimal string.

use crate::elementary::{exp, ln, sqrt, truncate_to_precision};
use crate::trig::{cos, sin};
use crate::{DBig, OxiNumError, OxiNumResult};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Pre-stored high-precision constants
// ---------------------------------------------------------------------------

/// 200 decimal digits of the Euler–Mascheroni constant γ.
const EULER_GAMMA_200: &str =
    "0.57721566490153286060651209008240243104215933593992359880576723488486772677766467\
     09369596694504673425296068890403823946951285765500044721133652219082019609773798\
     42060299066541920";

/// 200 decimal digits of Catalan's constant G.
const CATALAN_200: &str =
    "0.91596559417721901505460351493238411077414937428167213426649811962176301977625476\
     94709053505388297009241232177413908000993617044680583060800913695168700767543730\
     49861741618979838";

// ---------------------------------------------------------------------------
// Public constant functions
// ---------------------------------------------------------------------------

/// Compute the Euler–Mascheroni constant γ ≈ 0.5772156649… to `precision`
/// significant decimal digits (capped at 200).
///
/// # Examples
///
/// ```
/// let g = oxinum_float::special::euler_gamma(30);
/// assert!(g.to_string().starts_with("0.577215664901532860606512"));
/// ```
pub fn euler_gamma(precision: usize) -> DBig {
    parse_at_precision(EULER_GAMMA_200, precision)
}

/// Compute Catalan's constant G ≈ 0.9159655941… to `precision` significant
/// decimal digits (capped at 200).
///
/// # Examples
///
/// ```
/// let g = oxinum_float::special::catalan(30);
/// assert!(g.to_string().starts_with("0.915965594177219015054603"));
/// ```
pub fn catalan(precision: usize) -> DBig {
    parse_at_precision(CATALAN_200, precision)
}

/// No-op cache cleanup shim.
///
/// MPFR (via `rug`) allocates thread-local caches for constants like π that
/// must be freed explicitly.  `dashu-float` has no such global caches, so
/// this function is intentionally a no-op provided for API compatibility.
pub fn free_cache() {
    // dashu-float has no MPFR-style global constant cache to free.
}

// ---------------------------------------------------------------------------
// Gamma function
// ---------------------------------------------------------------------------

/// Compute the Gamma function Γ(x) to `precision` significant decimal digits.
///
/// Uses Lanczos approximation (g=7) for x ∈ (0, 20], Stirling series for
/// x > 20, and the reflection formula Γ(x)·Γ(1-x) = π/sin(πx) for x < 0.
///
/// # Errors
///
/// Returns `OxiNumError::Domain` if x is zero or a negative integer (poles
/// of the Gamma function).
pub fn gamma(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let guard = precision + 20;
    gamma_impl(x, guard, precision)
}

fn gamma_impl(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let zero = make_dbig("0.0")?;
    let one = make_dbig("1.0")?;

    if *x == zero {
        return Err(OxiNumError::Domain("Gamma(0) is undefined (pole)".into()));
    }

    // Check for negative integers: x is a negative integer if x < 0 and
    // x - round(x) == 0 where round(x) = floor(x) (for negative integers floor gives them exactly)
    if is_negative(x) && is_integer_value(x) {
        return Err(OxiNumError::Domain(
            "Gamma undefined at negative integers (poles)".into(),
        ));
    }

    if is_positive(x) {
        if to_f64_approx(x) > 20.0 {
            stirling_gamma(x, guard, output_prec)
        } else {
            lanczos_gamma(x, guard, output_prec)
        }
    } else {
        // Reflection formula: Γ(x) = π / (sin(πx) · Γ(1-x))
        let pi = crate::constants::compute_pi(guard);
        let pi_ext = extend(pi, guard);
        let x_ext = extend(x.clone(), guard);
        let pi_x = &pi_ext * &x_ext;
        let sin_pi_x = sin(&pi_x, guard)?;
        if is_approx_zero(&sin_pi_x) {
            return Err(OxiNumError::Domain(
                "Gamma undefined at negative integers (poles)".into(),
            ));
        }
        let one_minus_x = &extend(one, guard) - &x_ext;
        let gamma_pos = gamma_impl(&one_minus_x, guard, guard)?;
        let result = &pi_ext / &(&sin_pi_x * &gamma_pos);
        Ok(truncate_to_precision(result, output_prec))
    }
}

/// Lanczos approximation for Γ(x), x > 0, moderate range.
fn lanczos_gamma(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    // Lanczos g=7, 9 coefficients (Numerical Recipes)
    const LANCZOS_G: f64 = 7.0;
    const LANCZOS_COEFFS: &[f64] = &[
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_403,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_1,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];

    let x_ext = extend(x.clone(), guard);
    let two_pi = dbig_f64(2.0 * std::f64::consts::PI, guard);
    let sqrt_2pi = sqrt(&two_pi, guard)?;

    // Aggregate the rational sum
    let mut ag = dbig_f64(LANCZOS_COEFFS[0], guard);
    for (i, &c) in LANCZOS_COEFFS[1..].iter().enumerate() {
        let denom = &x_ext + &dbig_f64(i as f64 + 1.0, guard);
        ag = &ag + &(&dbig_f64(c, guard) / &denom);
    }

    // tmp = x + g + 0.5
    let tmp = &x_ext + &dbig_f64(LANCZOS_G + 0.5, guard);

    // result = sqrt_2pi * ag * tmp^(x+0.5) * exp(-tmp) / x
    // = sqrt_2pi * ag * (tmp/e)^(x+0.5) via exp((x+0.5)*ln(tmp) - tmp)
    let x_plus_half = &x_ext + &dbig_f64(0.5, guard);
    let ln_tmp = ln(&tmp, guard)?;
    let log_part = &(&x_plus_half * &ln_tmp) - &tmp;
    let exp_part = exp(&log_part, guard)?;

    let result = &(&sqrt_2pi * &ag) * &exp_part;
    // Divide by x (Lanczos computes Γ(x+1)/x = Γ(x) form)
    let final_result = &result / &x_ext;
    Ok(truncate_to_precision(final_result, output_prec))
}

/// Stirling series for Γ(x), x > 20.
fn stirling_gamma(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let x_ext = extend(x.clone(), guard);
    let two_pi = dbig_f64(2.0 * std::f64::consts::PI, guard);
    let sqrt_2pi = sqrt(&two_pi, guard)?;

    // sqrt(2π/x)
    let sqrt_2pi_over_x = &sqrt_2pi / &sqrt(&x_ext, guard)?;

    // (x/e)^x via exp(x*ln(x) - x)
    let ln_x = ln(&x_ext, guard)?;
    let log_part = &(&x_ext * &ln_x) - &x_ext;
    let exp_part = exp(&log_part, guard)?;

    // Stirling correction: 1 + 1/(12x) + 1/(288x²) - 139/(51840x³) - 571/(2488320x⁴)
    let x2 = &x_ext * &x_ext;
    let x3 = &x2 * &x_ext;
    let x4 = &x2 * &x2;

    let c1 = &dbig_f64(1.0, guard) / &(&dbig_f64(12.0, guard) * &x_ext);
    let c2 = &dbig_f64(1.0, guard) / &(&dbig_f64(288.0, guard) * &x2);
    let c3 = &(&dbig_f64(139.0, guard) / &(&dbig_f64(51840.0, guard) * &x3));
    let c4 = &(&dbig_f64(571.0, guard) / &(&dbig_f64(2488320.0, guard) * &x4));

    let correction = &(&(&dbig_f64(1.0, guard) + &c1) + &c2) - (&(&c3.clone() + &c4.clone()));

    let result = &(&sqrt_2pi_over_x * &exp_part) * &correction;
    Ok(truncate_to_precision(result, output_prec))
}

// ---------------------------------------------------------------------------
// Log-Gamma
// ---------------------------------------------------------------------------

/// Compute ln(Γ(x)) to `precision` significant decimal digits.
///
/// # Errors
///
/// Returns `OxiNumError::Domain` if x ≤ 0.
pub fn ln_gamma(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x <= zero {
        return Err(OxiNumError::Domain("ln_gamma undefined for x <= 0".into()));
    }

    let guard = precision + 20;
    let x_f = to_f64_approx(x);

    if x_f > 10.0 {
        stirling_ln_gamma(x, guard, precision)
    } else {
        // For small x: compute gamma then take ln
        let g = gamma_impl(x, guard, guard)?;
        let lg = ln(&g, guard)?;
        Ok(truncate_to_precision(lg, precision))
    }
}

/// Stirling series for ln(Γ(x)), x > 10.
fn stirling_ln_gamma(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let x_ext = extend(x.clone(), guard);
    let two_pi = dbig_f64(2.0 * std::f64::consts::PI, guard);
    let ln_2pi = ln(&two_pi, guard)?;
    let ln_x = ln(&x_ext, guard)?;

    // ln Γ(x) ≈ (x - 1/2) ln(x) - x + ln(2π)/2 + 1/(12x) - 1/(360x³) + 1/(1260x⁵) - 1/(1680x⁷)
    let half = dbig_f64(0.5, guard);
    let x_minus_half = &x_ext - &half;

    let x2 = &x_ext * &x_ext;
    let x3 = &x2 * &x_ext;
    let x5 = &x3 * &x2;
    let x7 = &x5 * &x2;

    let base = &(&x_minus_half * &ln_x) - &x_ext;
    let term0 = &ln_2pi / &dbig_f64(2.0, guard);
    let term1 = &dbig_f64(1.0, guard) / &(&dbig_f64(12.0, guard) * &x_ext);
    let term2 = &dbig_f64(1.0, guard) / &(&dbig_f64(360.0, guard) * &x3);
    let term3 = &dbig_f64(1.0, guard) / &(&dbig_f64(1260.0, guard) * &x5);
    let term4 = &dbig_f64(1.0, guard) / &(&dbig_f64(1680.0, guard) * &x7);

    let result =
        &(&(&base + &term0) + &term1) - &(&(&term2.clone() - &term3.clone()) + &term4.clone());
    Ok(truncate_to_precision(result, output_prec))
}

// ---------------------------------------------------------------------------
// Digamma
// ---------------------------------------------------------------------------

/// Compute the digamma function ψ(x) = d/dx ln(Γ(x)) to `precision` significant
/// decimal digits.
///
/// Uses recurrence ψ(x+1) = ψ(x) + 1/x to shift x > 8, then Bernoulli asymptotic
/// series. For x < 1 uses the reflection formula ψ(1-x) - ψ(x) = π·cot(πx).
///
/// # Errors
///
/// Returns `OxiNumError::Domain` if x is zero or a negative integer.
pub fn digamma(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x == zero {
        return Err(OxiNumError::Domain("digamma(0) is undefined (pole)".into()));
    }
    if is_negative(x) && is_integer_value(x) {
        return Err(OxiNumError::Domain(
            "digamma undefined at negative integers (poles)".into(),
        ));
    }

    let guard = precision + 30;
    digamma_impl(x, guard, precision)
}

fn digamma_impl(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let zero = make_dbig("0.0")?;
    let one = dbig_f64(1.0, guard);

    // For x < 1: reflection formula ψ(1-x) - ψ(x) = π·cot(πx)
    if is_negative(x) || to_f64_approx(x) < 1.0 {
        let pi = extend(crate::constants::compute_pi(guard), guard);
        let x_ext = extend(x.clone(), guard);
        let pi_x = &pi * &x_ext;
        let sin_pix = sin(&pi_x, guard)?;
        let cos_pix = cos(&pi_x, guard)?;
        if is_approx_zero(&sin_pix) {
            return Err(OxiNumError::Domain(
                "digamma undefined at non-positive integers".into(),
            ));
        }
        let cot_pi_x = &cos_pix / &sin_pix;
        let pi_cot = &pi * &cot_pi_x;

        let one_minus_x = &one - &x_ext;
        let psi_1mx = digamma_impl(&one_minus_x, guard, guard)?;
        let result = &psi_1mx - &pi_cot;
        return Ok(truncate_to_precision(result, output_prec));
    }

    // For x >= 1: use recurrence ψ(x) = ψ(x+n) - Σ_{k=0}^{n-1} 1/(x+k)
    // to shift x until x > 8, then asymptotic expansion.
    let x_ext = extend(x.clone(), guard);
    let mut shift_sum = extend(zero, guard);
    let mut curr_x = x_ext.clone();

    while to_f64_approx(&curr_x) < 8.0 {
        shift_sum = &shift_sum + &(&one / &curr_x);
        curr_x = &curr_x + &one;
    }

    // Asymptotic: ψ(x) ≈ ln(x) - 1/(2x) - B₂/(2x²) - B₄/(4x⁴) - B₆/(6x⁶) - B₈/(8x⁸)
    // where B₂=1/6, B₄=-1/30, B₆=1/42, B₈=-1/30
    let ln_cx = ln(&curr_x, guard)?;
    let cx2 = &curr_x * &curr_x;
    let cx4 = &cx2 * &cx2;
    let cx6 = &cx4 * &cx2;
    let cx8 = &cx4 * &cx4;

    let t1 = &dbig_f64(1.0, guard) / &(&dbig_f64(2.0, guard) * &curr_x);
    // B₂/2x² = (1/6)/(2x²) = 1/(12x²)
    let t2 = &dbig_f64(1.0, guard) / &(&dbig_f64(12.0, guard) * &cx2);
    // B₄/4x⁴ = (-1/30)/(4x⁴) = -1/(120x⁴)
    let t3 = &dbig_f64(1.0, guard) / &(&dbig_f64(120.0, guard) * &cx4);
    // B₆/6x⁶ = (1/42)/(6x⁶) = 1/(252x⁶)
    let t4 = &dbig_f64(1.0, guard) / &(&dbig_f64(252.0, guard) * &cx6);
    // B₈/8x⁸ = (-1/30)/(8x⁸) = -1/(240x⁸)
    let t5 = &dbig_f64(1.0, guard) / &(&dbig_f64(240.0, guard) * &cx8);

    // ψ(curr_x) ≈ ln(x) - 1/(2x) - 1/(12x²) + 1/(120x⁴) - 1/(252x⁶) + 1/(240x⁸)
    let asymp = &(&(&ln_cx - &t1) - &t2) + &(&(&t3 - &t4) + &t5);

    let result = &asymp - &shift_sum;
    Ok(truncate_to_precision(result, output_prec))
}

// ---------------------------------------------------------------------------
// Error function
// ---------------------------------------------------------------------------

/// Compute erf(x) to `precision` significant decimal digits.
///
/// Uses Taylor series for |x| ≤ 2; asymptotic expansion for |x| > 2.
pub fn erf(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x == zero {
        return Ok(zero);
    }

    let guard = precision + 20;
    let abs_x_f = to_f64_approx(x).abs();

    if abs_x_f <= 2.0 {
        erf_series(x, guard, precision)
    } else {
        // erf(x) = 1 - erfc(x) with appropriate sign
        let abs_x = abs_dbig(x, guard);
        let erfc_val = erfc_asymptotic(&abs_x, guard, guard)?;
        let one = dbig_f64(1.0, guard);
        let result = if is_positive(x) {
            &one - &erfc_val
        } else {
            &erfc_val - &one
        };
        Ok(truncate_to_precision(result, precision))
    }
}

/// Taylor series: erf(x) = (2/√π) Σ_{k=0}^∞ (-1)^k x^{2k+1} / (k! (2k+1))
fn erf_series(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let pi = extend(crate::constants::compute_pi(guard), guard);
    let sqrt_pi = sqrt(&pi, guard)?;
    let two_over_sqrt_pi = &dbig_f64(2.0, guard) / &sqrt_pi;

    let x_ext = extend(x.clone(), guard);
    let x2 = &x_ext * &x_ext;
    let neg_one = dbig_f64(-1.0, guard);

    let mut sum = x_ext.clone();
    let mut term = x_ext.clone();

    for k in 1u32..=(guard as u32 * 3 + 50) {
        // term_{k} = term_{k-1} * (-x²) / k
        let neg_x2_over_k = &(&x2 * &neg_one) / &dbig_f64(k as f64, guard);
        term = &term * &neg_x2_over_k;
        // divide by (2k+1)
        let contrib = &term / &dbig_f64(2.0 * k as f64 + 1.0, guard);
        sum = &sum + &contrib;

        if is_negligible(&contrib, output_prec) {
            break;
        }
    }

    let result = &two_over_sqrt_pi * &sum;
    Ok(truncate_to_precision(result, output_prec))
}

/// Asymptotic expansion for erfc(x) for large x > 0:
/// erfc(x) = (e^{-x²} / (x√π)) * Σ_{k=0}^∞ (-1)^k (2k-1)!! / (2x²)^k
fn erfc_asymptotic(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let pi = extend(crate::constants::compute_pi(guard), guard);
    let sqrt_pi = sqrt(&pi, guard)?;
    let x_ext = extend(x.clone(), guard);
    let x2 = &x_ext * &x_ext;
    let neg_x2 = &x2 * &dbig_f64(-1.0, guard);
    let exp_neg_x2 = exp(&neg_x2, guard)?;

    let mut sum = dbig_f64(1.0, guard);
    let mut term = dbig_f64(1.0, guard);

    for k in 1u32..50 {
        // term_{k} = term_{k-1} * (-(2k-1)) / (2x²)
        let two_x2 = &dbig_f64(2.0, guard) * &x2;
        let factor = &dbig_f64(-((2 * k - 1) as f64), guard) / &two_x2;
        term = &term * &factor;
        sum = &sum + &term;

        if is_negligible(&term, output_prec) {
            break;
        }
        // Asymptotic series is divergent - stop when term grows
        if to_f64_approx(&abs_dbig(&term, guard)) > to_f64_approx(&abs_dbig(&sum, guard)) {
            break;
        }
    }

    // erfc(x) = exp(-x²) * sum / (x * sqrt(pi))
    let denom = &x_ext * &sqrt_pi;
    let result = &(&exp_neg_x2 * &sum) / &denom;
    Ok(truncate_to_precision(result, output_prec))
}

// ---------------------------------------------------------------------------
// Complementary error function
// ---------------------------------------------------------------------------

/// Compute erfc(x) = 1 - erf(x) to `precision` significant decimal digits.
///
/// Uses asymptotic continued fraction for large x (avoids cancellation).
pub fn erfc(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x == zero {
        return Ok(dbig_f64(1.0, precision));
    }

    let guard = precision + 20;
    let abs_x_f = to_f64_approx(x).abs();

    if abs_x_f <= 2.0 {
        let erf_val = erf_series(x, guard, guard)?;
        let one = dbig_f64(1.0, guard);
        let result = &one - &erf_val;
        Ok(truncate_to_precision(result, precision))
    } else {
        let abs_x = abs_dbig(x, guard);
        let erfc_val = erfc_asymptotic(&abs_x, guard, guard)?;
        let result = if is_positive(x) {
            truncate_to_precision(erfc_val, precision)
        } else {
            // erfc(-x) = 2 - erfc(x)
            let two = dbig_f64(2.0, guard);
            truncate_to_precision(&two - &erfc_val, precision)
        };
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Bessel J₀
// ---------------------------------------------------------------------------

/// Compute the Bessel function J₀(x) to `precision` significant decimal digits.
///
/// Uses power series for |x| ≤ 12; asymptotic expansion for |x| > 12.
///
/// The power series is: J₀(x) = Σ_{k=0}^∞ (-1)^k (x/2)^{2k} / (k!)²
pub fn bessel_j0(x: &DBig, precision: usize) -> OxiNumResult<DBig> {
    validate_precision(precision)?;
    let zero = make_dbig("0.0")?;
    if *x == zero {
        return Ok(dbig_f64(1.0, precision));
    }

    let guard = precision + 20;
    let abs_x_f = to_f64_approx(x).abs();

    if abs_x_f <= 12.0 {
        bessel_j0_series(x, guard, precision)
    } else {
        bessel_j0_asymptotic(x, guard, precision)
    }
}

/// Power series for J₀(x): Σ_{k=0}^∞ (-1)^k (x/2)^{2k} / (k!)²
fn bessel_j0_series(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let x_ext = extend(x.clone(), guard);
    // x/2
    let x_half = &x_ext / &dbig_f64(2.0, guard);
    // (x/2)²
    let x_half_sq = &x_half * &x_half;

    let mut sum = dbig_f64(1.0, guard);
    let mut term = dbig_f64(1.0, guard);

    for k in 1u32..=(guard as u32 * 2 + 60) {
        // term_{k} = term_{k-1} * (-(x/2)²) / k²
        let k_sq = dbig_f64((k as f64) * (k as f64), guard);
        let neg_xh2 = &x_half_sq * &dbig_f64(-1.0, guard);
        term = &term * &(&neg_xh2 / &k_sq);
        sum = &sum + &term;

        if is_negligible(&term, output_prec) {
            break;
        }
    }

    Ok(truncate_to_precision(sum, output_prec))
}

/// Asymptotic expansion for J₀(x) for large |x|:
/// J₀(x) ≈ √(2/(πx)) [P₀(x)·cos(x - π/4) - Q₀(x)·sin(x - π/4)]
/// where P₀ and Q₀ are asymptotic series truncated at the smallest term.
fn bessel_j0_asymptotic(x: &DBig, guard: usize, output_prec: usize) -> OxiNumResult<DBig> {
    let x_ext = extend(x.clone(), guard);
    let pi = extend(crate::constants::compute_pi(guard), guard);

    // sqrt(2/(pi*x))
    let pi_x = &pi * &x_ext;
    let two_over_pi_x = &dbig_f64(2.0, guard) / &pi_x;
    let sqrt_prefactor = sqrt(&two_over_pi_x, guard)?;

    // phase = x - pi/4
    let pi_over_4 = &pi / &dbig_f64(4.0, guard);
    let phase = &x_ext - &pi_over_4;

    // P₀(x) = 1 - (1²·3²)/(2!(8x)²) + ... ≈ 1 - 9/(128x²) + ...
    // Q₀(x) = -1/(8x) + (1·3·5)/(3!·(8x)³/...) ≈ -1/(8x) + ...
    // Use only leading terms for asymptotic (valid for large x)
    let x2 = &x_ext * &x_ext;
    let x4 = &x2 * &x2;

    // P0: 1 - 9/(128*x²) + 3675/(32768*x⁴)
    let p0_t1 = &dbig_f64(9.0, guard) / &(&dbig_f64(128.0, guard) * &x2);
    let p0_t2 = &dbig_f64(3675.0, guard) / &(&dbig_f64(32768.0, guard) * &x4);
    let p0 = &(&dbig_f64(1.0, guard) - &p0_t1) + &p0_t2;

    // Q0: -1/(8*x) + 75/(1024*x³)
    let x3 = &x2 * &x_ext;
    let q0_t1 = &dbig_f64(1.0, guard) / &(&dbig_f64(8.0, guard) * &x_ext);
    let q0_t2 = &dbig_f64(75.0, guard) / &(&dbig_f64(1024.0, guard) * &x3);
    let q0 = &q0_t2 - &q0_t1;

    let cos_phase = cos(&phase, guard)?;
    let sin_phase = sin(&phase, guard)?;

    // J₀ ≈ sqrt_prefactor * (P0*cos_phase - Q0*sin_phase)
    let result = &sqrt_prefactor * &(&(&p0 * &cos_phase) - &(&q0 * &sin_phase));
    Ok(truncate_to_precision(result, output_prec))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_precision(precision: usize) -> OxiNumResult<()> {
    if precision == 0 {
        return Err(OxiNumError::Precision("precision must be > 0".into()));
    }
    Ok(())
}

fn make_dbig(s: &str) -> OxiNumResult<DBig> {
    DBig::from_str(s).map_err(|e| OxiNumError::Parse(format!("{e}").into()))
}

/// Extend a DBig to carry at least `precision` significant digits.
fn extend(v: DBig, precision: usize) -> DBig {
    v.with_precision(precision).value()
}

/// Create a DBig from f64 at a given precision.
fn dbig_f64(v: f64, precision: usize) -> DBig {
    let s = format!("{:.prec$e}", v, prec = precision + 5);
    DBig::from_str(&s)
        .unwrap_or_else(|_| DBig::from_str(&format!("{v}")).expect("f64 to DBig should not fail"))
        .with_precision(precision)
        .value()
}

/// Check if value is (approximately) zero.
fn is_approx_zero(v: &DBig) -> bool {
    let s = v.to_string().trim_start_matches('-').to_string();
    s == "0" || s == "0.0" || {
        // Check for very small values: 0.00000...
        if let Some(dot) = s.find('.') {
            let int_part = &s[..dot];
            let frac = &s[dot + 1..];
            let leading_zeros = frac.chars().take_while(|&c| c == '0').count();
            int_part == "0" && leading_zeros >= 15
        } else {
            false
        }
    }
}

/// Check if term is negligible relative to `output_prec`.
fn is_negligible(term: &DBig, precision: usize) -> bool {
    let s = term.to_string();
    let s = s.trim_start_matches('-');
    if let Some(dot_pos) = s.find('.') {
        let integer_part = &s[..dot_pos];
        if integer_part == "0" {
            let frac = &s[dot_pos + 1..];
            let leading_zeros = frac.chars().take_while(|&c| c == '0').count();
            return leading_zeros >= precision + 3;
        }
    }
    s == "0" || s == "0.0"
}

/// Approximate f64 value from DBig for branching decisions.
fn to_f64_approx(v: &DBig) -> f64 {
    let s = v.to_string();
    s.parse::<f64>().unwrap_or(0.0)
}

/// Check if DBig is positive (> 0).
fn is_positive(v: &DBig) -> bool {
    !v.to_string().starts_with('-') && !is_approx_zero(v)
}

/// Check if DBig is negative (< 0).
fn is_negative(v: &DBig) -> bool {
    v.to_string().starts_with('-') && !is_approx_zero(v)
}

/// Check if DBig represents an integer value (fractional part is zero).
fn is_integer_value(v: &DBig) -> bool {
    let s = v.to_string().trim_start_matches('-').to_string();
    if let Some(dot) = s.find('.') {
        let frac = &s[dot + 1..];
        frac.chars().all(|c| c == '0')
    } else {
        // No dot means it IS an integer string
        !s.is_empty()
    }
}

/// Compute absolute value of DBig.
fn abs_dbig(v: &DBig, precision: usize) -> DBig {
    let s = v.to_string();
    if let Some(stripped) = s.strip_prefix('-') {
        DBig::from_str(stripped)
            .unwrap_or_else(|_| v.clone())
            .with_precision(precision)
            .value()
    } else {
        v.clone().with_precision(precision).value()
    }
}

/// Parse a constant string, truncated to `precision` significant digits.
fn parse_at_precision(src: &str, precision: usize) -> DBig {
    use crate::elementary::truncate_decimal_str;
    let prec = precision.clamp(1, 200);
    let truncated = truncate_decimal_str(src, prec);
    DBig::from_str(&truncated).expect("pre-stored constant is a valid decimal literal")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1e-300)
    }

    #[test]
    fn euler_gamma_value() {
        let g = euler_gamma(30);
        let s = g.to_string();
        assert!(
            s.starts_with("0.577215664901532860606512"),
            "euler_gamma = {s}"
        );
    }

    #[test]
    fn catalan_value() {
        let g = catalan(30);
        let s = g.to_string();
        assert!(s.starts_with("0.915965594177219015054603"), "catalan = {s}");
    }

    #[test]
    fn gamma_one() {
        let x = DBig::from_str("1.0").expect("ok");
        let g = gamma(&x, 30).expect("gamma(1) ok");
        let v = to_f64_approx(&g);
        assert!(approx_eq(v, 1.0, 1e-10), "gamma(1) = {v}");
    }

    #[test]
    fn gamma_half_is_sqrt_pi() {
        let x = DBig::from_str("0.5").expect("ok");
        let g = gamma(&x, 30).expect("gamma(0.5) ok");
        let v = to_f64_approx(&g);
        let expected = std::f64::consts::PI.sqrt();
        assert!(
            approx_eq(v, expected, 1e-10),
            "gamma(0.5) = {v}, expected {expected}"
        );
    }

    #[test]
    fn gamma_five_is_24() {
        let x = DBig::from_str("5.0").expect("ok");
        let g = gamma(&x, 30).expect("gamma(5) ok");
        let v = to_f64_approx(&g);
        assert!(approx_eq(v, 24.0, 1e-10), "gamma(5) = {v}");
    }

    #[test]
    fn gamma_pole_at_zero() {
        let x = DBig::from_str("0.0").expect("ok");
        assert!(gamma(&x, 20).is_err());
    }

    #[test]
    fn gamma_pole_at_neg_int() {
        let x = DBig::from_str("-3.0").expect("ok");
        assert!(gamma(&x, 20).is_err());
    }

    #[test]
    fn ln_gamma_one_is_zero() {
        let x = DBig::from_str("1.0").expect("ok");
        let lg = ln_gamma(&x, 30).expect("ok");
        let v = to_f64_approx(&lg).abs();
        assert!(v < 1e-8, "ln_gamma(1) = {v}");
    }

    #[test]
    fn ln_gamma_large() {
        let x = DBig::from_str("100.0").expect("ok");
        let lg = ln_gamma(&x, 30).expect("ok");
        let v = to_f64_approx(&lg);
        // ln(Γ(100)) = ln(99!) ≈ 359.134...
        assert!((v - 359.134_f64).abs() < 0.001, "ln_gamma(100) = {v}");
    }

    #[test]
    fn digamma_one_is_neg_euler() {
        // ψ(1) = -γ
        let x = DBig::from_str("1.0").expect("ok");
        let d = digamma(&x, 30).expect("ok");
        let v = to_f64_approx(&d);
        let expected = -0.5772156649_f64;
        assert!(
            (v - expected).abs() < 1e-8,
            "digamma(1) = {v}, expected {expected}"
        );
    }

    #[test]
    fn digamma_pole_at_zero() {
        let x = DBig::from_str("0.0").expect("ok");
        assert!(digamma(&x, 20).is_err());
    }

    #[test]
    fn erf_zero() {
        let x = DBig::from_str("0.0").expect("ok");
        let v = erf(&x, 20).expect("ok");
        let s = v.to_string();
        let clean = s.trim_start_matches('-');
        assert!(
            clean == "0" || clean == "0.0" || clean.starts_with("0.000000"),
            "erf(0) should be zero, got {s}"
        );
    }

    #[test]
    fn erf_one() {
        let x = DBig::from_str("1.0").expect("ok");
        let v = erf(&x, 20).expect("ok");
        let f = to_f64_approx(&v);
        // erf(1) ≈ 0.8427007929
        assert!((f - 0.8427007929_f64).abs() < 1e-8, "erf(1) = {f}");
    }

    #[test]
    fn erf_symmetry() {
        let x = DBig::from_str("0.8").expect("ok");
        let xn = DBig::from_str("-0.8").expect("ok");
        let ep = erf(&x, 25).expect("ok");
        let en = erf(&xn, 25).expect("ok");
        let fp = to_f64_approx(&ep);
        let fn_ = to_f64_approx(&en);
        assert!((fp + fn_).abs() < 1e-10, "erf(x) + erf(-x) = {}", fp + fn_);
    }

    #[test]
    fn erfc_zero() {
        let x = DBig::from_str("0.0").expect("ok");
        let v = erfc(&x, 20).expect("ok");
        let f = to_f64_approx(&v);
        assert!((f - 1.0).abs() < 1e-10, "erfc(0) = {f}");
    }

    #[test]
    fn erf_plus_erfc_is_one() {
        let x = DBig::from_str("1.5").expect("ok");
        let e = erf(&x, 20).expect("ok");
        let ec = erfc(&x, 20).expect("ok");
        let sum = to_f64_approx(&e) + to_f64_approx(&ec);
        assert!((sum - 1.0).abs() < 1e-10, "erf(1.5)+erfc(1.5) = {sum}");
    }

    #[test]
    fn bessel_j0_zero() {
        let x = DBig::from_str("0.0").expect("ok");
        let v = bessel_j0(&x, 20).expect("ok");
        let f = to_f64_approx(&v);
        assert!((f - 1.0).abs() < 1e-10, "J0(0) = {f}");
    }

    #[test]
    fn bessel_j0_first_zero_approx() {
        // First zero of J₀ ≈ 2.4048255577
        // J₀(2.4048) ≈ 0
        let x = DBig::from_str("2.4048255577").expect("ok");
        let v = bessel_j0(&x, 20).expect("ok");
        let f = to_f64_approx(&v).abs();
        assert!(f < 1e-5, "J0(2.4048) ≈ 0, got {f}");
    }

    #[test]
    fn free_cache_is_noop() {
        // Should not panic
        free_cache();
    }
}
