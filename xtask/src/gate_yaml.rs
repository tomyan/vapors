//! YAML for the gate, read with yaml-rust2 (YAML 1.2, maintained, no C
//! library) into `serde_json` values, which the gate's code walks. One
//! document per file; anything with no JSON form is refused by name.

use serde_json::{Map, Number, Value};
use yaml_rust2::{Yaml, YamlLoader};

/// The one YAML document in `text`, from the file named `what`.
///
/// # Errors
///
/// Text that is not YAML, not exactly one document, or holds a value with
/// no JSON form (a key that is not a scalar, an alias left unresolved).
pub fn parse(text: &str, what: &str) -> Result<Value, String> {
    let mut documents =
        YamlLoader::load_from_str(text).map_err(|e| format!("{what}: not YAML: {e}"))?;
    if documents.len() != 1 {
        return Err(format!(
            "{what}: {} documents, where the gate reads exactly 1",
            documents.len()
        ));
    }
    value(documents.remove(0)).map_err(|why| format!("{what}: {why}"))
}

fn value(yaml: Yaml) -> Result<Value, String> {
    Ok(match yaml {
        Yaml::Hash(hash) => {
            let mut map = Map::new();
            for (key, entry) in hash {
                map.insert(key_text(key)?, value(entry)?);
            }
            Value::Object(map)
        }
        Yaml::Array(items) => Value::Array(items.into_iter().map(value).collect::<Result<_, _>>()?),
        Yaml::String(text) => Value::String(text),
        Yaml::Integer(number) => Value::Number(number.into()),
        Yaml::Real(text) => real(&text)?,
        Yaml::Boolean(flag) => Value::Bool(flag),
        Yaml::Null => Value::Null,
        other => return Err(format!("a value the gate cannot read: {other:?}")),
    })
}

fn key_text(key: Yaml) -> Result<String, String> {
    match key {
        Yaml::String(text) | Yaml::Real(text) => Ok(text),
        Yaml::Integer(number) => Ok(number.to_string()),
        Yaml::Boolean(flag) => Ok(flag.to_string()),
        other => Err(format!(
            "a key that is not a string, number or bool: {other:?}"
        )),
    }
}

fn real(text: &str) -> Result<Value, String> {
    text.parse::<f64>()
        .ok()
        .and_then(Number::from_f64)
        .map(Value::Number)
        .ok_or_else(|| format!("a number the gate cannot read: {text}"))
}

#[cfg(test)]
#[path = "gate_yaml_tests.rs"]
mod tests;
