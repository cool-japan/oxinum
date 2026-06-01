//! Sieve of Eratosthenes for generating small primes.
//!
//! Uses a bit-packed sieve to enumerate all primes up to a given limit.
//! Memory usage: approximately `limit / 16` bytes (only odd numbers sieved).

/// Returns all primes ≤ `limit` in ascending order.
///
/// Uses a bit-packed sieve that tracks only odd numbers, halving memory
/// relative to a naive boolean array. Index `i` in the bit array represents
/// the odd number `2*i + 1`.
///
/// # Examples
///
/// ```
/// use oxinum_int::native::prime_sieve;
/// let primes = prime_sieve(10);
/// assert_eq!(primes, vec![2, 3, 5, 7]);
/// ```
pub fn prime_sieve(limit: u64) -> Vec<u64> {
    if limit < 2 {
        return vec![];
    }

    // Bit-packed sieve over odd numbers only.
    // Index i represents the odd number 2*i + 1.
    // sieve[i/64] bit (i%64) is 1 if (2i+1) is composite.
    // Odd numbers in [1, limit]: 1, 3, 5, ..., so count = ceil(limit / 2).
    let odd_count = limit.div_ceil(2) as usize; // = number of odd values in [1, limit]
    let word_count = odd_count.div_ceil(64);
    let mut sieve = vec![0u64; word_count];

    // Mark 1 as composite (index 0 represents the number 1).
    sieve[0] |= 1;

    // Outer loop: p runs over odd primes.
    // We test p up to sqrt(limit). For each unmarked p, mark multiples.
    let mut p = 3u64;
    while p.saturating_mul(p) <= limit {
        let pi = ((p - 1) / 2) as usize; // bit index for p
                                         // Check if p is still marked prime (bit = 0 means prime).
        if sieve[pi / 64] & (1 << (pi % 64)) == 0 {
            // p is prime; mark all odd multiples starting from p^2.
            // Multiples of p that are odd: p^2, p^2 + 2p, p^2 + 4p, ...
            let mut multiple = p * p;
            while multiple <= limit {
                let mi = ((multiple - 1) / 2) as usize;
                sieve[mi / 64] |= 1 << (mi % 64);
                multiple = match multiple.checked_add(2 * p) {
                    Some(v) => v,
                    None => break,
                };
            }
        }
        p += 2;
    }

    // Collect results: 2 plus all unmarked odd indices.
    let mut primes = vec![2u64];
    for i in 1..odd_count {
        if sieve[i / 64] & (1 << (i % 64)) == 0 {
            primes.push(2 * i as u64 + 1);
        }
    }
    primes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sieve_empty_for_small_limits() {
        assert!(prime_sieve(0).is_empty());
        assert!(prime_sieve(1).is_empty());
    }

    #[test]
    fn sieve_two_is_first_prime() {
        assert_eq!(prime_sieve(2), vec![2]);
    }

    #[test]
    fn sieve_up_to_10() {
        assert_eq!(prime_sieve(10), vec![2, 3, 5, 7]);
    }

    #[test]
    fn sieve_up_to_100() {
        let expected = vec![
            2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79,
            83, 89, 97,
        ];
        assert_eq!(prime_sieve(100), expected);
    }

    #[test]
    fn sieve_count_1000() {
        assert_eq!(prime_sieve(1000).len(), 168);
    }

    #[test]
    fn sieve_count_10000() {
        assert_eq!(prime_sieve(10_000).len(), 1229);
    }
}
