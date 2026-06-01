//! Serde `Serialize` / `Deserialize` implementations for [`BigRational`].
//!
//! The wire format uses a flat repr struct with `num: BigInt` and `den:
//! BigUint`, both of which already have serde impls (gated on the `serde`
//! feature of `oxinum-int`). Deserialization re-establishes all invariants
//! (non-zero denominator, GCD == 1) via `BigRational::from_parts`.
//!
//! Enabled only when the `serde` feature is active.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use oxinum_int::native::{BigInt, BigUint};

use super::rational::BigRational;

// ---------------------------------------------------------------------------
// Wire repr
// ---------------------------------------------------------------------------

/// Flat wire representation of a [`BigRational`].
///
/// Both `BigInt` and `BigUint` have their own serde impls, so derive handles
/// the structural encoding. The invariant gate on deserialization is in the
/// `TryFrom` impl.
#[derive(Serialize, Deserialize)]
struct BigRationalRepr {
    num: BigInt,
    den: BigUint,
}

// ---------------------------------------------------------------------------
// TryFrom: invariant gate
// ---------------------------------------------------------------------------

impl TryFrom<BigRationalRepr> for BigRational {
    type Error = String;

    fn try_from(r: BigRationalRepr) -> Result<Self, Self::Error> {
        // Delegate to from_parts which checks den != 0 and auto-reduces.
        BigRational::from_parts(r.num, r.den).map_err(|e| format!("{e}"))
    }
}

// ---------------------------------------------------------------------------
// Serialize
// ---------------------------------------------------------------------------

impl Serialize for BigRational {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        BigRationalRepr {
            num: self.num.clone(),
            den: self.den.clone(),
        }
        .serialize(s)
    }
}

// ---------------------------------------------------------------------------
// Deserialize
// ---------------------------------------------------------------------------

impl<'de> Deserialize<'de> for BigRational {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        BigRationalRepr::deserialize(d)
            .and_then(|r| BigRational::try_from(r).map_err(serde::de::Error::custom))
    }
}
