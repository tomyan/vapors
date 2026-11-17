//! A request body parsed once, refusing a key that appears twice in one
//! object (review of #476-#478, N2a). `serde_json::Value` keeps the last
//! of a repeated key, and a parser upstream may keep the first: the proxy
//! would judge one request and forward another. Refused here, and the
//! proxy forwards this parse, re-serialized, never the bytes it was given
//! (`api_server_proxy.rs`).

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

/// `bytes` as JSON with no repeated key in any object.
///
/// # Errors
///
/// Not JSON, or a key repeated, naming it.
pub fn parsed(bytes: &[u8]) -> Result<Value, String> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let Strict(value) = Strict::deserialize(&mut deserializer)
        .map_err(|e| format!("a request body that is not JSON with no repeated key: {e}"))?;
    deserializer
        .end()
        .map_err(|e| format!("a request body with more after its JSON: {e}"))?;
    Ok(value)
}

/// A value whose objects hold no key twice.
struct Strict(Value);

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor).map(Strict)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("JSON")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("a number JSON cannot hold"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(Strict(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let Strict(value) = map.next_value()?;
            if object.insert(key.clone(), value).is_some() {
                return Err(de::Error::custom(format!(
                    "the key {key:?} appears twice in one object"
                )));
            }
        }
        Ok(Value::Object(object))
    }
}

#[cfg(test)]
#[path = "api_json_strict_tests.rs"]
mod tests;
