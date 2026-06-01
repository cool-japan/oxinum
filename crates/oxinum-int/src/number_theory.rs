//! Number theory functions: factorial, fibonacci, binomial, primality,
//! modular exponentiation, extended GCD, and Lucas sequences.

use crate::{IBig, UBig};

// ---------------------------------------------------------------------------
// Factorial
// ---------------------------------------------------------------------------

/// Computes `n!` (factorial of `n`).
///
/// Uses a simple iterative product. For very large `n`, a prime-swing
/// or divide-and-conquer approach would be faster, but this is correct
/// and practical for typical use.
///
/// # Examples
///
/// ```
/// use oxinum_int::factorial;
/// assert_eq!(factorial(0), dashu_int::UBig::ONE);
/// assert_eq!(factorial(5), dashu_int::UBig::from(120u32));
/// ```
pub fn factorial(n: u32) -> UBig {
    if n <= 1 {
        return UBig::ONE;
    }
    // Product tree: split the range and multiply halves for better balance.
    product_tree(2, n as u64)
}

/// Computes the product of integers in `[lo, hi]` using a recursive split
/// to keep operand sizes balanced (better cache + Karatsuba utilisation).
fn product_tree(lo: u64, hi: u64) -> UBig {
    if lo > hi {
        return UBig::ONE;
    }
    if lo == hi {
        return UBig::from(lo);
    }
    if hi - lo == 1 {
        return UBig::from(lo) * UBig::from(hi);
    }
    let mid = lo + (hi - lo) / 2;
    let left = product_tree(lo, mid);
    let right = product_tree(mid + 1, hi);
    left * right
}

// ---------------------------------------------------------------------------
// Fibonacci (fast doubling)
// ---------------------------------------------------------------------------

/// Computes the `n`-th Fibonacci number using the fast-doubling method.
///
/// The fast-doubling identities are:
///
/// ```text
/// F(2k)   = F(k) * (2 * F(k+1) - F(k))
/// F(2k+1) = F(k)^2 + F(k+1)^2
/// ```
///
/// This runs in O(log n) multiplications.
///
/// # Examples
///
/// ```
/// use oxinum_int::fibonacci;
/// assert_eq!(fibonacci(0), dashu_int::UBig::ZERO);
/// assert_eq!(fibonacci(1), dashu_int::UBig::ONE);
/// assert_eq!(fibonacci(10), dashu_int::UBig::from(55u32));
/// ```
pub fn fibonacci(n: u32) -> UBig {
    let (f, _) = fib_pair(n);
    f
}

/// Returns `(F(n), F(n+1))` via fast doubling.
fn fib_pair(n: u32) -> (UBig, UBig) {
    if n == 0 {
        return (UBig::ZERO, UBig::ONE);
    }
    let (a, b) = fib_pair(n / 2);
    // c = a * (2*b - a)
    let two_b = &b * UBig::from(2u32);
    let c = &a * (&two_b - &a);
    // d = a^2 + b^2
    let d = a.pow(2usize) + b.pow(2usize);
    if n % 2 == 0 {
        (c, d)
    } else {
        let next = &c + &d;
        (d, next)
    }
}

// ---------------------------------------------------------------------------
// Lucas sequences
// ---------------------------------------------------------------------------

/// Computes the Lucas number `L(n)` where `L(0) = 2, L(1) = 1`.
///
/// Uses the identity `L(n) = F(n-1) + F(n+1) = 2*F(n+1) - F(n)`
/// for `n >= 1`, falling back to direct computation.
///
/// # Examples
///
/// ```
/// use oxinum_int::lucas;
/// assert_eq!(lucas(0), dashu_int::UBig::from(2u32));
/// assert_eq!(lucas(1), dashu_int::UBig::ONE);
/// assert_eq!(lucas(5), dashu_int::UBig::from(11u32));
/// ```
pub fn lucas(n: u32) -> UBig {
    if n == 0 {
        return UBig::from(2u32);
    }
    // L(n) = 2*F(n+1) - F(n)
    let (fn_val, fn1_val) = fib_pair(n);
    let two_fn1 = &fn1_val * UBig::from(2u32);
    two_fn1 - fn_val
}

// ---------------------------------------------------------------------------
// Binomial coefficient
// ---------------------------------------------------------------------------

