//! Svelte 5's `runtime-runes/samples/snippet-*` (checked 2026-10-26 in
//! `~/projects/svelte`) ported to `.wisp` under
//! `src/svelte_snippets/` and run on the recording Dom (1-R1, R1c).
//! Svelte's `htmlEqual` ignores whitespace between tags; these compare
//! exact HTML, so the ports leave out the indentation Svelte's files
//! have. Adaptations are named on each test; the skipped samples, with
//! the reason for each, are listed in `svelte_snippets_skipped.rs`.

#[path = "svelte/mount.rs"]
mod mount;

use mount::{element, html, mounted};
use vapors_compiler_examples::svelte_snippets::{
    after_text::{AfterText, AfterTextProps},
    argument::{Argument, ArgumentProps},
    argument_call_expression::{ArgumentCallExpression, ArgumentCallExpressionProps},
    argument_multiple::{ArgumentMultiple, ArgumentMultipleProps},
    comment_sibling::{CommentSibling, CommentSiblingProps},
    hoisting2::{Hoisting2, Hoisting2Props},
};
use vapors_dom::recording::RecordingDom;
use vapors_signals::Signal;

#[test]
fn snippet_after_text() {
    // When: children after a text hole in the child
    let (dom, _c) = mounted(|dom, root| AfterText::mount(dom, root, AfterTextProps::default()));

    // Then
    assert_eq!(html(&dom), "<div>123</div>");
}

#[test]
fn snippet_argument() {
    // Given
    let (dom, _c) = mounted(|dom, root| Argument::mount(dom, root, ArgumentProps::default()));
    assert_eq!(html(&dom), "<p>clicks: 0</p><button>click me</button>");

    // When
    RecordingDom::click(&dom, element(&dom, "button", 0));

    // Then
    assert_eq!(html(&dom), "<p>clicks: 1</p><button>click me</button>");
}

#[test]
fn snippet_argument_call_expression() {
    // Given: Svelte logs `invoked`; the port counts calls in a signal.
    // Adapted: `a.foo`, `a.bar` are the tuple's fields, read from the memo
    let calls = Signal::new(0);

    // When
    let (dom, _c) = mounted(|dom, root| {
        ArgumentCallExpression::mount(dom, root, ArgumentCallExpressionProps { calls })
    });

    // Then: the argument is computed once, though read twice
    assert_eq!(html(&dom), "foo bar");
    assert_eq!(calls.get(), 1);
}

#[test]
fn snippet_argument_multiple() {
    // Given: adapted, the parameter `doubled` is `twice`: a parameter
    // may not shadow a script binding in a .wisp
    let (dom, _c) =
        mounted(|dom, root| ArgumentMultiple::mount(dom, root, ArgumentMultipleProps::default()));
    assert_eq!(
        html(&dom),
        "<p>clicks: 0, doubled: 0</p><button>click me</button>"
    );

    // When
    RecordingDom::click(&dom, element(&dom, "button", 0));

    // Then
    assert_eq!(
        html(&dom),
        "<p>clicks: 1, doubled: 2</p><button>click me</button>"
    );
}

#[test]
fn snippet_comment_sibling() {
    // When: adapted, without the HTML comment beside the snippet, which a
    // .wisp component tag refuses
    let (dom, _c) =
        mounted(|dom, root| CommentSibling::mount(dom, root, CommentSiblingProps::default()));

    // Then
    assert_eq!(html(&dom), "The content");
}

#[test]
fn snippet_hoisting_2() {
    // When: `b` renders before it is declared, and renders `a`, declared before it
    let (dom, _c) = mounted(|dom, root| Hoisting2::mount(dom, root, Hoisting2Props::default()));

    // Then
    assert_eq!(html(&dom), "a");
}
