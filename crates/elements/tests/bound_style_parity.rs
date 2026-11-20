//! One question, one answer (decision 2026-11-20-style-bindings, 9):
//! `RecordingDom` and `Document` take the same shared cases, accept or
//! refuse each the same way and keep the same `style` attribute.

use crate::support::{document, viewport};
use vapors_dom::BoundProperty as P;
use vapors_dom::recording::RecordingDom;
use vapors_dom::{BoundValue as V, Dom, NodeId, Rgba, StyleKey, Winners};

static KEY: StyleKey = StyleKey {
    site: "crates/x/parity.wisp:2:3",
    scope: None,
    tag: "div",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: Some("overflow: visible"),
    bound: &[P::Height, P::Top, P::BackgroundColor],
    winners: Winners::Unanalysed,
};

const RED: V = V::Color(Rgba::rgb(255, 0, 0));

/// One call on a keyed element, and whether both trees must accept it.
type Call = (P, V, bool);

/// Named sequences of calls.
const CASES: &[(&str, &[Call])] = &[
    ("a good length", &[(P::Height, V::Length(4.0), true)]),
    ("a colour", &[(P::BackgroundColor, RED, true)]),
    ("a negative top", &[(P::Top, V::Length(-3.0), true)]),
    ("a zero", &[(P::Height, V::Length(-0.0), true)]),
    (
        "a repeat then a change",
        &[
            (P::Height, V::Length(4.0), true),
            (P::Height, V::Length(4.0), true),
            (P::Height, V::Length(5.5), true),
        ],
    ),
    ("an unlisted", &[(P::Width, V::Length(4.0), false)]),
    ("a negative height", &[(P::Height, V::Length(-1.0), false)]),
    ("a NaN", &[(P::Top, V::Length(f32::NAN), false)]),
    ("an infinity", &[(P::Top, V::Length(f32::INFINITY), false)]),
    ("a colour for a length", &[(P::Height, RED, false)]),
    (
        "a length for a colour",
        &[(P::BackgroundColor, V::Length(1.0), false)],
    ),
    (
        "a refusal after good values",
        &[
            (P::Height, V::Length(8.0), true),
            (P::BackgroundColor, RED, true),
            (P::Height, V::Length(-8.0), false),
        ],
    ),
];

/// The `style` after the calls, and whether each was accepted.
fn on_recording(calls: &[Call]) -> (Option<String>, bool) {
    let mut dom = RecordingDom::new();
    dom.expect_refusals();
    let node = dom.create_keyed_element(&KEY);
    for (property, value, _) in calls {
        dom.bind_style(node, *property, *value);
    }
    let refused = !dom.refusals().is_empty();
    (dom.attribute(node, "style"), refused)
}

fn on_document(calls: &[Call]) -> (Option<String>, bool) {
    let mut doc = document();
    let node: NodeId = doc.create_keyed_element(&KEY);
    doc.append(doc.body(), node);
    for (property, value, _) in calls {
        doc.bind_style(node, *property, *value);
    }
    let refused = doc.update(viewport(100.0, 100.0)).is_err();
    (doc.attribute(node, "style").map(str::to_owned), refused)
}

#[test]
fn both_trees_accept_and_refuse_every_shared_case_alike_with_the_same_style() {
    for (name, calls) in CASES {
        let (recorded, document) = (on_recording(calls), on_document(calls));
        let refused = calls.iter().any(|(_, _, accepted)| !accepted);
        assert_eq!(recorded.1, refused, "{name}: RecordingDom");
        assert_eq!(document.1, refused, "{name}: Document");
        assert_eq!(recorded.0, document.0, "{name}: the style attribute");
    }
}

#[test]
fn both_trees_refuse_an_unkeyed_element_alike_and_leave_it_unstyled() {
    let mut rec = RecordingDom::new();
    rec.expect_refusals();
    let plain = rec.create_element("div");
    rec.bind_style(plain, P::Height, V::Length(4.0));
    let mut doc = document();
    let plain_doc = doc.create_element("div");
    doc.append(doc.body(), plain_doc);
    doc.bind_style(plain_doc, P::Height, V::Length(4.0));
    let why = rec.refusals().join("");
    assert!(
        why.contains("was not created by create_keyed_element"),
        "{why}"
    );
    assert!(doc.update(viewport(100.0, 100.0)).is_err());
    assert_eq!(rec.attribute(plain, "style"), None);
    assert_eq!(doc.attribute(plain_doc, "style"), None);
}
