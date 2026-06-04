//! Serde `Serialize` / `Deserialize` implementations for [`CBig`].
//!
//! The wire format uses a flat repr struct with `re: DBig` and `im: DBig`,
//! both of which already have serde impls (gated on the `dashu-float/serde`
//! feature, which this crate's `serde` feature enables). Deserialization
//! reconstructs the value via [`CBig::from_parts`]; complex numbers carry no
//! structural invariant beyond holding two independent components, so no
//! validating `TryFrom` shim is required.
//!
//! Enabled only when the `serde` feature is active.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::CBig;
use crate::DBig;

// ---------------------------------------------------------------------------
// Wire repr
// ---------------------------------------------------------------------------

/// Flat wire representation of a [`CBig`].
///
/// `DBig` carries its own serde impl, so derive handles the structural
/// encoding. There is no invariant to re-establish on deserialization, so the
/// `Deserialize` impl reconstructs directly via [`CBig::from_parts`].
#[derive(Serialize, Deserialize)]
struct CBigRepr {
    re: DBig,
    im: DBig,
}

// ---------------------------------------------------------------------------
// Serialize
// ---------------------------------------------------------------------------

impl Serialize for CBig {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        CBigRepr {
            re: self.re.clone(),
            im: self.im.clone(),
        }
        .serialize(s)
    }
}

// ---------------------------------------------------------------------------
// Deserialize
// ---------------------------------------------------------------------------

impl<'de> Deserialize<'de> for CBig {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        CBigRepr::deserialize(d).map(|repr| CBig::from_parts(repr.re, repr.im))
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
        let z = CBig::from_f64(3.0, -4.0).expect("finite parts");
        let json = serde_json::to_string(&z).expect("serialize CBig");
        let back: CBig = serde_json::from_str(&json).expect("deserialize CBig");
        assert_eq!(back.re().to_string(), z.re().to_string());
        assert_eq!(back.im().to_string(), z.im().to_string());
    }
}
