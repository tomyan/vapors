//! Choosing the package whose example or binary `cargo xtask screenshot`
//! runs: the `[screenshot] packages` list and `cargo metadata`'s answer.

use super::*;
use crate::config::Config;

const METADATA: &str = r#"{"packages": [
  {"name": "vapors-platform", "targets": [
    {"name": "vapors_platform", "kind": ["lib"]},
    {"name": "clear_window", "kind": ["example"]},
    {"name": "image_window", "kind": ["example"]}]},
  {"name": "vapors-app", "targets": [
    {"name": "demo", "kind": ["example"]},
    {"name": "viewer", "kind": ["bin"]}]},
  {"name": "vapors-text", "targets": [
    {"name": "demo", "kind": ["example"]}]}
]}"#;

fn example(name: &str) -> Target {
    Target {
        name: name.to_owned(),
        is_binary: false,
    }
}

fn binary(name: &str) -> Target {
    Target {
        name: name.to_owned(),
        is_binary: true,
    }
}

fn listed(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn the_packages_setting_is_split_on_commas_and_trimmed() {
    // Given
    let config =
        Config::parse("[screenshot]\npackages = \"vapors-platform, vapors-app\"\n").expect("valid");

    // When
    let packages = packages(&config);

    // Then
    assert_eq!(packages, Ok(listed(&["vapors-platform", "vapors-app"])));
}

#[test]
fn an_empty_entry_in_the_packages_setting_is_refused() {
    // Given
    let config =
        Config::parse("[screenshot]\npackages = \"vapors-platform,,vapors-app\"\n").expect("valid");

    // When
    let refusal = packages(&config).expect_err("an empty entry");

    // Then
    assert!(refusal.contains("empty entry"), "{refusal}");
    assert!(
        refusal.contains("packages = \"vapors-platform,,vapors-app\""),
        "{refusal}"
    );
}

#[test]
fn the_old_single_package_key_is_refused_with_the_rename() {
    // Given
    let config = Config::parse("[screenshot]\npackage = \"vapors-platform\"\n").expect("valid");

    // When
    let refusal = packages(&config).expect_err("the old key");

    // Then
    assert!(
        refusal.contains("rename `package` to `packages`"),
        "{refusal}"
    );
}

#[test]
fn a_missing_packages_setting_is_refused_naming_the_key() {
    // Given
    let config = Config::parse("[web]\npackage = \"vapors-web-demo\"\n").expect("valid");

    // When
    let refusal = packages(&config).expect_err("no setting");

    // Then
    assert!(refusal.contains("screenshot.packages"), "{refusal}");
}

#[test]
fn an_example_is_found_in_the_one_listed_package_that_has_it() {
    // Given
    let packages = listed(&["vapors-platform", "vapors-app"]);

    // When
    let chosen = choose(&packages, METADATA, &example("demo"));

    // Then
    assert_eq!(chosen, Ok("vapors-app".to_owned()));
}

#[test]
fn a_binary_is_looked_for_among_binaries_not_examples() {
    // Given
    let packages = listed(&["vapors-platform", "vapors-app"]);

    // When
    let binary_choice = choose(&packages, METADATA, &binary("viewer"));
    let example_choice = choose(&packages, METADATA, &example("viewer"));

    // Then
    assert_eq!(binary_choice, Ok("vapors-app".to_owned()));
    assert!(example_choice.is_err());
}

#[test]
fn a_target_no_listed_package_has_is_refused_listing_what_each_has() {
    // Given
    let packages = listed(&["vapors-platform", "vapors-app"]);

    // When
    let refusal = choose(&packages, METADATA, &example("missing")).expect_err("absent");

    // Then
    assert!(refusal.contains("no example `missing`"), "{refusal}");
    assert!(
        refusal.contains("vapors-platform: clear_window, image_window"),
        "{refusal}"
    );
    assert!(refusal.contains("vapors-app: demo"), "{refusal}");
}

#[test]
fn a_target_in_two_listed_packages_is_refused_naming_both() {
    // Given
    let packages = listed(&["vapors-app", "vapors-text"]);

    // When
    let refusal = choose(&packages, METADATA, &example("demo")).expect_err("ambiguous");

    // Then
    assert!(refusal.contains("vapors-app, vapors-text"), "{refusal}");
}

#[test]
fn a_listed_package_outside_the_workspace_is_refused_by_name() {
    // Given
    let packages = listed(&["vapors-platform", "vapors-gone"]);

    // When
    let refusal = choose(&packages, METADATA, &example("clear_window")).expect_err("unknown");

    // Then
    assert!(
        refusal.contains("`vapors-gone` is not a workspace package"),
        "{refusal}"
    );
}

#[test]
fn metadata_of_an_unrecognised_shape_is_refused() {
    // Given
    let packages = listed(&["vapors-platform"]);

    // When
    let refusal = choose(
        &packages,
        r#"{"packages": [{"name": 3, "targets": []}]}"#,
        &example("clear_window"),
    )
    .expect_err("bad shape");

    // Then
    assert!(refusal.contains("cargo metadata"), "{refusal}");
}
