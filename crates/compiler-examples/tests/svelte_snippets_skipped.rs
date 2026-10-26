//! Every one of Svelte 5's 54 `runtime-runes/samples/snippet-*`
//! (checked 2026-10-26 in `~/projects/svelte`) is either ported, in
//! `svelte_snippets.rs` and `svelte_snippets_components.rs`, or skipped
//! here with the feature a `.wisp` lacks. `snippet-expressions` and
//! `snippet-prop-reactive` are ported in `svelte_snippets_values.rs` (R1d).

use std::collections::HashSet;

/// Ported: the sample, its `.wisp` under `src/svelte_snippets/` and its
/// test function.
const PORTED: &[(&str, &str, &str)] = &[
    ("snippet-after-text", "after_text", "snippet_after_text"),
    ("snippet-argument", "argument", "snippet_argument"),
    (
        "snippet-argument-call-expression",
        "argument_call_expression",
        "snippet_argument_call_expression",
    ),
    (
        "snippet-argument-multiple",
        "argument_multiple",
        "snippet_argument_multiple",
    ),
    ("snippet-deconflict", "deconflict", "snippet_deconflict"),
    (
        "snippet-duplicate-children",
        "duplicate_children",
        "snippet_duplicate_children",
    ),
    (
        "snippet-dynamic-children",
        "dynamic_children",
        "snippet_dynamic_children",
    ),
    ("snippet-hoisting-2", "hoisting2", "snippet_hoisting_2"),
    (
        "snippet-prop-explicit",
        "prop_explicit",
        "snippet_prop_explicit",
    ),
    (
        "snippet-prop-explicit-no-args",
        "prop_explicit_no_args",
        "snippet_prop_explicit_no_args",
    ),
    (
        "snippet-prop-implicit",
        "prop_implicit",
        "snippet_prop_implicit",
    ),
    ("snippet-scope", "scope", "snippet_scope"),
    ("snippet-whitespace", "whitespace", "snippet_whitespace"),
    ("snippet-expressions", "expressions", "snippet_expressions"),
    (
        "snippet-prop-reactive",
        "prop_reactive",
        "snippet_prop_reactive",
    ),
];

/// The test files the ported samples' functions are in.
const TESTS: [&str; 3] = [
    include_str!("svelte_snippets.rs"),
    include_str!("svelte_snippets_components.rs"),
    include_str!("svelte_snippets_values.rs"),
];

/// Skipped, with why.
const SKIPPED: &[(&str, &str)] = &[
    (
        "snippet-access-in-script",
        "the script reads a template snippet",
    ),
    ("snippet-argument-destructured", "destructured parameters"),
    (
        "snippet-argument-destructured-multiple",
        "destructured parameters",
    ),
    (
        "snippet-block-without-render-tag-dev",
        "a snippet in a hole is a Rust type error, not a dev warning",
    ),
    (
        "snippet-block-without-render-tag-prod",
        "a snippet in a hole is a Rust type error",
    ),
    (
        "snippet-children-without-render-tag-dev",
        "a snippet in a hole is a Rust type error, not a dev warning",
    ),
    (
        "snippet-children-without-render-tag-dev-prod",
        "a snippet in a hole is a Rust type error",
    ),
    ("snippet-complicated-defaults", "default parameter values"),
    (
        "snippet-comment-sibling",
        "a .wisp template has no comment syntax; the refusal is pinned below",
    ),
    ("snippet-const", "{@const}"),
    ("snippet-const-same-block", "{@const}"),
    ("snippet-default-arg", "default parameter values"),
    ("snippet-destructure-array", "destructured parameters"),
    ("snippet-hoisted-module", "<script module> exports"),
    ("snippet-hoisted-module-2", "<script module> exports"),
    ("snippet-hoisting", "the script reads a template snippet"),
    ("snippet-hoisting-3", "<script module> exports"),
    (
        "snippet-hoisting-4",
        "a component chosen by an expression (`<object.property />`)",
    ),
    (
        "snippet-invalid-call",
        "the script calls a template snippet",
    ),
    ("snippet-namespace-1", "SVG namespace"),
    ("snippet-namespace-2", "SVG namespace and foreignObject"),
    ("snippet-optional-arguments", "optional parameters"),
    (
        "snippet-optional-arguments-defaults",
        "default parameter values",
    ),
    ("snippet-raw", "createRawSnippet"),
    (
        "snippet-raw-component",
        "createRawSnippet, SSR and hydration",
    ),
    ("snippet-raw-component-ssr-dev", "createRawSnippet, SSR"),
    ("snippet-raw-teardown", "createRawSnippet"),
    ("snippet-reactive", "the script reads a template snippet"),
    (
        "snippet-reactive-args",
        "the script reads a template snippet; destructuring and spread",
    ),
    ("snippet-ref", "bind:this"),
    ("snippet-select", "spread props and <select> value"),
    ("snippet-slot-let-error", "<slot> and let:"),
    ("snippet-slot-let-renamed-children-error", "<slot> and let:"),
    (
        "snippet-slot-no-error",
        "children rendered with an argument is a Rust type error",
    ),
    ("snippet-spread", "spread props and <slot>"),
    ("snippet-store", "stores"),
    (
        "snippet-top-level",
        "a prop's default is a template snippet the script cannot see",
    ),
    (
        "snippet-typescript",
        "optional, default and destructured parameters",
    ),
    ("snippet-undefined", "the script reads a template snippet"),
];

/// Svelte's sample names, vendored with the commit they were read from.
fn svelte_samples() -> HashSet<&'static str> {
    include_str!("svelte/samples.txt")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect()
}

#[test]
fn every_svelte_snippet_sample_is_ported_or_skipped_once() {
    // Given
    let ported = PORTED.iter().map(|(name, _, _)| *name);
    let names: Vec<&str> = ported
        .chain(SKIPPED.iter().map(|(name, _)| *name))
        .collect();

    // Then: none listed twice, and exactly Svelte's names
    let listed: HashSet<&str> = names.iter().copied().collect();
    assert_eq!(listed.len(), names.len(), "a sample is listed twice");
    let samples = svelte_samples();
    let missing: Vec<&&str> = samples.difference(&listed).collect();
    let unknown: Vec<&&str> = listed.difference(&samples).collect();
    assert!(
        missing.is_empty() && unknown.is_empty(),
        "not listed: {missing:?}; not Svelte's: {unknown:?}"
    );
}

#[test]
fn every_ported_sample_has_its_wisp_and_its_test() {
    for (name, wisp, test) in PORTED {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("src/svelte_snippets/{wisp}.wisp"));
        assert!(path.is_file(), "{name}: no {}", path.display());
        let function = format!("fn {test}()");
        assert!(
            TESTS.iter().any(|file| file.contains(&function)),
            "{name}: no {function} in the svelte_snippets tests"
        );
    }
}

#[test]
fn snippet_comment_sibling_is_refused_as_a_comment() {
    // When: the sample's markup, its comment beside the children snippet
    let text = "<script>\nuse super::c::{C, CProps};\n</script>\n\n<C>\n\t<!-- I'm just a poor comment -->\n\t{snippet children()}The content{/snippet}\n</C>\n";
    let refusal = vapors_compiler::compile_str("comment_sibling.wisp", text).expect_err("refused");

    // Then: at the comment, naming the fix
    assert_eq!((refusal.line, refusal.col), (6, 2), "{refusal}");
    assert!(refusal.message.contains("no comment syntax"), "{refusal}");
}
