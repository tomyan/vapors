//! Svelte's `snippet-*` samples that render a snippet held in a value
//! (1-R1, R1d), ported under `src/svelte_snippets/`; see
//! `svelte_snippets.rs` for the conventions.

#[path = "svelte/mount.rs"]
mod mount;

use std::fmt::Write;

use mount::{element, html, mounted};
use vapors_compiler_examples::svelte_snippets::{
    expressions::{Expressions, ExpressionsProps},
    prop_reactive::{PropReactive, PropReactiveProps},
};
use vapors_dom::recording::RecordingDom;

/// `<p>{name}</p><hr></hr>` for each name, then `rest` (the recording
/// Dom writes every element with its end tag).
fn lines(names: &[&str], rest: &str) -> String {
    let mut html = names.iter().fold(String::new(), |mut html, name| {
        let _ = write!(html, "<p>{name}</p><hr></hr>");
        html
    });
    html.push_str(rest);
    html
}

#[test]
fn snippet_expressions() {
    // Given: adapted, Svelte's object of snippets is a HashMap prop,
    // `?.` is `.get()` (an Option), `??` is `or_else`, and `optional`
    // is a memo the parent builds in the attribute (its script cannot
    // see template snippets)
    let (dom, _c) = mounted(|dom, root| Expressions::mount(dom, root, ExpressionsProps::default()));
    let foo = ["foo"; 6];
    assert_eq!(
        html(&dom),
        lines(
            &[&foo[..], &["bar"]].concat(),
            "<hr></hr><button>toggle</button>"
        )
    );

    // When
    RecordingDom::click(&dom, element(&dom, "button", 0));

    // Then
    let after = [&["bar", "bar"][..], &["foo"; 6][..]].concat();
    assert_eq!(
        html(&dom),
        lines(&after, "<p>foo</p><button>toggle</button>")
    );
}

#[test]
fn snippet_prop_reactive() {
    // Given: adapted, the prop is a memo choosing between the parent's
    // snippets, as a .wisp prop is reactive only as a signal or memo
    let (dom, _c) =
        mounted(|dom, root| PropReactive::mount(dom, root, PropReactiveProps::default()));
    assert_eq!(html(&dom), "<p>foo</p><button>show bar</button>");

    // When
    RecordingDom::click(&dom, element(&dom, "button", 0));

    // Then
    assert_eq!(html(&dom), "<p>bar</p><button>show bar</button>");
}
