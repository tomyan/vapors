//! Keyed children of keyed parents share a computed style only under
//! one parent style (analysis PR 2): the parent's kept style is part of
//! the share key, so what a child inherits, and what a selector reads
//! from its ancestors, splits the share when the parents differ.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, viewport};
use vapors_dom::{Dom, NodeId, StyleKey};
use vapors_elements::Document;

const SCOPE: &str = "tests/style_sharing_parents.wisp";

static PARENT: StyleKey = StyleKey {
    site: "tests/style_sharing_parents.wisp:1",
    scope: Some(SCOPE),
    tag: "div",
    classes: &["p"],
    toggles: &["on"],
    style: None,
};

static CHILD: StyleKey = StyleKey {
    site: "tests/style_sharing_parents.wisp:2",
    scope: Some(SCOPE),
    tag: "span",
    classes: &["c"],
    toggles: &[],
    style: None,
};

static BUTTON: StyleKey = StyleKey {
    site: "tests/style_sharing_parents.wisp:3",
    scope: Some(SCOPE),
    tag: "button",
    classes: &["b"],
    toggles: &[],
    style: None,
};

/// A document with `css` and `count` keyed parents under the body, each
/// with the keyed children `children` lists; the parents and, per
/// parent, its children.
fn page(
    css: &str,
    count: usize,
    children: &[&'static StyleKey],
) -> (Document, Vec<NodeId>, Vec<Vec<NodeId>>) {
    let mut doc = document();
    doc.add_style(SCOPE, css);
    let body = doc.body();
    let (mut parents, mut kids) = (Vec::new(), Vec::new());
    for _ in 0..count {
        let parent = doc.create_keyed_element(&PARENT);
        doc.append(body, parent);
        kids.push(
            children
                .iter()
                .map(|&key| keyed_child(&mut doc, parent, key))
                .collect(),
        );
        parents.push(parent);
    }
    (doc, parents, kids)
}

fn keyed_child(doc: &mut Document, parent: NodeId, key: &'static StyleKey) -> NodeId {
    let child = doc.create_keyed_element(key);
    doc.append(parent, child);
    child
}

fn update(doc: &mut Document) {
    doc.update(viewport(200.0, 300.0)).expect("updates");
}

fn property(doc: &Document, node: NodeId, name: &str) -> String {
    doc.computed_style(node)
        .expect("styled")
        .get_by_name(name)
        .expect("a longhand")
        .to_css()
}

#[test]
fn children_of_a_parent_toggled_later_take_its_selectors_style_and_lose_it_again() {
    // Given
    let css = ".p.on > .c { color: rgb(9 9 9); } .c { color: rgb(1 1 1); }";
    let (mut doc, parents, kids) = page(css, 3, &[&CHILD]);
    update(&mut doc);

    // When
    doc.toggle_classes(parents[1], &[("on", true)]);
    update(&mut doc);
    doc.toggle_classes(parents[1], &[("on", false)]);
    doc.toggle_classes(parents[2], &[("on", true)]);
    update(&mut doc);

    // Then
    let colors: Vec<String> = kids.iter().map(|k| property(&doc, k[0], "color")).collect();
    assert_eq!(colors, ["rgb(1 1 1)", "rgb(1 1 1)", "rgb(9 9 9)"]);
}

#[test]
fn children_inherit_custom_properties_and_font_size_from_their_own_parent() {
    // Given
    let css = ".p { --k: rgb(1 1 1); font-size: 10px; } .p.on { --k: rgb(9 9 9); font-size: 20px; } \
               .c { display: block; color: var(--k); padding-left: 1em; }";
    let (mut doc, parents, kids) = page(css, 2, &[&CHILD]);
    doc.toggle_classes(parents[0], &[("on", true)]);

    // When
    update(&mut doc);

    // Then
    let (on, off) = (kids[0][0], kids[1][0]);
    assert_eq!(property(&doc, on, "color"), "rgb(9 9 9)");
    assert_eq!(property(&doc, off, "color"), "rgb(1 1 1)");
    assert_ne!(
        property(&doc, on, "padding-left"),
        property(&doc, off, "padding-left")
    );
}

#[test]
fn a_state_on_an_ancestor_splits_the_children_its_selector_reaches() {
    // Given: two parents, each with a button and a child
    let css = ".p { display: block; height: 30px; } .b { display: block; width: 20px; height: 20px; } \
               .p:focus-within .c { color: rgb(9 9 9); }";
    let (mut doc, _, kids) = page(css, 2, &[&BUTTON, &CHILD]);
    update(&mut doc);

    // When: the second parent's button takes focus
    let doc = Rc::new(RefCell::new(doc));
    Document::focus(&doc, Some(kids[1][0]));
    let mut doc = Rc::try_unwrap(doc).ok().expect("one owner").into_inner();
    update(&mut doc);

    // Then
    assert_eq!(property(&doc, kids[1][1], "color"), "rgb(9 9 9)");
    assert_ne!(property(&doc, kids[0][1], "color"), "rgb(9 9 9)");
}
