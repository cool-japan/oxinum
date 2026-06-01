//! Serde `Serialize` / `Deserialize` implementations for [`BigFloat`].
//!
//! The wire format uses a flat repr struct so serde's derive machinery handles
//! the structural mapping. Deserialization validates all invariants via a
//! `TryFrom<BigFloatRepr>` shim before constructing the final value.
//!
//! **Sign encoding**: `Sign` has no public serde impl; it is encoded as a
//! `bool` matching the convention used in `oxinum-int` (same as BigInt):
//! `false == Positive`, `true == Negative`.
//!
//! Enabled only when the `serde` feature is active.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use oxinum_core::Sign;
use oxinum_int::native::BigUint;

use super::float::{BigFloat, FloatClass};

// ---------------------------------------------------------------------------
// Sign bool helper (mirrors `oxinum-int` sign_serde)
// ---------------------------------------------------------------------------

mod sign_serde {
    use oxinum_core::Sign;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(s: &Sign, ser: S) -> Result<S::Ok, S::Error> {
        bool::from(*s).serialize(ser)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Sign, D::Error> {
        bool::deserialize(de).map(Sign::from)
    }
}

// ---------------------------------------------------------------------------
// FloatClass serde helper (no derive on FloatClass — it lives in float.rs)
// ---------------------------------------------------------------------------

mod class_serde {
    use super::FloatClass;
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(class: &FloatClass, s: S) -> Result<S::Ok, S::Error> {
        let name = match class {
            FloatClass::Finite => "Finite",
            FloatClass::Infinite => "Infinite",
            FloatClass::Nan => "Nan",
        };
        s.serialize_str(name)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<FloatClass, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "Finite" => Ok(FloatClass::Finite),
            "Infinite" => Ok(FloatClass::Infinite),
            "Nan" => Ok(FloatClass::Nan),
            other => Err(D::Error::unknown_variant(
                other,
                &["Finite", "Infinite", "Nan"],
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Wire repr
// ---------------------------------------------------------------------------

/// Flat wire representation of a [`BigFloat`].
///
/// Derived serde handles JSON/MessagePack/etc. encoding automatically. The
/// `TryFrom` impl re-establishes all BigFloat invariants on the way back.
///
/// The `class` field defaults to [`FloatClass::Finite`] so that payloads
/// serialised before Wave 2 (which lacked this field) continue to deserialize
/// correctly.
#[derive(Serialize, Deserialize)]
struct BigFloatRepr {
    #[serde(default)]
    #[serde(with = "class_serde")]
    class: FloatClass,
    #[serde(with = "sign_serde")]
    sign: Sign,
    mantissa: BigUint,
    exponent: i64,
    precision: u32,
}

// ---------------------------------------------------------------------------
// TryFrom: invariant gate
// ---------------------------------------------------------------------------

impl TryFrom<BigFloatRepr> for BigFloat {
    type Error = String;

    fn try_from(r: BigFloatRepr) -> Result<Self, Self::Error> {
        // Invariant 1: precision > 0
        if r.precision == 0 {
            return Err("BigFloat precision must be > 0".to_string());
        }

        match r.class {
            FloatClass::Nan => Ok(BigFloat::nan(r.precision)),
            FloatClass::Infinite => Ok(if r.sign == Sign::Negative {
                BigFloat::neg_infinity(r.precision)
            } else {
                BigFloat::infinity(r.precision)
            }),
            FloatClass::Finite => {
                if r.mantissa.is_zero() {
                    // Invariant 2: canonical zero has sign=Positive, exponent=0
                    Ok(BigFloat::zero(r.precision))
                } else {
                    // Invariant 3: mantissa.bit_length() == precision for non-zero values
                    let bl = r.mantissa.bit_length() as u32;
                    if bl != r.precision {
                        return Err(format!(
                            "BigFloat invariant violated: mantissa bit_length {bl} != precision {}",
                            r.precision
                        ));
                    }
                    Ok(BigFloat {
                        class: FloatClass::Finite,
                        sign: r.sign,
                        mantissa: r.mantissa,
                        exponent: r.exponent,
                        precision: r.precision,
                    })
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Serialize
// ---------------------------------------------------------------------------

impl Serialize for BigFloat {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        BigFloatRepr {
            class: self.class,
            sign: self.sign,
            mantissa: self.mantissa.clone(),
            exponent: self.exponent,
            precision: self.precision,
        }
        .serialize(s)
    }
}

// ---------------------------------------------------------------------------
// Deserialize
// ---------------------------------------------------------------------------

impl<'de> Deserialize<'de> for BigFloat {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        BigFloatRepr::deserialize(d)
            .and_then(|r| BigFloat::try_from(r).map_err(serde::de::Error::custom))
    }
}