/// Computes the binomial coefficient `C(n, k) = n! / (k! * (n-k)!)`.
///
/// Uses multiplicative formula to avoid computing huge factorials:
///
/// ```text
/// C(n, k) = product(i=0..k) of (n - i) / (i + 1)
/// ```
///
/// # Examples
///
/// ```
/// use oxinum_int::binomial;
/// assert_eq!(binomial(10, 3), dashu_int::UBig::from(120u32));
/// assert_eq!(binomial(0, 0), dashu_int::UBig::ONE);
/// ```
pub fn binomial(n: u32, k: u32) -> UBig {
    if k > n {
        return UBig::ZERO;
    }
    // Symmetry: C(n, k) = C(n, n-k)
    let k = std::cmp::min(k, n - k);
    if k == 0 {
        return UBig::ONE;
    }

    let mut result = UBig::ONE;
    for i in 0..k {
        result *= UBig::from(n - i);
        result /= UBig::from(i + 1);
    }
    result
}

// ---------------------------------------------------------------------------
// Extended GCD
// ---------------------------------------------------------------------------

/// Computes the extended GCD of `a` and `b`, returning `(gcd, x, y)` such that
/// `a * x + b * y = gcd`.
///
/// Both `a` and `b` must be non-negative. The GCD is always non-negative.
///
/// # Examples
///
/// ```
/// use oxinum_int::extended_gcd;
/// use dashu_int::IBig;
/// let (g, x, y) = extended_gcd(&IBig::from(35), &IBig::from(15));
/// assert_eq!(g, IBig::from(5));
/// assert_eq!(&IBig::from(35) * &x + &IBig::from(15) * &y, g);
/// ```
pub fn extended_gcd(a: &IBig, b: &IBig) -> (IBig, IBig, IBig) {
    if *b == IBig::ZERO {
        let sign = if *a >= IBig::ZERO {
            IBig::ONE
        } else {
            IBig::from(-1)
        };
        return (a.clone() * &sign, sign, IBig::ZERO);
    }
    let (g, x1, y1) = extended_gcd(b, &(a % b));
    let q = a / b;
    let x = y1.clone();
    let y = x1 - &q * &y1;
    (g, x, y)
}

// ---------------------------------------------------------------------------
// Modular exponentiation
// ---------------------------------------------------------------------------

