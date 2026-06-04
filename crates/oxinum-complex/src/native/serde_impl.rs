//! Serde `Serialize` / `Deserialize` implementations for native
//! [`BigComplex`].
//!
//! The wire format uses a flat repr struct with `re: BigFloat` and
//! `im: BigFloat`, both of which already have serde impls (gated on the
//! `oxinum-float/serde` feature, which this crate's `serde` feature enables).
//! Deserialization reconstructs the value via [`BigComplex::from_parts`]; a
//! complex number carries no structural invariant beyond holding two
//! independent components, so no validating `TryFrom` shim is required (each
//! `BigFloat` re-establishes its own invariants on the way back).
//!
//! Enabled only when the `serde` feature is active.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::native::BigComplex;
use oxinum_float::native::BigFloat;

// ---------------------------------------------------------------------------
// Wire repr
// ---------------------------------------------------------------------------

/// Flat wire representation of a native [`BigComplex`].
///
/// `BigFloat` carries its own serde impl, so derive handles the structural
/// encoding. There is no invariant to re-establish at the complex level, so
/// the `Deserialize` impl reconstructs directly via [`BigComplex::from_parts`].
#[derive(Serialize, Deserialize)]
struct BigComplexRepr {
    re: BigFloat,
    im: BigFloat,
}

// ---------------------------------------------------------------------------
// Serialize
// ---------------------------------------------------------------------------

impl Serialize for BigComplex {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        BigComplexRepr {
            re: self.re.clone(),
            im: self.im.clone(),
        }
        .serialize(s)
    }
}

// ---------------------------------------------------------------------------
// Deserialize
// ---------------------------------------------------------------------------

impl<'de> Deserialize<'de> for BigComplex {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        BigComplexRepr::deserialize(d).map(|repr| BigComplex::from_parts(repr.re, repr.im))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let z = BigComplex::from_f64(1.0, 2.0, 64).expect("finite parts");
        let json = serde_json::to_string(&z).expect("serialize BigComplex");
        let back: BigComplex = serde_json::from_str(&json).expect("deserialize BigComplex");
        assert_eq!(back.re().to_f64(), z.re().to_f64());
        assert_eq!(back.im().to_f64(), z.im().to_f64());
    }
}
