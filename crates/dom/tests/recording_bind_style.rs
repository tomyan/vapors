//! `RecordingDom::bind_style` records what `Document` would accept and
//! refuses what it would refuse (decision 2026-11-20-style-bindings).

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{BoundProperty, BoundValue, Dom, Rgba, StyleKey, Winners};

static BAR: StyleKey = StyleKey {
    site: "crates/x/bars.wisp:3:5",
    scope: None,
    tag: "div",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: None,
    bound: &[BoundProperty::Height],
    winners: Winners::Unanalysed,
};

const H: BoundValue = BoundValue::Length(4.0);

fn refusals_of(dom: &mut RecordingDom) -> String {
    dom.refusals().join("\n")
}

fn has_bound(dom: &RecordingDom) -> bool {
    (dom.calls().iter()).any(|c| matches!(c, Call::BindStyle { .. }))
}

#[test]
fn a_listed_property_with_a_good_value_is_recorded() {
    let mut dom = RecordingDom::new();
    let node = dom.create_keyed_element(&BAR);
    dom.bind_style(node, BoundProperty::Height, H);
    let call = Call::BindStyle {
        node,
        property: BoundProperty::Height,
        value: H,
    };
    assert_eq!(dom.calls().last(), Some(&call));
    assert_eq!(refusals_of(&mut dom), "");
}

#[test]
fn each_refusal_document_gives_is_given_and_nothing_is_recorded() {
    let bad = [
        (
            BoundProperty::Width,
            H,
            "not bound in this element's template",
        ),
        (
            BoundProperty::Height,
            BoundValue::Length(-1.0),
            "outside its domain",
        ),
        (
            BoundProperty::Height,
            BoundValue::Color(Rgba::rgb(1, 2, 3)),
            "takes a px f32",
        ),
    ];
    for (property, value, expected) in bad {
        let mut dom = RecordingDom::new();
        dom.expect_refusals();
        let node = dom.create_keyed_element(&BAR);
        dom.bind_style(node, property, value);
        assert!(!has_bound(&dom), "recorded a refused call");
        assert!(refusals_of(&mut dom).contains(expected), "{expected}");
    }
}

#[test]
fn an_unkeyed_element_and_a_text_node_are_refused() {
    let mut dom = RecordingDom::new();
    dom.expect_refusals();
    let plain = dom.create_element("div");
    let text = dom.create_text("hi");
    for node in [plain, text] {
        dom.bind_style(node, BoundProperty::Height, H);
    }
    assert!(!has_bound(&dom));
    let why = dom.refusals();
    assert_eq!(why.len(), 2);
    assert!(
        why.iter()
            .all(|w| w.contains("was not created by create_keyed_element"))
    );
}
