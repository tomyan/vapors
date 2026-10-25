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
    let rules = rules(&config);

    // Then
    let allow = &rules["dom_api"];
    assert_eq!(allow.len(), 2);
    assert_eq!(allow["crates/runtime/src/key_web.rs"], "keys");
}

#[test]
fn an_allowed_path_that_does_not_exist_is_refused_naming_it() {
    // Given
    let rules =
        rules(&Config::parse("[imports.dom_api]\n\"crates/gone/\" = \"moved\"\n").expect("config"));

    // When
    let refusal = check_paths_exist(Path::new("."), &rules).expect_err("refused");

    // Then
    assert!(refusal.contains("[imports.dom_api]"), "{refusal}");
    assert!(refusal.contains("crates/gone/"), "{refusal}");
}
