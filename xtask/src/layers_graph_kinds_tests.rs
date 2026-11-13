//! Which dependency kinds are layering dependencies: normal (`kind`
//! null in `cargo metadata`) counts, dev and build do not, anything
//! else is refused.

use serde_json::{Value, json};

use super::*;

/// `engine` depending on `platform` with the given `dep_kinds`.
fn engine_on_platform(kinds: Value) -> Result<Graph, String> {
    parse(&json!({
        "packages": [{ "id": "engine", "name": "engine" }, { "id": "platform", "name": "platform" }],
        "resolve": { "nodes": [
            { "id": "engine", "deps": [{ "pkg": "platform", "dep_kinds": kinds }] },
            { "id": "platform", "deps": [] }
        ] }
    }))
}

fn engine_rule() -> Rule {
    Rule {
        crate_name: "engine".to_owned(),
        forbidden: vec!["platform".to_owned()],
    }
}

#[test]
fn a_dev_dependency_is_not_a_layering_dependency() {
    // Given: tests may use a UI crate; the library does not
    let graph = engine_on_platform(json!([{ "kind": "dev" }])).expect("graph");

    // When / Then
    assert_eq!(violations(&graph, &[engine_rule()]), Ok(Vec::new()));
}

#[test]
fn a_build_dependency_is_not_a_layering_dependency() {
    // Given: a build script is not linked into the crate
    let graph = engine_on_platform(json!([{ "kind": "build" }])).expect("graph");

    // When / Then
    assert_eq!(violations(&graph, &[engine_rule()]), Ok(Vec::new()));
}

#[test]
fn a_dependency_that_is_both_normal_and_build_counts() {
    // Given: cargo lists one kind per use of the crate
    let graph = engine_on_platform(json!([{ "kind": "build" }, { "kind": null }])).expect("graph");

    // When / Then
    assert_eq!(
        violations(&graph, &[engine_rule()]),
        Ok(vec![
            "engine depends on platform: engine -> platform".to_owned()
        ])
    );
}

#[test]
fn a_dependency_kind_cargo_does_not_emit_is_refused_naming_it() {
    // When
    let refusal = engine_on_platform(json!([{ "kind": "optional" }])).expect_err("refused");

    // Then
    assert!(
        refusal.contains("platform") && refusal.contains("\"optional\""),
        "{refusal}"
    );
}
