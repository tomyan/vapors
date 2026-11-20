//! What `Document::bind_style` refuses (decision 2026-11-20-style-bindings,
//! 8): loudly, naming the element, keeping the last valid value.

use crate::bound_style::{BAR, bars};
use crate::support::{document, viewport};
use vapors_dom::BoundProperty as P;
use vapors_dom::{BoundValue, Dom, Rgba, StyleKey, Winners};
use vapors_elements::Document;

const SITE: &str = "crates/x/bars.wisp:3:5";

fn refusal_of(doc: &mut Document) -> String {
    let Err(errors) = doc.update(viewport(4000.0, 200.0)) else {
        panic!("expected a refusal");
    };
    errors.iter().map(ToString::to_string).collect()
}

fn style_of(doc: &Document, node: vapors_dom::NodeId) -> String {
    doc.attribute(node, "style").unwrap_or("").to_owned()
}

#[test]
fn a_value_outside_the_domain_is_refused_naming_the_element_and_changes_nothing() {
    let cases = [
        (P::Height, BoundValue::Length(-1.0)),
        (P::Height, BoundValue::Length(f32::NAN)),
        (P::Height, BoundValue::Length(f32::INFINITY)),
        (P::Height, BoundValue::Length(f32::NEG_INFINITY)),
    ];
    for (property, value) in cases {
        let (mut doc, nodes) = bars(1, 1);
        let before = style_of(&doc, nodes[0]);
        doc.bind_style(nodes[0], property, value);
        let why = refusal_of(&mut doc);
        assert!(why.contains(SITE), "{why}");
        assert!(why.contains("outside its domain"), "{why}");
        assert!(why.contains("<div"), "names the element: {why}");
        assert_eq!(style_of(&doc, nodes[0]), before, "{value:?}");
    }
}

#[test]
fn a_wrong_kind_and_an_unlisted_property_are_refused_and_change_nothing() {
    let (len, col) = (
        BoundValue::Length(3.0),
        BoundValue::Color(Rgba::rgb(1, 2, 3)),
    );
    let unlisted = "not bound in this element's template";
    for (property, value, expected) in [
        (P::Width, len, unlisted),
        (P::Color, col, unlisted),
        (P::Height, col, "takes a px f32, got an Rgba"),
        (P::BackgroundColor, len, "takes an Rgba, got a px f32"),
    ] {
        let (mut doc, nodes) = bars(1, 1);
        let before = style_of(&doc, nodes[0]);
        doc.bind_style(nodes[0], property, value);
        let why = refusal_of(&mut doc);
        assert!(why.contains(expected) && why.contains(SITE), "{why}");
        assert_eq!(style_of(&doc, nodes[0]), before);
    }
}

#[test]
fn a_refusal_keeps_the_last_valid_value_in_the_cascade() {
    let (mut doc, nodes) = bars(1, 1);
    doc.bind_style(nodes[0], P::Height, BoundValue::Length(21.0));
    doc.bind_style(nodes[0], P::Height, BoundValue::Length(-5.0));
    refusal_of(&mut doc);
    assert_eq!(style_of(&doc, nodes[0]), "height: 21px");
}

#[test]
fn an_unkeyed_element_is_refused_naming_the_fix() {
    let mut doc = document();
    let plain = doc.create_element("div");
    doc.append(doc.body(), plain);
    doc.bind_style(plain, P::Height, BoundValue::Length(4.0));
    let why = refusal_of(&mut doc);
    assert!(
        why.contains("was not created by create_keyed_element"),
        "{why}"
    );
    assert_eq!(doc.attribute(plain, "style"), None);
}

static TWICE: StyleKey = StyleKey {
    bound: &[P::Height, P::Height],
    ..BAR
};

#[test]
fn a_key_listing_a_property_twice_is_refused_at_creation() {
    let mut doc = document();
    let bar = doc.create_keyed_element(&TWICE);
    doc.append(doc.body(), bar);
    let why = refusal_of(&mut doc);
    assert!(why.contains("lists `style:height` twice"), "{why}");
    assert!(why.contains(SITE), "{why}");
}

#[test]
#[should_panic(expected = "bind_style")]
fn a_text_node_panics_naming_the_call_as_every_element_call_does() {
    let mut doc = document();
    let text = doc.create_text("hi");
    doc.bind_style(text, P::Height, BoundValue::Length(4.0));
}

#[test]
#[should_panic(expected = "bind_style")]
fn a_node_the_document_does_not_hold_panics_naming_the_call() {
    let mut other = document();
    let ids: Vec<_> = (0..5).map(|_| other.create_element("div")).collect();
    let mut doc = document();
    doc.bind_style(ids[4], P::Height, BoundValue::Length(4.0));
}

#[test]
fn bound_keys_are_the_only_bindings() {
    static NONE: StyleKey = StyleKey {
        bound: &[],
        winners: Winners::Unanalysed,
        ..BAR
    };
    let mut doc = document();
    let bar = doc.create_keyed_element(&NONE);
    doc.append(doc.body(), bar);
    doc.bind_style(bar, P::Height, BoundValue::Length(4.0));
    assert!(refusal_of(&mut doc).contains("not bound in this element's template"));
}
