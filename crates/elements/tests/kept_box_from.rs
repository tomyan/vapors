//! A kept box's paint copied only while what it was made from holds
//! (IU3 3c-2, `kept_box.rs`): each case changes one value a kept box's
//! paint reads, with the box itself kept, and the update's scene must
//! equal a full paint's, which a debug build checks on every update
//! (`kept_paint_check.rs`). A case for a value in `BoxFrom` fails with
//! that value left out of it. A case for a value not in it fails with
//! `LayoutBox::kept` ignored: what changes the value builds the box
//! again. The text layout case fails without `KeptBox::touch`.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId, PropertyValue};
use vapors_elements::Document;

fn styled(css: &str) -> Document {
    let mut doc = document();
    doc.add_stylesheet("app.css", css).expect("parses");
    doc
}

#[test]
fn an_earlier_child_unmoved_is_clipped_again_when_its_clipping_container_grows() {
    // Given: a clipping container sized by its children
    let mut doc = styled(".clip { overflow: hidden; } .tall { height: 50px; }");
    let body = doc.body();
    let container = element(&mut doc, body, "div", "clip");
    let first = element(&mut doc, container, "div", "");
    text(&mut doc, first, "first");
    let later = element(&mut doc, container, "div", "");
    doc.update(viewport(200.0, 200.0)).expect("updates");

    // When: the later child grows the container
    doc.set_attribute(later, "class", "tall");

    // Then: the debug check compares this update's paint with a full one
    doc.update(viewport(200.0, 200.0)).expect("updates");
}

/// img and svg refuse % padding; a range reads its content box too.
/// Its container is sized by a sibling's text, as a restyle of the
/// container or a viewport change would build the range again.
#[test]
fn a_range_whose_percent_padding_follows_its_container_paints_its_track_again() {
    // Given: a fixed border box whose padding is 10% of its container's
    // width, a flex item's, which follows a label's
    let mut doc = styled(
        "body { display: flex; align-items: flex-start; } \
         input { display: block; box-sizing: border-box; margin: 0; \
         width: 50px; height: 20px; padding-left: 10%; }",
    );
    let body = doc.body();
    let container = element(&mut doc, body, "div", "");
    let label = element(&mut doc, container, "div", "");
    let run = text(&mut doc, label, "a label");
    let range = element(&mut doc, container, "input", "");
    doc.set_attribute(range, "type", "range");
    doc.update(viewport(400.0, 200.0)).expect("updates");

    // When: the label widens the container; the range's rect is unchanged
    doc.set_text(run, "a label twice as wide");
    doc.update(viewport(400.0, 200.0)).expect("updates");
}

/// Not in `BoxFrom`: a text field's text origin.
#[test]
fn a_fields_text_moved_inside_an_unmoved_field_moves_its_text_origin() {
    // Given: a centred field of fixed size
    let mut doc = styled(
        "body { margin: 0; } input { display: block; box-sizing: border-box; \
         width: 160px; height: 24px; text-align: center; }",
    );
    let body = doc.body();
    let field = element(&mut doc, body, "input", "");
    doc.set_property(field, "value", PropertyValue::Text("Warm".into()));
    doc.update(viewport(200.0, 200.0)).expect("updates");

    // When: a longer value moves the centred text left
    doc.set_property(field, "value", PropertyValue::Text("Warmer day".into()));
    doc.update(viewport(200.0, 200.0)).expect("updates");
}

/// Not in `BoxFrom`: whether its background became the canvas's.
#[test]
fn html_given_a_background_takes_the_canvas_from_body() {
    // Given: body's background is the canvas's
    let mut doc = styled("body { background-color: blue; } .red { background-color: red; }");
    let body = doc.body();
    text(&mut doc, body, "page");
    doc.update(viewport(200.0, 200.0)).expect("updates");

    // When: html's background takes the canvas, so body paints its own
    let root = doc.root();
    doc.set_attribute(root, "class", "red");
    doc.update(viewport(200.0, 200.0)).expect("updates");
}

/// A spacer, then A and B, each a div with text.
fn two_runs() -> (Document, NodeId, NodeId) {
    let mut doc = styled(".tall { height: 50px; }");
    let body = doc.body();
    let spacer = element(&mut doc, body, "div", "");
    let a = element(&mut doc, body, "div", "");
    let run = text(&mut doc, a, "a");
    let b = element(&mut doc, body, "div", "");
    text(&mut doc, b, "b");
    doc.update(viewport(200.0, 200.0)).expect("updates");
    (doc, spacer, run)
}

#[test]
fn a_box_copied_over_two_updates_keeps_its_text_layout_for_a_later_move() {
    // Given: B's paint copied while A's text changes, twice
    let (mut doc, spacer, run) = two_runs();
    for edit in ["a1", "a2"] {
        doc.set_text(run, edit);
        doc.update(viewport(200.0, 200.0)).expect("updates");
    }

    // When: the spacer grows, moving A and B without changing their widths
    doc.set_attribute(spacer, "class", "tall");
    doc.update(viewport(200.0, 200.0)).expect("updates");

    // Then: both are painted from the layouts the text cache kept
    assert_eq!(doc.update_counts().boxes_painted, 7);
    assert_eq!(doc.text_layouts_built(), 0);
}
