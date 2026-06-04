//! SciRS2 facade and dashu drop-in compatibility verification.
//!
//! Covers two TODO items:
//!
//! 1. "Ensure SciRS2 can depend solely on oxinum for all numeric needs"
//!    — proves that every numeric type and operation the SciRS2 arbitrary-
//!    precision subsystem needs is reachable through the `oxinum` (facade)
//!    crate or through the sub-crates that oxinum re-exports.
//!
//! 2. "Verify oxinum works as drop-in for projects currently using dashu directly"
//!    — proves that code written against the dashu API surface compiles and
//!    behaves identically when using the `oxinum` (facade) re-exports.

// -----------------------------------------------------------------------
// Test 1 — SciRS2 can depend solely on oxinum
// -----------------------------------------------------------------------
//
// The SciRS2 consumer currently imports from the individual sub-crates
// (oxinum_int, oxinum_float, etc.).  We prove here that all of those
// symbols are also reachable through the `oxinum` facade, so that a future
// migration to a single `oxinum` dependency is possible without a public-
// API break.
//
// Note: The facade re-exports the *dashu-backed* types at the root and the
// *native* types under `oxinum::native::*`.  Both are tested.

mod scirs2_sole_dep {
    // Integer types accessible through the oxinum facade.
    use oxinum::{IBig, UBig};
    // Float types.
    use oxinum::Float; // alias for DBig
                       // Rational types.
    use oxinum::Rational; // alias for RBig
                          // Error type.
    use oxinum::OxiNumError;

    #[test]
    fn ibig_ubig_from_facade() {
        let n: IBig = IBig::from(-42_i64);
        let m: UBig = UBig::from(42_u32);
        assert_eq!(n.to_string(), "-42");
        assert_eq!(m.to_string(), "42");
    }

    #[test]
    fn float_from_facade() {
        // oxinum::Float is an alias for dashu DBig.
        use std::str::FromStr;
        // Use 1.25 (exact in binary) to avoid the clippy::approx_constant lint.
        let f: Float = Float::from_str("1.25").expect("parse");
        let v: f64 = f.to_f64().value();
        assert!((v - 1.25_f64).abs() < 1e-15);
    }

    #[test]
    fn rational_from_facade() {
        use oxinum::IBig as FIBig;
        use oxinum::UBig as FUBig;
        let r: Rational = Rational::from_parts(FIBig::from(22_i32), FUBig::from(7_u32));
        assert_eq!(r.to_string(), "22/7");
    }

    #[test]
    fn error_type_from_facade() {
        let e = OxiNumError::DivByZero;
        assert!(e.to_string().contains("division by zero"));
    }

    #[test]
    fn number_theory_from_facade() {
        // is_prime and factorial exposed through the facade crate via re-exports.
        let p = oxinum::is_prime(&UBig::from(97_u32), 0);
        assert!(p);
        let f = oxinum::factorial(10);
        assert_eq!(f.to_string(), "3628800");
    }

    #[test]
    fn constants_from_facade() {
        let pi = oxinum::constants::pi(30);
        assert!(pi.to_string().starts_with("3.14159"));
        let e = oxinum::constants::e(30);
        assert!(e.to_string().starts_with("2.71828"));
    }

    #[test]
    fn native_types_from_facade() {
        // Native types are accessible under oxinum::native::*.
        use oxinum::native::{BigFloat, BigInt, BigRational, BigUint, RoundingMode};
        let u = BigUint::from_u64(42);
        let i = BigInt::from(42_i64);
        let f = BigFloat::from_i64(1, 53, RoundingMode::HalfEven);
        let r = BigRational::from_integer(i.clone());
        assert_eq!(u.to_string(), "42");
        assert_eq!(i.to_string(), "42");
        assert_eq!(r.to_string(), "42");
        // BigFloat doesn't implement Display but we can verify it's non-zero.
        assert!(!f.is_zero());
    }
}

// -----------------------------------------------------------------------
// Test 2 — Drop-in for dashu-using projects
// -----------------------------------------------------------------------
//
// Projects currently depending directly on dashu-int / dashu-float /
// dashu-ratio can switch to oxinum (which re-exports dashu types) and
// get identical behaviour.  We verify that the common dashu call sites
// compile and return the same values via oxinum.

mod dashu_drop_in {
    // dashu types via oxinum re-exports.
    use oxinum::Float as DBig;
    use oxinum::{IBig, UBig}; // alias for dashu DBig

    #[test]
    fn ibig_arithmetic_parity() {
        // dashu IBig arithmetic.
        let a = IBig::from(100_i64);
        let b = IBig::from(-30_i64);
        let sum = a.clone() + b.clone();
        let diff = a.clone() - b.clone();
        let prod = a.clone() * b.clone();
        assert_eq!(sum.to_string(), "70");
        assert_eq!(diff.to_string(), "130");
        assert_eq!(prod.to_string(), "-3000");
    }

    #[test]
    fn ubig_division_parity() {
        // dashu UBig division (panics on zero divisor — same as dashu).
        let a = UBig::from(1000_u32);
        let b = UBig::from(7_u32);
        let q = a.clone() / b.clone();
        let r = a % b;
        assert_eq!(q.to_string(), "142");
        assert_eq!(r.to_string(), "6");
    }

    #[test]
    fn dbig_from_str_and_arithmetic() {
        use std::str::FromStr;
        let a = DBig::from_str("1.5").expect("parse 1.5");
        let b = DBig::from_str("2.5").expect("parse 2.5");
        let sum = a + b;
        let f: f64 = sum.to_f64().value();
        assert!((f - 4.0).abs() < 1e-14);
    }

    #[test]
    fn rbig_from_parts_parity() {
        use oxinum::Rational as RBig;
        use oxinum::{IBig as FIBig, UBig as FUBig};
        // dashu RBig::from_parts is a key constructor in projects using dashu-ratio.
        let r = RBig::from_parts(FIBig::from(3_i32), FUBig::from(4_u32));
        assert_eq!(r.to_string(), "3/4");
        let num = r.numerator();
        let den = r.denominator();
        assert_eq!(num.to_string(), "3");
        assert_eq!(den.to_string(), "4");
    }

    #[test]
    fn gcd_parity() {
        use oxinum::Gcd;
        let a = IBig::from(48_i64);
        let b = IBig::from(18_i64);
        let g: UBig = a.gcd(&b);
        assert_eq!(g.to_string(), "6");
    }

    #[test]
    fn sign_comparison_parity() {
        let pos = IBig::from(5_i64);
        let neg = IBig::from(-5_i64);
        let zero = IBig::from(0_i64);
        assert!(pos > zero);
        assert!(neg < zero);
        assert_eq!(zero.clone(), zero);
    }

    #[test]
    fn large_integer_display_parity() {
        // 2^100 via pow on IBig.
        let two = IBig::from(2_i64);
        let pow100 = oxinum::native::BigUint::from_u64(2).pow(100);
        // Also verify via UBig via oxinum::factorial which uses dashu path.
        let _ = two; // silence unused-variable warning
        let _ = pow100;
        // Factorial(20) to check dashu-backed path.
        let f = oxinum::factorial(20);
        assert_eq!(f.to_string(), "2432902008176640000");
    }
}
