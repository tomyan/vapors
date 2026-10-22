//! Helpers that were in `tests/value_var.rs`.
#![allow(dead_code)]

use std::collections::BTreeMap;
use vapors_css::ast::ComponentValue;
use vapors_css::serialize::component_values;
use vapors_css::value::{VarError, substitute_vars};

pub fn tokens(text: &str) -> Vec<ComponentValue> {
    let sheet = vapors_css::parse(&format!("a {{ --v: {text}; }}")).expect("parses");
    sheet.rules[0].declarations[0].value.clone()
}

pub fn expand(value: &str, vars: &[(&str, &str)]) -> Result<String, VarError> {
    let table: BTreeMap<String, Vec<ComponentValue>> = vars
        .iter()
        .map(|(k, v)| ((*k).to_owned(), tokens(v)))
        .collect();
    let lookup = |name: &str| table.get(name).map(Vec::as_slice);
    substitute_vars(&tokens(value), &lookup).map(|v| component_values(&v))
}

pub fn ok(value: &str, vars: &[(&str, &str)]) -> String {
    expand(value, vars).unwrap_or_else(|e| panic!("{value}: {e}"))
}
