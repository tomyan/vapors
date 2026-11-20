//! `RecordingDom::bind_style` records what `Document` accepts and
//! refuses what it refuses, keeping the `style` attribute `Document`
//! keeps (decision 2026-11-20-style-bindings; the shared cases are in
//! the elements crate's `bound_style_parity`).

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{BoundProperty as P, BoundValue, Dom, Rgba, StyleKey, Winners};

static BAR: StyleKey = StyleKey {
    site: "crates/x/bars.wisp:3:5",
    scope: None,
    tag: "div",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: Some("color: red"),
    bound: &[P::Height, P::BackgroundColor],
    winners: Winners::Unanalysed,
};

const H: BoundValue = BoundValue::Length(4.0);

fn has_bound(dom: &RecordingDom) -> bool {
    (dom.calls().iter()).any(|c| matches!(c, Call::BindStyle { .. }))
}

#[test]
fn a_listed_property_with_a_good_value_is_recorded_and_written_to_style() {
    let mut dom = RecordingDom::new();
    let node = dom.create_keyed_element(&BAR);
    dom.bind_style(node, P::Height, H);
    let call = Call::BindStyle {
        node,
        property: P::Height,
        value: H,
    };
    assert_eq!(dom.calls().last(), Some(&call));
    assert_eq!(dom.refusals(), Vec::<String>::new());
    let style = dom.attribute(node, "style");
    assert_eq!(style.as_deref(), Some("color: red; height: 4px"));
}

#[test]
fn style_lists_the_declarations_in_property_order_and_a_new_value_replaces() {
    let mut dom = RecordingDom::new();
    let node = dom.create_keyed_element(&BAR);
    let red = BoundValue::Color(Rgba::rgb(255, 0, 0));
    dom.bind_style(node, P::BackgroundColor, red);
    dom.bind_style(node, P::Height, H);
    dom.bind_style(node, P::Height, BoundValue::Length(9.0));
    let style = dom.attribute(node, "style").expect("style");
    assert_eq!(
        style,
        "color: red; height: 9px; background-color: #ff0000ff"
    );
}

#[test]
fn each_refusal_is_given_nothing_is_recorded_and_the_last_valid_value_stays() {
    let bad = [
        (P::Width, H, "not bound in this element's template"),
        (P::Height, BoundValue::Length(-1.0), "outside its domain"),
        (
            P::Height,
            BoundValue::Length(f32::NAN),
            "outside its domain",
        ),
        (
            P::Height,
            BoundValue::Color(Rgba::rgb(1, 2, 3)),
            "takes a px f32",
        ),
    ];
    for (property, value, expected) in bad {
        let mut dom = RecordingDom::new();
        dom.expect_refusals();
        let node = dom.create_keyed_element(&BAR);
        dom.bind_style(node, P::Height, H);
        let before = dom.calls().len();
        dom.bind_style(node, property, value);
        assert!(has_bound(&dom));
        assert_eq!(dom.calls().len(), before + 1, "only the refusal is added");
        assert!(dom.refusals().join("\n").contains(expected), "{expected}");
        let style = dom.attribute(node, "style");
        assert_eq!(style.as_deref(), Some("color: red; height: 4px"));
    }
}

#[test]
fn an_unkeyed_element_and_a_text_node_are_refused() {
    let mut dom = RecordingDom::new();
    dom.expect_refusals();
    let plain = dom.create_element("div");
    let text = dom.create_text("hi");
    for node in [plain, text] {
        dom.bind_style(node, P::Height, H);
    }
    assert!(!has_bound(&dom));
    let why = dom.refusals();
    assert_eq!(why.len(), 2);
    assert!(why[0].contains("<div> was not created by create_keyed_element"));
    assert!(why[1].contains("a text node was not created by create_keyed_element"));
    assert_eq!(dom.attribute(plain, "style"), None);
}
