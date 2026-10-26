//! Svelte's `snippet-*` samples that pass snippets between components,
//! ported to `.wisp` under `src/svelte_snippets/` (1-R1, R1c); see
//! `svelte_snippets.rs` for the conventions.

#[path = "svelte/mount.rs"]
mod mount;

use std::fmt::Write;

use mount::{element, html, mounted};
use vapors_compiler_examples::svelte_snippets::{
    deconflict::{Deconflict, DeconflictProps},
    duplicate_children::{DuplicateChildren, DuplicateChildrenProps},
    dynamic_children::{DynamicChildren, DynamicChildrenProps},
    prop_explicit::{PropExplicit, PropExplicitProps},
    prop_explicit_no_args::{PropExplicitNoArgs, PropExplicitNoArgsProps},
    prop_implicit::{PropImplicit, PropImplicitProps},
    scope::{Scope, ScopeProps},
    whitespace::{Whitespace, WhitespaceProps},
};
use vapors_dom::recording::RecordingDom;

const CLICKS_0: &str = "<p>clicks: 0</p><button>click me</button>";
const CLICKS_1: &str = "<p>clicks: 1</p><button>click me</button>";

/// `<p style="color: {color}">n</p>` for each n.
fn paragraphs(color: &str, numbers: std::ops::RangeInclusive<i32>) -> String {
    numbers.fold(String::new(), |mut html, n| {
        let _ = write!(html, "<p style=\"color: {color}\">{n}</p>");
        html
    })
}

#[test]
fn snippet_deconflict() {
    // Given: two snippets named `x`, one in each of two sibling lists
    let (dom, _c) = mounted(|dom, root| Deconflict::mount(dom, root, DeconflictProps::default()));

    // When
    for _ in 0..3 {
        RecordingDom::click(&dom, element(&dom, "button", 0));
    }

    // Then: each list renders its own
    let expected = format!(
        "<button>push</button><div style=\"display: grid; grid-template-columns: 1fr 1fr\"><div>{}</div><div>{}</div></div>",
        paragraphs("red", 1..=6),
        paragraphs("blue", 1..=6)
    );
    assert_eq!(html(&dom), expected);
}

#[test]
fn snippet_duplicate_children() {
    // When: two tags of one component, each with its own `children`
    let (dom, _c) =
        mounted(|dom, root| DuplicateChildren::mount(dom, root, DuplicateChildrenProps::default()));

    // Then
    assert_eq!(html(&dom), "<div>Hello</div><div>World</div>");
}

#[test]
fn snippet_dynamic_children() {
    // Given: adapted, Svelte's `change` callback prop is a `show` signal
    // the child sets, as a .wisp prop is reactive only as a signal
    let (dom, _c) =
        mounted(|dom, root| DynamicChildren::mount(dom, root, DynamicChildrenProps::default()));
    assert_eq!(html(&dom), "<button><span>hidden</span></button>");

    // When
    RecordingDom::click(&dom, element(&dom, "button", 0));

    // Then
    assert_eq!(html(&dom), "<button><span>showing</span></button>");
}

#[test]
fn snippet_prop_explicit() {
    // Given: a snippet passed as an attribute, rendered with the child's count
    let (dom, _c) =
        mounted(|dom, root| PropExplicit::mount(dom, root, PropExplicitProps::default()));
    assert_eq!(html(&dom), CLICKS_0);

    // When
    RecordingDom::click(&dom, element(&dom, "button", 0));

    // Then
    assert_eq!(html(&dom), CLICKS_1);
}

#[test]
fn snippet_prop_explicit_no_args() {
    // When
    let (dom, _c) = mounted(|dom, root| {
        PropExplicitNoArgs::mount(dom, root, PropExplicitNoArgsProps::default())
    });

    // Then
    assert_eq!(html(&dom), "<p>foo</p>");
}

#[test]
fn snippet_prop_implicit() {
    // Given: the snippet declared inside the tag is the child's `foo`
    let (dom, _c) =
        mounted(|dom, root| PropImplicit::mount(dom, root, PropImplicitProps::default()));
    assert_eq!(html(&dom), CLICKS_0);

    // When
    RecordingDom::click(&dom, element(&dom, "button", 0));

    // Then
    assert_eq!(html(&dom), CLICKS_1);
}

#[test]
fn snippet_scope() {
    // When: a snippet in a tag inside a loop reads the loop's binding.
    // Adapted: the child renders `inner` without Svelte's `{#if inner}`,
    // as a .wisp snippet prop always has a value
    let (dom, _c) = mounted(|dom, root| Scope::mount(dom, root, ScopeProps::default()));

    // Then
    let expected = "Svelte".chars().fold(String::new(), |mut html, c| {
        let _ = write!(html, "<p>{c}</p><a href=\"#\">{c}</a>");
        html
    });
    assert_eq!(html(&dom), expected);
}

#[test]
fn snippet_whitespace() {
    // When: adapted, the text is inside a `<div>` (a .wisp has no text at
    // its top level), and a .wisp's whitespace rule applies: a run with a
    // newline between two non-text things is dropped, other text is kept
    let (dom, _c) = mounted(|dom, root| Whitespace::mount(dom, root, WhitespaceProps::default()));

    // Then: Svelte's is `A B C D <pre>Testing\n123          ;\n    456</pre>`
    assert_eq!(
        html(&dom),
        "<div>\nA\n\nB\nC\nD\n</div><pre>\n    Testing\n123          ;\n    456\n</pre>"
    );
}
