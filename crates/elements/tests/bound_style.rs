//! `Dom::bind_style` (decision 2026-11-20-style-bindings): one change
//! among 1,000 restyles one element and lays out as the property
//! requires; styles are shared per distinct value; a binding wins over
//! the sheet.

use crate::support;

use support::{document, viewport};
use vapors_dom::{BoundProperty, BoundValue, CompiledSheet, Dom, NodeId, Rgba, StyleKey};
use vapors_elements::{Document, UpdateCounts};

const SCOPE: &str = "bars.wisp";
const CSS: &str = ".bar { width: 2px; height: 5px; background-color: #000000; } .strip { display: flex; align-items: flex-end; height: 100px; width: 4000px; }";

pub(crate) static BAR: StyleKey = StyleKey {
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

const RED: BoundValue = BoundValue::Color(Rgba::rgb(255, 0, 0));

/// A document holding an empty strip.
pub(crate) fn strip_document() -> (Document, NodeId) {
    let mut doc = document();
    doc.add_style(CompiledSheet::for_tests(SCOPE, CSS));
    let strip = doc.create_element("div");
    doc.set_attribute(strip, "class", "strip");
    doc.set_attribute(strip, "data-wisp", SCOPE);
    doc.append(doc.body(), strip);
    (doc, strip)
}

/// A strip of `count` bars, heights cycling through `distinct`.
pub(crate) fn bars(count: usize, distinct: usize) -> (Document, Vec<NodeId>) {
    let (mut doc, strip) = strip_document();
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

pub(crate) fn update(doc: &mut Document) -> UpdateCounts {
    doc.update(viewport(4000.0, 200.0)).expect("updates");
    doc.update_counts()
}

pub(crate) fn computed(doc: &Document, node: NodeId, property: &str) -> String {
    let style = doc.computed_style(node).expect("styled");
    style.get_by_name(property).expect("longhand").to_css()
}

#[test]
fn one_height_among_a_thousand_restyles_one_element_and_lays_out_as_for_ten() {
    let work = |count| {
        let (mut doc, nodes) = bars(count, 3);
        let first = update(&mut doc);
        assert_eq!(first.styles_computed, 3, "one style per distinct height");
        doc.bind_style(nodes[1], BoundProperty::Height, BoundValue::Length(60.0));
        let counts = update(&mut doc);
        assert_eq!(computed(&doc, nodes[1], "height"), "60px", "beats sheet");
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
    doc.bind_style(nodes[7], BoundProperty::BackgroundColor, RED);
    let counts = update(&mut doc);
    assert_eq!((counts.elements_restyled, counts.boxes_laid_out), (1, 0));
    assert_eq!(computed(&doc, nodes[7], "background-color"), "rgb(255 0 0)");
}

#[test]
fn the_same_value_changes_nothing_and_a_different_one_restyles() {
    let (mut doc, nodes) = bars(10, 3);
    update(&mut doc);
    doc.bind_style(nodes[0], BoundProperty::Height, BoundValue::Length(10.0));
    let same = update(&mut doc);
    assert_eq!((same.elements_restyled, same.boxes_laid_out), (0, 0));
    doc.bind_style(nodes[0], BoundProperty::Height, BoundValue::Length(11.0));
    assert_eq!(update(&mut doc).elements_restyled, 1, "the check can fail");
}

#[test]
fn bindings_made_in_another_order_share_one_computed_style() {
    use BoundProperty as P;
    let (mut doc, strip) = strip_document();
    let (h, c) = (
        (P::Height, BoundValue::Length(10.0)),
        (P::BackgroundColor, RED),
    );
    for order in [[h, c], [c, h]] {
        let bar = doc.create_keyed_element(&BAR);
        for (property, value) in order {
            doc.bind_style(bar, property, value);
        }
        doc.append(strip, bar);
    }
    let counts = update(&mut doc);
    assert_eq!((counts.styles_computed, counts.styles_shared), (1, 1));
}

#[test]
fn a_binding_beats_the_literal_style_of_the_key_and_the_sheet() {
    static KEYED: StyleKey = StyleKey {
        style: Some("height: 7px; width: 9px"),
        ..BAR
    };
    let (mut doc, strip) = strip_document();
    let bar = doc.create_keyed_element(&KEYED);
    doc.bind_style(bar, BoundProperty::Height, BoundValue::Length(33.0));
    doc.append(strip, bar);
    update(&mut doc);
    assert_eq!(computed(&doc, bar, "height"), "33px");
    assert_eq!(computed(&doc, bar, "width"), "9px", "the literal stays");
}

#[test]
fn a_style_attribute_cannot_replace_the_bound_declarations() {
    let (mut doc, nodes) = bars(1, 1);
    doc.set_attribute(nodes[0], "style", "height: 99px");
    let Err(errors) = doc.update(viewport(4000.0, 200.0)) else {
        panic!("expected a refusal");
    };
    let why: String = errors.iter().map(ToString::to_string).collect();
    assert!(why.contains("`style` on <div>"), "{why}");
    assert!(why.contains("set from its template"), "{why}");
}
