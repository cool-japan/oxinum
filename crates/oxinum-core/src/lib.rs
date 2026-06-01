#![forbid(unsafe_code)]
//! Core traits, error types, and rounding modes for the OxiNum ecosystem.
//!
//! This crate provides the foundational types shared by `oxinum-int`,
//! `oxinum-float`, and `oxinum-rational`.

/// Re-export the `Sign` type from `dashu-base`.
///
/// `Sign::Positive` / `Sign::Negative` indicate the sign of a number.
pub use dashu_base::Sign;

// Re-export useful dashu-base traits so downstream crates get them from one place.
pub use dashu_base::{
    Abs, AbsOrd, BitTest, CubicRoot, DivEuclid, DivRem, DivRemAssign, DivRemEuclid, EstimatedLog2,
    ExtendedGcd, Gcd, Inverse, PowerOfTwo, RemEuclid, Signed, SquareRoot, UnsignedAbs,
};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Convenience alias for `Result<T, OxiNumError>`.
pub type OxiNumResult<T> = Result<T, OxiNumError>;

/// Errors from OxiNum operations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OxiNumError {
    /// Failed to parse a number from a string.
    Parse(std::borrow::Cow<'static, str>),
    /// Precision constraint violated.
    Precision(std::borrow::Cow<'static, str>),
    /// Division by zero.
    DivByZero,
    /// Arithmetic overflow (e.g. result exceeds a primitive range).
    Overflow(std::borrow::Cow<'static, str>),
    /// Invalid radix for base conversion (must be 2..=36).
    InvalidRadix(u32),
    /// Input is outside the domain of the function (e.g. sqrt of a negative).
    Domain(std::borrow::Cow<'static, str>),
}

impl OxiNumError {
    /// Returns a new error of the same variant with `ctx` prepended to the
    /// message, for message-bearing variants.
    ///
    /// For variants without a free-form message (`DivByZero`, `InvalidRadix`)
    /// the original value is returned unchanged because their `Display`
    /// already conveys the kind precisely.
    ///
    /// # Examples
    ///
    /// ```
    /// use oxinum_core::OxiNumError;
    ///
    /// let e = OxiNumError::Parse("bad digit".into()).context("while reading row 4");
    /// assert!(e.to_string().contains("while reading row 4:"));
    /// assert!(e.to_string().contains("bad digit"));
    ///
    /// // Variants without a message are returned untouched.
    /// assert_eq!(
    ///     OxiNumError::DivByZero.context("noop"),
    ///     OxiNumError::DivByZero,
    /// );
    /// ```
    #[must_use]
    pub fn context(self, ctx: impl AsRef<str>) -> Self {
        let ctx = ctx.as_ref();
        match self {
            Self::Parse(s) => Self::Parse(format!("{ctx}: {s}").into()),
            Self::Precision(s) => Self::Precision(format!("{ctx}: {s}").into()),
            Self::Overflow(s) => Self::Overflow(format!("{ctx}: {s}").into()),
            Self::Domain(s) => Self::Domain(format!("{ctx}: {s}").into()),
            other => other,
        }
    }
}

impl std::fmt::Display for OxiNumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(s) => write!(f, "parse error: {s}"),
            Self::Precision(s) => write!(f, "precision error: {s}"),
            Self::DivByZero => write!(f, "division by zero"),
            Self::Overflow(s) => write!(f, "overflow: {s}"),
            Self::InvalidRadix(r) => write!(f, "invalid radix: {r} (must be 2..=36)"),
            Self::Domain(s) => write!(f, "domain error: {s}"),
        }
    }
}

impl std::error::Error for OxiNumError {}