/// Computes `(base^exp) mod modulus` using binary (right-to-left) exponentiation.
///
/// All inputs must be non-negative and `modulus` must be > 0.
///
/// # Errors
///
/// Returns `OxiNumError::DivByZero` if `modulus` is zero.
///
/// # Examples
///
/// ```
/// use oxinum_int::mod_pow;
/// use dashu_int::UBig;
/// let result = mod_pow(
///     &UBig::from(2u32),
///     &UBig::from(10u32),
///     &UBig::from(1000u32),
/// ).unwrap();
/// assert_eq!(result, UBig::from(24u32));
/// ```
pub fn mod_pow(base: &UBig, exp: &UBig, modulus: &UBig) -> crate::OxiNumResult<UBig> {
    if *modulus == UBig::ZERO {
        return Err(crate::OxiNumError::DivByZero);
    }
    if *modulus == UBig::ONE {
        return Ok(UBig::ZERO);
    }

    let mut result = UBig::ONE;
    let mut base = base % modulus;
    let mut exp = exp.clone();

    while exp > UBig::ZERO {
        if &exp % UBig::from(2u32) == UBig::ONE {
            result = (&result * &base) % modulus;
        }
        exp /= UBig::from(2u32);
        base = (&base * &base) % modulus;
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Primality testing (Miller-Rabin)
// ---------------------------------------------------------------------------

/// Small primes for trial division.
const SMALL_PRIMES: [u64; 54] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193,
    197, 199, 211, 223, 227, 229, 233, 239, 241, 251,
];

/// Deterministic Miller-Rabin witnesses that are sufficient for numbers
/// below 3,317,044,064,679,887,385,961,981.
const DETERMINISTIC_WITNESSES: [u64; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

/// Tests whether `n` is (probably) prime using the Miller-Rabin test.
///
/// If `witnesses` is 0, uses a deterministic set of witnesses that is
/// correct for all numbers up to about 3.3 * 10^24, and likely correct
/// beyond that.
///
/// # Examples
///
/// ```
/// use oxinum_int::is_prime;
/// use dashu_int::UBig;
/// assert!(is_prime(&UBig::from(17u32), 0));
/// assert!(!is_prime(&UBig::from(15u32), 0));
/// assert!(!is_prime(&UBig::from(1u32), 0));
/// ```
pub fn is_prime(n: &UBig, witnesses: u32) -> bool {
    if *n < UBig::from(2u32) {
        return false;
    }
    // Trial division by small primes
    for &p in &SMALL_PRIMES {
        let p_big = UBig::from(p);
        if *n == p_big {
            return true;
        }
        if n % &p_big == UBig::ZERO {
            return false;
        }
    }

    // Write n - 1 = 2^r * d  where d is odd
    let n_minus_1 = n - UBig::ONE;
    let mut d = n_minus_1.clone();
    let mut r: u32 = 0;
    while &d % UBig::from(2u32) == UBig::ZERO {
        d /= UBig::from(2u32);
        r += 1;
    }

    // Choose witnesses
    let witness_list: Vec<UBig> = if witnesses == 0 {
        DETERMINISTIC_WITNESSES
            .iter()
            .filter(|&&w| UBig::from(w) < *n)
            .map(|&w| UBig::from(w))
            .collect()
    } else {
        // Use small primes as witnesses up to the requested count.
        SMALL_PRIMES
            .iter()
            .take(witnesses as usize)
            .filter(|&&w| UBig::from(w) < *n)
            .map(|&w| UBig::from(w))
            .collect()
    };

    for a in &witness_list {
        if !miller_rabin_witness(n, a, &d, r) {
            return false;
        }
    }
    true
}

/// Single Miller-Rabin witness test.
///
/// Returns `true` if `n` passes the test for witness `a`.
fn miller_rabin_witness(n: &UBig, a: &UBig, d: &UBig, r: u32) -> bool {
    let n_minus_1 = n - UBig::ONE;

    // x = a^d mod n
    let mut x = match mod_pow(a, d, n) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if x == UBig::ONE || x == n_minus_1 {
        return true;
    }

    for _ in 0..r.saturating_sub(1) {
        x = (&x * &x) % n;
        if x == n_minus_1 {
            return true;
        }
    }
    false
}

/// Returns the smallest prime greater than `n`.
///
/// # Examples
///
/// ```
/// use oxinum_int::next_prime;
/// use dashu_int::UBig;
/// assert_eq!(next_prime(&UBig::from(10u32)), UBig::from(11u32));
/// assert_eq!(next_prime(&UBig::from(11u32)), UBig::from(13u32));
/// ```
pub fn next_prime(n: &UBig) -> UBig {
    if *n < UBig::from(2u32) {
        return UBig::from(2u32);
    }
    // Start at n + 1 (or n + 2 if n + 1 is even)
    let mut candidate = n + UBig::ONE;
    if &candidate % UBig::from(2u32) == UBig::ZERO {
        candidate += UBig::ONE;
    }
    while !is_prime(&candidate, 0) {
        candidate += UBig::from(2u32);
    }
    candidate
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factorial_small() {
        assert_eq!(factorial(0), UBig::ONE);
        assert_eq!(factorial(1), UBig::ONE);
        assert_eq!(factorial(5), UBig::from(120u32));
        assert_eq!(factorial(10), UBig::from(3_628_800u32));
    }

    #[test]
    fn factorial_20() {
        let f20 = factorial(20);
        assert_eq!(f20.to_string(), "2432902008176640000");
    }

    #[test]
    fn factorial_100_digit_count() {
        let f100 = factorial(100);
        let s = f100.to_string();
        // 100! has 158 decimal digits
        assert_eq!(s.len(), 158, "100! has {len} digits", len = s.len());
    }

    #[test]
    fn fibonacci_small() {
        assert_eq!(fibonacci(0), UBig::ZERO);
        assert_eq!(fibonacci(1), UBig::ONE);
        assert_eq!(fibonacci(2), UBig::ONE);
        assert_eq!(fibonacci(10), UBig::from(55u32));
        assert_eq!(fibonacci(20), UBig::from(6765u32));
    }

    #[test]
    fn fibonacci_large() {
        let f50 = fibonacci(50);
        assert_eq!(f50.to_string(), "12586269025");
    }

    #[test]
    fn lucas_small() {
        assert_eq!(lucas(0), UBig::from(2u32));
        assert_eq!(lucas(1), UBig::ONE);
        assert_eq!(lucas(2), UBig::from(3u32));
        assert_eq!(lucas(5), UBig::from(11u32));
        assert_eq!(lucas(10), UBig::from(123u32));
    }

    #[test]
    fn binomial_basic() {
        assert_eq!(binomial(0, 0), UBig::ONE);
        assert_eq!(binomial(5, 0), UBig::ONE);
        assert_eq!(binomial(5, 5), UBig::ONE);
        assert_eq!(binomial(5, 2), UBig::from(10u32));
        assert_eq!(binomial(10, 3), UBig::from(120u32));
        assert_eq!(binomial(20, 10), UBig::from(184_756u32));
    }

    #[test]
    fn binomial_symmetry() {
        for n in 0..15 {
            for k in 0..=n {
                assert_eq!(binomial(n, k), binomial(n, n - k));
            }
        }
    }

    #[test]
    fn binomial_k_gt_n_is_zero() {
        assert_eq!(binomial(3, 5), UBig::ZERO);
    }

    #[test]
    fn extended_gcd_basic() {
        let a = IBig::from(35);
        let b = IBig::from(15);
        let (g, x, y) = extended_gcd(&a, &b);
        assert_eq!(g, IBig::from(5));
        // Verify Bezout identity: a*x + b*y = gcd
        assert_eq!(&a * &x + &b * &y, g);
    }

    #[test]
    fn extended_gcd_coprime() {
        let a = IBig::from(17);
        let b = IBig::from(13);
        let (g, x, y) = extended_gcd(&a, &b);
        assert_eq!(g, IBig::ONE);
        assert_eq!(&a * &x + &b * &y, IBig::ONE);
    }

    #[test]
    fn extended_gcd_one_zero() {
        let a = IBig::from(42);
        let b = IBig::ZERO;
        let (g, x, y) = extended_gcd(&a, &b);
        assert_eq!(g, IBig::from(42));
        assert_eq!(&a * &x + &b * &y, g);
    }

    #[test]
    fn mod_pow_basic() {
        // 2^10 mod 1000 = 1024 mod 1000 = 24
        let result =
            mod_pow(&UBig::from(2u32), &UBig::from(10u32), &UBig::from(1000u32)).expect("ok");
        assert_eq!(result, UBig::from(24u32));
    }

    #[test]
    fn mod_pow_large() {
        // 3^100 mod 97 -- Fermat's little theorem: 3^96 ≡ 1 mod 97
        // 3^100 = 3^96 * 3^4 ≡ 81 mod 97
        let result =
            mod_pow(&UBig::from(3u32), &UBig::from(100u32), &UBig::from(97u32)).expect("ok");
        assert_eq!(result, UBig::from(81u32));
    }

    #[test]
    fn mod_pow_zero_exponent() {
        let result = mod_pow(&UBig::from(5u32), &UBig::ZERO, &UBig::from(13u32)).expect("ok");
        assert_eq!(result, UBig::ONE);
    }

    #[test]
    fn mod_pow_modulus_one() {
        let result = mod_pow(&UBig::from(5u32), &UBig::from(10u32), &UBig::ONE).expect("ok");
        assert_eq!(result, UBig::ZERO);
    }

    #[test]
    fn mod_pow_div_by_zero() {
        let result = mod_pow(&UBig::from(2u32), &UBig::from(3u32), &UBig::ZERO);
        assert!(result.is_err());
    }

    #[test]
    fn is_prime_small() {
        let primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
        for &p in &primes {
            assert!(is_prime(&UBig::from(p as u32), 0), "{p} should be prime");
        }
    }

    #[test]
    fn is_prime_composites() {
        let composites = [0, 1, 4, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20, 21, 25];
        for &c in &composites {
            assert!(
                !is_prime(&UBig::from(c as u32), 0),
                "{c} should not be prime"
            );
        }
    }

    #[test]
    fn is_prime_mersenne() {
        // M_17 = 131071 is prime, M_23 = 8388607 is not
        assert!(is_prime(&UBig::from(131071u32), 0));
        assert!(!is_prime(&UBig::from(8_388_607u32), 0));
    }

    #[test]
    fn is_prime_carmichael_numbers() {
        // Carmichael numbers are composite but fool Fermat test.
        // They should NOT fool Miller-Rabin with enough witnesses.
        let carmichael = [561u64, 1105, 1729, 2465, 2821, 6601, 8911];
        for &c in &carmichael {
            assert!(
                !is_prime(&UBig::from(c), 0),
                "Carmichael number {c} should NOT be prime"
            );
        }
    }

    #[test]
    fn next_prime_basic() {
        assert_eq!(next_prime(&UBig::ZERO), UBig::from(2u32));
        assert_eq!(next_prime(&UBig::ONE), UBig::from(2u32));
        assert_eq!(next_prime(&UBig::from(2u32)), UBig::from(3u32));
        assert_eq!(next_prime(&UBig::from(10u32)), UBig::from(11u32));
        assert_eq!(next_prime(&UBig::from(11u32)), UBig::from(13u32));
        assert_eq!(next_prime(&UBig::from(100u32)), UBig::from(101u32));
    }

    #[test]
    fn division_roundtrip() {
        // (a / b) * b + (a % b) == a
        let a = UBig::from(12345u32);
        let b = UBig::from(67u32);
        let q = &a / &b;
        let r = &a % &b;
        assert_eq!(q * b + r, a);
    }
}
