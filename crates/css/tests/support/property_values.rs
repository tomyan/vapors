//! Helpers that were in `tests/property_values.rs`.
#![allow(dead_code)]

use super::sheet;

/// The longhand values of one declaration, as `name: value` text.
pub fn longhands(decl: &str) -> Vec<String> {
    let s = sheet(&format!("a {{ {decl}; }}"));
    s.rules[0].declarations[0]
        .longhands
        .iter()
        .map(|l| format!("{}: {}", l.id.name(), l.value.to_css()))
        .collect()
}

pub fn accepts(decl: &str) {
    accepts_as(decl, decl);
}

/// `decl` parses and serializes canonically as `want`.
pub fn accepts_as(decl: &str, want: &str) {
    let got = longhands(decl);
    assert_eq!(got.len(), 1, "{decl}: {got:?}");
    assert_eq!(got[0], want, "{decl}");
}