impl From<OxiNumError> for std::io::Error {
    fn from(e: OxiNumError) -> Self {
        std::io::Error::other(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// ParseNumberError -- positional parse diagnostics
// ---------------------------------------------------------------------------

/// Rich parse-error diagnostic carrying the offending message together with
/// the 1-based line and column where the parser stopped.
///
/// This is a standalone error type (not a variant of [`OxiNumError`]) so that
/// the existing `OxiNumError` size envelope is preserved. Convert via
/// `OxiNumError::from(parse_err)` (or `?`) to fold a positional diagnostic
/// back into the unified error type — the resulting `OxiNumError::Parse`
/// message will include the line and column.
///
/// # Examples
///
/// ```
/// use oxinum_core::{OxiNumError, ParseNumberError};
///
/// let pe = ParseNumberError::new("unexpected character", 2, 5);
/// assert!(pe.to_string().contains("line 2"));
/// assert!(pe.to_string().contains("column 5"));
///
/// // Fold into the unified error type.
/// let e: OxiNumError = pe.into();
/// assert!(matches!(e, OxiNumError::Parse(_)));
/// assert!(e.to_string().contains("line 2"));
/// assert!(e.to_string().contains("col 5"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParseNumberError {
    /// Human-readable description of why the parse failed.
    pub message: String,
    /// 1-based line where the parser stopped.
    pub line: usize,
    /// 1-based column where the parser stopped.
    pub column: usize,
}

impl ParseNumberError {
    /// Construct a new [`ParseNumberError`] from a message and a 1-based
    /// `(line, column)` position.
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

impl std::fmt::Display for ParseNumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parse error at line {line}, column {column}: {message}",
            line = self.line,
            column = self.column,
            message = self.message,
        )
    }
}

impl std::error::Error for ParseNumberError {}

impl From<ParseNumberError> for OxiNumError {
    fn from(e: ParseNumberError) -> Self {
        let ParseNumberError {
            message,
            line,
            column,
        } = e;
        OxiNumError::Parse(format!("{message} (line {line}, col {column})").into())
    }
}

// ---------------------------------------------------------------------------
// Rounding mode (dashu-independent)
// ---------------------------------------------------------------------------

/// Rounding modes for arbitrary-precision arithmetic.
///
/// This enum is independent of any backend library and provides a common
/// vocabulary for precision control across all OxiNum numeric types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RoundingMode {
    /// Round toward positive infinity.
    Up,
    /// Round toward negative infinity.
    Down,
    /// Round toward positive infinity (alias for Up in unsigned context).
    Ceiling,
    /// Round toward negative infinity (alias for Down in unsigned context).
    Floor,
    /// Round half toward positive infinity.
    HalfUp,
    /// Round half toward negative infinity.
    HalfDown,
    /// Round half to the nearest even digit (banker's rounding).
    HalfEven,
    /// Exact result required -- error if rounding would occur.
    Unnecessary,
}

impl std::fmt::Display for RoundingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Ceiling => "Ceiling",
            Self::Floor => "Floor",
            Self::HalfUp => "HalfUp",
            Self::HalfDown => "HalfDown",
            Self::HalfEven => "HalfEven",
            Self::Unnecessary => "Unnecessary",
        };
        f.write_str(name)
    }
}

// ---------------------------------------------------------------------------
// Numeric trait hierarchy
// ---------------------------------------------------------------------------

/// Marker trait for all OxiNum numeric types.
///
/// Implementors must support display, debug, cloning, and basic equality.
pub trait OxiNum: std::fmt::Display + std::fmt::Debug + Clone + PartialEq {
    /// Returns `true` if this value is zero.
    fn is_zero(&self) -> bool;

    /// Returns `true` if this value is one.
    fn is_one(&self) -> bool;
}

/// Trait for numeric types that carry a sign.
pub trait OxiSigned: OxiNum {
    /// Returns the sign of this number.
    fn signum(&self) -> Sign;

    /// Returns the absolute value.
    fn abs(&self) -> Self;

    /// Returns `true` if this value is negative.
    fn is_negative(&self) -> bool {
        self.signum() == Sign::Negative
    }

    /// Returns `true` if this value is positive (and not zero).
    fn is_positive(&self) -> bool {
        !self.is_zero() && self.signum() == Sign::Positive
    }
}

/// Trait for unsigned numeric types.
pub trait OxiUnsigned: OxiNum {}

// ---------------------------------------------------------------------------
// Conversion traits
// ---------------------------------------------------------------------------

