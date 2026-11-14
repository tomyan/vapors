//! A scroll container keeps its marks box while its thumbs are hidden
//! and shown again, so a touch that toggles them allocates and frees no
//! box per container; a repaint of the box makes it afresh, without one
//! when it has no outline or thumbs (review of #367, M2 and L1).

use vapors_dom::Dom;

use crate::kept_box_size_tests::{VIEWPORT, kept_box, of};

#[test]
fn a_scrollers_marks_stay_boxed_while_its_thumbs_are_hidden_and_shown() {
    // Given: a scroller painted with its thumbs
    let (mut doc, scroller) = kept_box("div", "scroller");
    assert_eq!(of(&doc, scroller).boxed_apart(), (true, false));

    // When: hidden
    doc.set_thumbs_shown(false);
    doc.update(VIEWPORT).expect("updates");

    // Then: kept, no thumb items
    let hidden = of(&doc, scroller);
    assert_eq!(
        (hidden.boxed_apart(), hidden.thumb().items().len()),
        ((true, false), 0)
    );

    // When: shown again
    doc.set_thumbs_shown(true);
    doc.update(VIEWPORT).expect("updates");

    // Then
    let shown = of(&doc, scroller);
    assert_eq!(shown.boxed_apart(), (true, false));
    assert_ne!(shown.thumb().items().len(), 0);
}

#[test]
fn a_repainted_box_with_its_thumbs_hidden_boxes_nothing_apart() {
    // Given: a scroller whose thumbs are hidden, its box kept
    let (mut doc, scroller) = kept_box("div", "scroller");
    doc.set_thumbs_shown(false);
    doc.update(VIEWPORT).expect("updates");

    // When: its own paint changes, so its box is painted again
    doc.set_attribute(scroller, "style", "background-color: blue");
    doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(of(&doc, scroller).boxed_apart(), (false, false));
}
