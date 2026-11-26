//! A kept box's slot holds what nearly every box paints, its outline,
//! thumbs and rarer entries boxed apart (property 8: 424 bytes a slot,
//! 48.6 MB of slots at 16,384 rows, most of it empty scenes and lists).

use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::kept_box::KeptBox;
use crate::update::Viewport;

pub(crate) const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 400.0,
    scale: 1.0,
};

const CSS: &str = "div { height: 10px; background-color: red; } \
                   .ringed { outline: 2px solid blue; } \
                   .scroller { overflow: scroll; height: 20px; } \
                   input { display: block; width: 6em; }";

/// An element of `tag` with `class` under the body, updated once, and
/// its kept box.
pub(crate) fn kept_box(tag: &str, class: &str) -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("t.css", CSS).expect("parses");
    let body = doc.body();
    let element = doc.create_element(tag);
    doc.set_attribute(element, "class", class);
    if class == "scroller" {
        let inner = doc.create_element("div");
        doc.set_attribute(inner, "style", "height: 100px");
        doc.append(element, inner);
    }
    doc.append(body, element);
    doc.update(VIEWPORT).expect("updates");
    (doc, element)
}

pub(crate) fn of(doc: &Document, element: NodeId) -> &KeptBox {
    let painted = doc.painted.as_ref().expect("painted");
    painted
        .kept_boxes
        .iter()
        .flatten()
        .find(|kept| kept.own.hits.iter().any(|(node, _)| *node == element))
        .expect("the element's kept box")
}

#[test]
fn a_kept_box_slot_holds_at_most_224_bytes() {
    assert!(
        std::mem::size_of::<Option<KeptBox>>() <= 224,
        "{} bytes",
        std::mem::size_of::<Option<KeptBox>>()
    );
}

#[test]
fn a_plain_box_boxes_nothing_apart() {
    // When
    let (doc, div) = kept_box("div", "");

    // Then
    assert_eq!(of(&doc, div).boxed_apart(), (false, false));
}

#[test]
fn an_outlined_box_keeps_its_outline_apart() {
    // When
    let (doc, div) = kept_box("div", "ringed");

    // Then
    let kept = of(&doc, div);
    assert_eq!(kept.boxed_apart(), (true, false));
    assert_ne!(kept.outline().items().len(), 0);
}

#[test]
fn a_scroll_containers_thumbs_are_kept_apart() {
    // When
    let (doc, div) = kept_box("div", "scroller");

    // Then
    let kept = of(&doc, div);
    assert_eq!(kept.boxed_apart(), (true, false));
    assert_ne!(kept.thumb().items().len(), 0);
}

#[test]
fn a_text_fields_origin_is_kept_apart_on_its_run() {
    // When
    let (doc, input) = kept_box("input", "");

    // Then: the field's own box keeps nothing apart, and one box, its
    // run (its text, which paints its marks and origin), keeps rarer
    // entries apart: the page has no other
    let painted = doc.painted.as_ref().expect("painted");
    let apart = painted
        .kept_boxes
        .iter()
        .flatten()
        .filter(|kept| kept.boxed_apart().1);
    assert_eq!(of(&doc, input).boxed_apart(), (false, false));
    assert_eq!(apart.count(), 1);
}