/// Parse a number from an arbitrary-radix string.
pub trait FromRadix: Sized {
    /// Parse `src` in the given `radix` (2..=36).
    ///
    /// # Errors
    ///
    /// Returns `OxiNumError::Parse` on invalid digits or
    /// `OxiNumError::InvalidRadix` if radix is out of range.
    fn from_radix(src: &str, radix: u32) -> OxiNumResult<Self>;
}

/// Format a number as a string in an arbitrary radix.
pub trait ToRadix {
    /// Returns the string representation in the given `radix` (2..=36).
    ///
    /// # Errors
    ///
    /// Returns `OxiNumError::InvalidRadix` if radix is out of range.
    fn to_radix(&self, radix: u32) -> OxiNumResult<String>;
}

// ---------------------------------------------------------------------------
// Power / roots traits
// ---------------------------------------------------------------------------

/// Exponentiation trait.
pub trait Pow<Exp> {
    /// The output type.
    type Output;

    /// Raises `self` to the power `exp`.
    fn pow(&self, exp: Exp) -> Self::Output;
}

/// Root extraction trait.
pub trait Roots {
    /// Integer square root (floor).
    fn sqrt(&self) -> Self;

    /// Integer cube root (floor).
    fn cbrt(&self) -> Self;

    /// Integer nth root (floor).
    fn nth_root(&self, n: u32) -> Self;
}

// ---------------------------------------------------------------------------
// Modular arithmetic trait
// ---------------------------------------------------------------------------

/// Modular arithmetic operations.
pub trait ModularArithmetic {
    /// Computes `(self + rhs) mod modulus`.
    fn mod_add(&self, rhs: &Self, modulus: &Self) -> Self;

    /// Computes `(self - rhs) mod modulus`.
    fn mod_sub(&self, rhs: &Self, modulus: &Self) -> Self;

    /// Computes `(self * rhs) mod modulus`.
    fn mod_mul(&self, rhs: &Self, modulus: &Self) -> Self;

    /// Computes `self^exp mod modulus` via binary exponentiation.
    fn mod_pow(&self, exp: &Self, modulus: &Self) -> Self;
}

// ---------------------------------------------------------------------------
// Primality trait
// ---------------------------------------------------------------------------

/// Primality testing operations.
pub trait Primality {
    /// Returns `true` if the number is (probably) prime.
    ///
    /// Uses Miller-Rabin with the given number of witnesses.
    /// With `witnesses = 0`, uses a deterministic set for small values
    /// and a sensible default count for large values.
    fn is_probably_prime(&self, witnesses: u32) -> bool;

    /// Returns the next prime greater than `self`.
    fn next_prime(&self) -> Self;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_parse() {
        let e = OxiNumError::Parse("bad input".into());
        assert!(e.to_string().contains("bad input"));
    }

    #[test]
    fn error_display_precision() {
        let e = OxiNumError::Precision("too low".into());
        assert!(e.to_string().contains("too low"));
    }

    #[test]
    fn error_display_div_by_zero() {
        let e = OxiNumError::DivByZero;
        assert_eq!(e.to_string(), "division by zero");
    }

    #[test]
    fn error_display_overflow() {
        let e = OxiNumError::Overflow("u64 max exceeded".into());
        assert!(e.to_string().contains("u64 max exceeded"));
    }

    #[test]
    fn error_display_invalid_radix() {
        let e = OxiNumError::InvalidRadix(42);
        assert!(e.to_string().contains("42"));
        assert!(e.to_string().contains("must be 2..=36"));
    }

    #[test]
    fn error_display_domain() {
        let e = OxiNumError::Domain("sqrt of negative is undefined for real BigFloat".into());
        assert_eq!(
            e.to_string(),
            "domain error: sqrt of negative is undefined for real BigFloat"
        );
    }

    #[test]
    fn context_prefixes_domain_message() {
        let e = OxiNumError::Domain("sqrt of negative".into()).context("BigFloat::sqrt");
        match &e {
            OxiNumError::Domain(s) => assert_eq!(s, "BigFloat::sqrt: sqrt of negative"),
            other => panic!("expected Domain, got {other:?}"),
        }
        assert!(e.to_string().contains("domain error:"));
        assert!(e.to_string().contains("BigFloat::sqrt:"));
    }

