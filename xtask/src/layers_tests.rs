//! The rules read from `[layers.forbid]`.

use super::*;

#[test]
fn each_rule_lists_its_forbidden_crates_trimmed() {
    // Given
    let config =
        Config::parse("[layers.forbid]\n\"touche-engine\" = \"touche-viewer, vapors-platform\"\n")
            .expect("config");

    // When
    let rules = rules(&config).expect("rules");

    // Then
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].crate_name, "touche-engine");
    assert_eq!(rules[0].forbidden, ["touche-viewer", "vapors-platform"]);
}

#[test]
fn a_rule_with_an_empty_name_is_refused_naming_it() {
    // Given
    let config =
        Config::parse("[layers.forbid]\n\"touche-engine\" = \"touche-viewer,\"\n").expect("config");

    // When
    let refusal = rules(&config).expect_err("refused");

    // Then
    assert!(refusal.contains("touche-engine"), "{refusal}");
    assert!(refusal.contains("empty name"), "{refusal}");
}

#[test]
fn a_layers_table_of_another_shape_is_refused_naming_it() {
    for (text, needle) in [
        ("[layers]\nforbid = \"a\"\n", "[layers]"),
        ("[layers.allow]\n\"a\" = \"b\"\n", "[layers.allow]"),
        ("[layers.forbid]\n", "[layers.forbid] is empty"),
    ] {
        let refusal = rules(&Config::parse(text).expect("config")).expect_err(text);
        assert!(refusal.contains(needle), "wanted {needle} in: {refusal}");
    }
}
