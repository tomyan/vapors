//! The dependency graph from `cargo metadata`, and the paths a rule forbids.

use serde_json::{Value, json};

use super::*;

/// A crate's dependencies: (name, kind), kind `None` for a normal one.
type Deps<'a> = &'a [(&'a str, Option<&'a str>)];

/// A metadata document from each crate's name and dependencies.
fn metadata(crates: &[(&str, Deps<'_>)]) -> Value {
    let id = |name: &str| format!("path+file:///w/{name}#{name}@0.0.0");
    let packages: Vec<Value> = crates
        .iter()
        .map(|(name, _)| json!({ "id": id(name), "name": name }))
        .collect();
    let nodes: Vec<Value> = crates
        .iter()
        .map(|(name, deps)| {
            let deps: Vec<Value> = deps
                .iter()
                .map(|(dep, kind)| json!({ "pkg": id(dep), "dep_kinds": [{ "kind": kind, "target": null }] }))
                .collect();
            json!({ "id": id(name), "deps": deps })
        })
        .collect();
    json!({ "packages": packages, "resolve": { "nodes": nodes } })
}

fn rule(name: &str, forbidden: &[&str]) -> Rule {
    Rule {
        crate_name: name.to_owned(),
        forbidden: forbidden.iter().map(|f| (*f).to_owned()).collect(),
    }
}

#[test]
fn a_crate_clear_of_its_forbidden_crates_has_no_violations() {
    // Given
    let graph = parse(&metadata(&[
        ("document", &[("serde", None)]),
        ("serde", &[]),
        ("viewer", &[("document", None)]),
    ]))
    .expect("graph");

    // When / Then
    assert_eq!(
        violations(&graph, &[rule("document", &["viewer"])]),
        Ok(Vec::new())
    );
}

#[test]
fn a_forbidden_crate_reached_through_another_is_named_with_its_path() {
    // Given
    let graph = parse(&metadata(&[
        ("engine", &[("helper", None)]),
        ("helper", &[("platform", Some("build"))]),
        ("platform", &[]),
    ]))
    .expect("graph");

    // When
    let found = violations(&graph, &[rule("engine", &["platform"])]).expect("checked");

    // Then
    assert_eq!(
        found,
        ["engine depends on platform: engine -> helper -> platform"]
    );
}

#[test]
fn a_dev_dependency_is_not_a_layering_dependency() {
    // Given: tests may use a UI crate; the library does not
    let graph = parse(&metadata(&[
        ("engine", &[("platform", Some("dev"))]),
        ("platform", &[]),
    ]))
    .expect("graph");

    // When / Then
    assert_eq!(
        violations(&graph, &[rule("engine", &["platform"])]),
        Ok(Vec::new())
    );
}

#[test]
fn a_rule_naming_a_crate_not_in_the_graph_is_refused_naming_it() {
    // Given
    let graph = parse(&metadata(&[("engine", &[])])).expect("graph");

    // When
    let refusal = violations(&graph, &[rule("engine", &["platfrom"])]).expect_err("refused");

    // Then
    assert!(refusal.contains("platfrom"), "{refusal}");
    assert!(refusal.contains("[layers.forbid]"), "{refusal}");
}

#[test]
fn metadata_without_a_resolve_graph_is_refused() {
    // When
    let refusal = parse(&json!({ "packages": [] })).expect_err("refused");

    // Then
    assert!(refusal.contains("resolve"), "{refusal}");
}
