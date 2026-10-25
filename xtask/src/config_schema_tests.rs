//! The tables and keys the factory reads; anything else is refused.

use super::*;
use crate::config::Config;

fn checked(text: &str) -> Result<(), String> {
    check_known(&Config::parse(text).expect("parses")).map_err(|e| e.to_string())
}

#[test]
fn the_tables_and_keys_the_factory_reads_are_accepted() {
    let text = "[web]\npackage = \"p\"\nproduct = \"x\"\n[files.allow]\n\"a.rs\" = \"why\"\n[layers.forbid]\n\"a\" = \"b\"\n[imports.dom_api]\n\"crates/\" = \"why\"\n[gate]\ntwin = \"../vapors\"\n";
    assert_eq!(checked(text), Ok(()));
}

#[test]
fn a_misspelt_table_is_refused_naming_its_line() {
    // When
    let refusal =
        checked("[web]\npackage = \"p\"\n\n[layer.forbid]\n\"a\" = \"b\"\n").expect_err("refused");

    // Then
    assert!(refusal.contains(":4:"), "{refusal}");
    assert!(
        refusal.contains("[layer.forbid] is not a table the factory reads"),
        "{refusal}"
    );
}

#[test]
fn an_unknown_key_in_a_known_table_is_refused_naming_its_line() {
    // When
    let refusal = checked("[web]\npackage = \"p\"\npackge = \"q\"\n").expect_err("refused");

    // Then
    assert!(refusal.contains(":3:"), "{refusal}");
    assert!(refusal.contains("web.packge"), "{refusal}");
    assert!(refusal.contains("package, product"), "{refusal}");
}

#[test]
fn an_empty_table_of_an_unknown_name_is_refused_too() {
    let refusal = checked("[webb]\n").expect_err("refused");
    assert!(refusal.contains("[webb]"), "{refusal}");
}