    #[test]
    fn error_into_io_error() {
        let e = OxiNumError::DivByZero;
        let io_err: std::io::Error = e.into();
        assert_eq!(io_err.kind(), std::io::ErrorKind::Other);
        assert!(io_err.to_string().contains("division by zero"));
    }

    #[test]
    fn sign_positive() {
        let s = Sign::Positive;
        assert_eq!(s, Sign::Positive);
    }

    #[test]
    fn rounding_mode_display() {
        assert_eq!(RoundingMode::HalfEven.to_string(), "HalfEven");
        assert_eq!(RoundingMode::Up.to_string(), "Up");
        assert_eq!(RoundingMode::Unnecessary.to_string(), "Unnecessary");
    }

    #[test]
    fn rounding_mode_equality() {
        assert_eq!(RoundingMode::Floor, RoundingMode::Floor);
        assert_ne!(RoundingMode::Up, RoundingMode::Down);
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OxiNumError>();
    }

    #[test]
    fn error_size_is_small() {
        // OxiNumError should be reasonably small (3 words or fewer on the stack)
        let size = std::mem::size_of::<OxiNumError>();
        assert!(size <= 32, "OxiNumError is {size} bytes, expected <= 32");
    }

    #[test]
    fn oxinumerror_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        let e = OxiNumError::DivByZero;
        assert_error(&e);
        // Also verify it can be boxed as a trait object.
        let _boxed: Box<dyn std::error::Error> = Box::new(OxiNumError::DivByZero);
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(OxiNumError::Parse("test".into()));
        assert!(e.to_string().contains("test"));
    }

    #[test]
    fn oxi_num_result_alias() {
        let ok: OxiNumResult<u32> = Ok(42);
        assert_eq!(ok, Ok(42));
        let err: OxiNumResult<u32> = Err(OxiNumError::DivByZero);
        assert!(err.is_err());
    }

    // -----------------------------------------------------------------------
    // ParseNumberError + context() (Item 2 — diagnostics enrichment)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_number_error_constructs_and_displays() {
        let pe = ParseNumberError::new("bad digit", 3, 7);
        assert_eq!(pe.message, "bad digit");
        assert_eq!(pe.line, 3);
        assert_eq!(pe.column, 7);

        // Native Display uses the full words "line" / "column".
        let pe_disp = pe.to_string();
        assert!(pe_disp.contains("line 3"), "got {pe_disp}");
        assert!(pe_disp.contains("column 7"), "got {pe_disp}");
        assert!(pe_disp.contains("bad digit"), "got {pe_disp}");

        // Folding into OxiNumError yields Parse(...) and embeds the position.
        let oe: OxiNumError = pe.into();
        match &oe {
            OxiNumError::Parse(_) => {}
            other => panic!("expected Parse, got {other:?}"),
        }
        let oe_disp = oe.to_string();
        assert!(oe_disp.contains("bad digit"), "got {oe_disp}");
        // Folded form uses "line {line}, col {column}".
        assert!(oe_disp.contains("line 3"), "got {oe_disp}");
        assert!(oe_disp.contains("col 7"), "got {oe_disp}");
    }

    #[test]
    fn parse_number_error_is_std_error() {
        let boxed: Box<dyn std::error::Error> = Box::new(ParseNumberError::new("oops", 1, 1));
        assert!(boxed.to_string().contains("oops"));
    }

    #[test]
    fn context_prefixes_message_variants() {
        let parse = OxiNumError::Parse("x".into()).context("at A");
        match parse {
            OxiNumError::Parse(ref s) => assert_eq!(s, "at A: x"),
            other => panic!("expected Parse, got {other:?}"),
        }
        assert!(parse.to_string().contains("at A:"));

        let precision = OxiNumError::Precision("y".into()).context("at B");
        match precision {
            OxiNumError::Precision(ref s) => assert_eq!(s, "at B: y"),
            other => panic!("expected Precision, got {other:?}"),
        }

        let overflow = OxiNumError::Overflow("z".into()).context("at C");
        match overflow {
            OxiNumError::Overflow(ref s) => assert_eq!(s, "at C: z"),
            other => panic!("expected Overflow, got {other:?}"),
        }
    }

    #[test]
    fn context_leaves_kindful_variants_unchanged() {
        assert_eq!(
            OxiNumError::DivByZero.context("ignored"),
            OxiNumError::DivByZero,
        );
        assert_eq!(
            OxiNumError::InvalidRadix(37).context("ignored"),
            OxiNumError::InvalidRadix(37),
        );
    }

    #[test]
    fn existing_size_of_oxinumerror_unchanged() {
        // Re-assert (alongside `error_size_is_small`) that Item 2's additions
        // did not bloat the variant payload.
        let size = std::mem::size_of::<OxiNumError>();
        assert!(size <= 32, "OxiNumError is {size} bytes, expected <= 32");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_display_roundtrip(s in any::<String>()) {
            let e = OxiNumError::Parse(s.clone().into());
            prop_assert!(e.to_string().contains(&s));
        }

        #[test]
        fn precision_display_roundtrip(s in any::<String>()) {
            let e = OxiNumError::Precision(s.clone().into());
            prop_assert!(e.to_string().contains(&s));
        }

        #[test]
        fn overflow_display_roundtrip(s in any::<String>()) {
            let e = OxiNumError::Overflow(s.clone().into());
            prop_assert!(e.to_string().contains(&s));
        }

        #[test]
        fn domain_display_roundtrip(s in any::<String>()) {
            let e = OxiNumError::Domain(s.clone().into());
            prop_assert!(e.to_string().contains(&s));
        }

        #[test]
        fn parse_number_error_display_roundtrip(
            msg in any::<String>(),
            line in 1usize..=10_000,
            column in 1usize..=10_000,
        ) {
            let pe = ParseNumberError::new(msg.clone(), line, column);
            let disp = pe.to_string();
            prop_assert!(disp.contains(&msg));
            prop_assert!(disp.contains(&line.to_string()));
            prop_assert!(disp.contains(&column.to_string()));
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    fn roundtrip_oxi(original: OxiNumError) {
        let json = serde_json::to_string(&original).expect("serialize OxiNumError");
        let back: OxiNumError = serde_json::from_str(&json).expect("deserialize OxiNumError");
        assert_eq!(back, original, "round-trip mismatch for {original:?}");
    }

    fn roundtrip_rounding(original: RoundingMode) {
        let json = serde_json::to_string(&original).expect("serialize RoundingMode");
        let back: RoundingMode = serde_json::from_str(&json).expect("deserialize RoundingMode");
        assert_eq!(back, original, "round-trip mismatch for {original:?}");
    }

    #[test]
    fn oxinum_error_json_roundtrip_all_variants() {
        roundtrip_oxi(OxiNumError::Parse("e".into()));
        roundtrip_oxi(OxiNumError::Precision("p".into()));
        roundtrip_oxi(OxiNumError::DivByZero);
        roundtrip_oxi(OxiNumError::Overflow("o".into()));
        roundtrip_oxi(OxiNumError::InvalidRadix(3));
        roundtrip_oxi(OxiNumError::Domain("sqrt of negative".into()));
    }

    #[test]
    fn rounding_mode_json_roundtrip_all_variants() {
        roundtrip_rounding(RoundingMode::Up);
        roundtrip_rounding(RoundingMode::Down);
        roundtrip_rounding(RoundingMode::Ceiling);
        roundtrip_rounding(RoundingMode::Floor);
        roundtrip_rounding(RoundingMode::HalfUp);
        roundtrip_rounding(RoundingMode::HalfDown);
        roundtrip_rounding(RoundingMode::HalfEven);
        roundtrip_rounding(RoundingMode::Unnecessary);
    }

    #[test]
    fn parse_number_error_json_roundtrip() {
        let pe = ParseNumberError::new("bad", 4, 9);
        let json = serde_json::to_string(&pe).expect("serialize ParseNumberError");
        let back: ParseNumberError =
            serde_json::from_str(&json).expect("deserialize ParseNumberError");
        assert_eq!(back, pe);
    }
}
