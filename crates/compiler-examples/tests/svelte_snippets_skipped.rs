//! Every one of Svelte 5's 54 `runtime-runes/samples/snippet-*`
//! (checked 2026-10-26 in `~/projects/svelte`) is either ported, in
//! `svelte_snippets.rs` and `svelte_snippets_components.rs`, or skipped
//! here with the feature a `.wisp` lacks. `snippet-expressions` and
//! `snippet-prop-reactive` are ported in `svelte_snippets_values.rs` (R1d).

use std::collections::HashSet;

/// Ported, by sample name.
const PORTED: &[&str] = &[
    "snippet-after-text",
    "snippet-argument",
    "snippet-argument-call-expression",
    "snippet-argument-multiple",
    "snippet-comment-sibling",
    "snippet-deconflict",
    "snippet-duplicate-children",
    "snippet-dynamic-children",
    "snippet-hoisting-2",
    "snippet-prop-explicit",
    "snippet-prop-explicit-no-args",
    "snippet-prop-implicit",
    "snippet-scope",
    "snippet-whitespace",
    "snippet-expressions",
    "snippet-prop-reactive",
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

#[test]
fn every_svelte_snippet_sample_is_ported_or_skipped_once() {
    // Given
    let names: Vec<&str> = PORTED
        .iter()
        .copied()
        .chain(SKIPPED.iter().map(|(name, _)| *name))
        .collect();

    // Then: 54 samples, none listed twice
    let unique: HashSet<&str> = names.iter().copied().collect();
    assert_eq!(unique.len(), names.len(), "a sample is listed twice");
    assert_eq!(names.len(), 54, "Svelte has 54 snippet-* samples");
}
