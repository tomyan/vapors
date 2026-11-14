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
fn components_depending_on_vapors_css_normally_is_still_refused_by_the_repository_rules() {
    // Given: the repository's own rule for vapors-components, and a
    // graph holding every crate it names, where the dependency on
    // vapors-css is linked, not a build script's
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let rules = rules(&Config::load_from(&root).expect("config")).expect("rules");
    let rule = rules
        .into_iter()
        .find(|rule| rule.crate_name == "vapors-components")
        .expect("the components rule");
    let package = |name: &str| serde_json::json!({ "id": name, "name": name });
    let node = |name: &str, deps: Vec<&str>| {
        let deps: Vec<_> = deps
            .into_iter()
            .map(|dep| serde_json::json!({ "pkg": dep, "dep_kinds": [{ "kind": null }] }))
            .collect();
        serde_json::json!({ "id": name, "deps": deps })
    };
    let mut packages = vec![package("vapors-components")];
    let mut nodes = vec![node("vapors-components", vec!["vapors-css"])];
    for name in &rule.forbidden {
        packages.push(package(name));
        nodes.push(node(name, Vec::new()));
    }
    let graph = parse(&serde_json::json!({ "packages": packages, "resolve": { "nodes": nodes } }))
        .expect("graph");

    // When
    let found = violations(&graph, &[rule]).expect("checked");

    // Then
    assert_eq!(
        found,
        ["vapors-components depends on vapors-css: vapors-components -> vapors-css"]
    );
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
