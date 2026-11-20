//! `Dom::bind_style` (`style:height={expr}`, decision
//! 2026-11-20-style-bindings): the bound value is the element's own
//! inline declaration, the key and its winners are shared, one change
//! among 1,000 restyles one element and lays out only as the property
//! requires, and a value outside the domain is refused naming the
//! element.

use crate::support;

use support::{document, viewport};
use vapors_dom::{BoundProperty, BoundValue, CompiledSheet, Dom, NodeId, Rgba, StyleKey};
use vapors_elements::{Document, UpdateCounts};

const SCOPE: &str = "bars.wisp";
const CSS: &str = ".bar { width: 2px; height: 5px; background-color: #000000; } .strip { display: flex; align-items: flex-end; height: 100px; width: 4000px; }";

static BAR: StyleKey = StyleKey {
    site: "crates/x/bars.wisp:3:5",
    scope: Some(SCOPE),
    tag: "div",
    classes: &["bar"],
    toggles: &[],
    attributes: &[],
    style: None,
    bound: &[BoundProperty::Height, BoundProperty::BackgroundColor],
    winners: vapors_dom::Winners::Unanalysed,
};

/// A strip of `count` bars, heights cycling through `distinct`.
fn bars(count: usize, distinct: usize) -> (Document, Vec<NodeId>) {
    let mut doc = document();
    doc.add_style(CompiledSheet::for_tests(SCOPE, CSS));
    let strip = doc.create_element("div");
    doc.set_attribute(strip, "class", "strip");
    doc.set_attribute(strip, "data-wisp", SCOPE);
    let body = doc.body();
    doc.append(body, strip);
    let nodes: Vec<NodeId> = (0..count)
        .map(|i| {
            let bar = doc.create_keyed_element(&BAR);
            let height = 10.0 + f32::from(u8::try_from(i % distinct).expect("small"));
            doc.bind_style(bar, BoundProperty::Height, BoundValue::Length(height));
            doc.append(strip, bar);
            bar
        })
        .collect();
    (doc, nodes)
}

fn update(doc: &mut Document) -> UpdateCounts {
    doc.update(viewport(4000.0, 200.0)).expect("updates");
    doc.update_counts()
}

fn computed(doc: &Document, node: NodeId, property: &str) -> String {
    let style = doc.computed_style(node).expect("styled");
    style.get_by_name(property).expect("a longhand").to_css()
}

#[test]
fn the_binding_wins_over_the_sheet() {
    let (mut doc, nodes) = bars(1, 1);
    update(&mut doc);
    assert_eq!(computed(&doc, nodes[0], "height"), "10px");
}

#[test]
fn bars_of_three_distinct_heights_are_three_computed_styles() {
    let (mut doc, _) = bars(1000, 3);
    let counts = update(&mut doc);
    assert_eq!((counts.styles_computed, counts.styles_shared), (3, 997));
}

#[test]
fn one_height_among_a_thousand_restyles_one_element_and_lays_out_as_for_ten() {
    let work = |count| {
        let (mut doc, nodes) = bars(count, 3);
        update(&mut doc);
        doc.bind_style(nodes[1], BoundProperty::Height, BoundValue::Length(60.0));
        let counts = update(&mut doc);
        assert_eq!(computed(&doc, nodes[1], "height"), "60px");
        (counts.elements_restyled, counts.boxes_laid_out)
    };
    let (restyled, laid_out) = work(1000);
    assert_eq!(restyled, 1);
    assert!(laid_out > 0, "a height change lays out");
    assert_eq!(work(10), (restyled, laid_out), "the same work at 10 bars");
}

#[test]
fn one_colour_among_a_thousand_restyles_one_element_and_lays_out_nothing() {
    let (mut doc, nodes) = bars(1000, 3);
    update(&mut doc);
    let red = BoundValue::Color(Rgba::rgb(255, 0, 0));
    doc.bind_style(nodes[7], BoundProperty::BackgroundColor, red);
    let counts = update(&mut doc);
    assert_eq!((counts.elements_restyled, counts.boxes_laid_out), (1, 0));
    assert_eq!(computed(&doc, nodes[7], "background-color"), "rgb(255 0 0)");
}

#[test]
fn the_same_value_changes_nothing() {
    let (mut doc, nodes) = bars(10, 3);
    update(&mut doc);
    doc.bind_style(nodes[0], BoundProperty::Height, BoundValue::Length(10.0));
    assert_eq!(update(&mut doc).elements_restyled, 0);
}

fn refusal_after(property: BoundProperty, value: BoundValue) -> String {
    let (mut doc, nodes) = bars(1, 1);
    doc.bind_style(nodes[0], property, value);
    let Err(errors) = doc.update(viewport(4000.0, 200.0)) else {
        panic!("expected a refusal");
    };
    errors.iter().map(ToString::to_string).collect()
}

#[test]
fn a_negative_or_nan_height_is_refused_naming_the_element() {
    for bad in [-1.0, f32::NAN, f32::INFINITY] {
        let why = refusal_after(BoundProperty::Height, BoundValue::Length(bad));
        assert!(why.contains("crates/x/bars.wisp:3:5"), "{why}");
        assert!(why.contains("outside its domain"), "{why}");
    }
}

#[test]
fn a_property_the_key_does_not_list_is_refused() {
    let why = refusal_after(BoundProperty::Width, BoundValue::Length(3.0));
    assert!(
        why.contains("not bound in this element's template"),
        "{why}"
    );
}

#[test]
fn a_colour_for_a_length_property_is_refused() {
    let why = refusal_after(BoundProperty::Height, BoundValue::Color(Rgba::rgb(1, 2, 3)));
    assert!(why.contains("takes a px f32"), "{why}");
}

#[test]
fn a_length_for_a_colour_property_is_refused() {
    let why = refusal_after(BoundProperty::BackgroundColor, BoundValue::Length(3.0));
    assert!(why.contains("takes an Rgba"), "{why}");
}
