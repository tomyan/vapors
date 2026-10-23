//! Reading YAML into the values the gate works on.

use super::*;

#[test]
fn a_workflow_reads_as_nested_maps_lists_and_scalars() {
    // When
    let value = parse("on:\n  push: {}\njobs:\n  ci:\n    timeout-minutes: 9\n    steps:\n      - run: echo hi\n        env:\n          FLAG: true\n", "ci.yml").expect("yaml");

    // Then
    assert!(
        value["on"].is_object(),
        "`on` stays a key, not a boolean (YAML 1.2)"
    );
    assert_eq!(value["jobs"]["ci"]["timeout-minutes"], 9);
    assert_eq!(value["jobs"]["ci"]["steps"][0]["run"], "echo hi");
    assert_eq!(value["jobs"]["ci"]["steps"][0]["env"]["FLAG"], true);
}

#[test]
fn text_that_is_not_one_yaml_document_is_refused_naming_the_file() {
    for (text, why) in [
        ("a: [1, 2", "not YAML"),
        ("a: 1\n---\nb: 2\n", "2 documents"),
        ("", "0 documents"),
    ] {
        let refusal = parse(text, "ci.yml").expect_err(why);
        assert!(refusal.starts_with("ci.yml: "), "{refusal}");
        assert!(refusal.contains(why), "{refusal}");
    }
}

#[test]
fn a_key_that_is_not_a_scalar_is_refused() {
    let refusal = parse("? [a, b]\n: 1\n", "ci.yml").expect_err("a list as a key");
    assert!(refusal.contains("a key that is not"), "{refusal}");
}
