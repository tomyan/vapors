//! A framed lane's ink kept through a splice (`paint_frames.rs`,
//! `paint_lane_ink.rs`): a splice makes the scroll frames afresh and
//! notes ink only for the lane frames it painted, so the ink the kept
//! rows noted before is carried into the frames made afresh, and the
//! draw still finds the shadows of rows above the view.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::lane_page::assert_drawn_afresh;
use support::narrow_list::VIEW;
use support::{document, element};
use vapors_dom::NodeId;
use vapors_elements::Document;

/// A framed list of 1px rows starting above the view, each with a
/// shadow 40px down, into the view; the list.
fn shadowed_rows() -> (Rc<RefCell<Document>>, NodeId) {
    let css = "body { margin: 0; } .list { margin-top: -64px; } \
               .row { height: 1px; } .shadowed { box-shadow: 0 40px 0 0 red; }";
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..200 {
        element(&mut doc, list, "div", "row shadowed");
    }
    doc.add_stylesheet("page.css", css).expect("parses");
    (Rc::new(RefCell::new(doc)), list)
}

#[test]
fn a_row_appended_to_a_lane_keeps_the_ink_of_the_rows_above_the_view() {
    // Given: drawn once, the list's lane framed
    let (doc, list) = shadowed_rows();
    assert_drawn_afresh(&doc, VIEW(), "first draw");
    assert!(doc.borrow().update_counts().lanes_framed > 0);

    // When: a row with no shadow appended, far below the view, so the
    // splice paints no row that notes the shadows' ink
    {
        let mut doc = doc.borrow_mut();
        element(&mut doc, list, "div", "row");
    }

    // Then: spliced, and the draw holds the shadows above the view
    assert_drawn_afresh(&doc, VIEW(), "after the append");
    assert_eq!(doc.borrow().update_counts().order_splices, 1);
}
