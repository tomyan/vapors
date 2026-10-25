//! The rules read from `[imports.<crate>]`.

use super::*;

#[test]
fn each_crate_gets_its_allowed_paths() {
    // Given
    let config = Config::parse(
        "[imports.dom_api]\n\"crates/web/\" = \"the page\"\n\"crates/runtime/src/key_web.rs\" = \"keys\"\n",
    )
    .expect("config");

    // When
    let rules = rules(&config).expect("rules");

    // Then
    let allow = &rules["dom_api"];
    assert_eq!(allow.len(), 2);
    assert_eq!(allow["crates/runtime/src/key_web.rs"], "keys");
}

#[test]
fn an_allowed_path_that_does_not_exist_is_refused_naming_it() {
    // Given
    let rules =
        rules(&Config::parse("[imports.dom_api]\n\"crates/gone/\" = \"moved\"\n").expect("config"))
            .expect("rules");

    // When
    let refusal = check_paths_exist(Path::new("."), &rules).expect_err("refused");

    // Then
    assert!(refusal.contains("[imports.dom_api]"), "{refusal}");
    assert!(refusal.contains("crates/gone/"), "{refusal}");
}

#[test]
fn an_imports_table_of_another_shape_is_refused_naming_it() {
    for (text, needle) in [
        ("[imports]\ndom_api = \"crates/web/\"\n", "[imports]"),
        (
            "[imports.dom-api]\n\"crates/web/\" = \"why\"\n",
            "dom-api is not a Rust crate name",
        ),
        ("[imports.dom_api]\n", "[imports.dom_api] is empty"),
    ] {
        let refusal = rules(&Config::parse(text).expect("config")).expect_err(text);
        assert!(refusal.contains(needle), "wanted {needle} in: {refusal}");
    }
}
