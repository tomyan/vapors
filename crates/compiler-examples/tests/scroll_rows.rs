//! `src/scroll_rows.wisp` (virtual list V1, `docs/plans/virtual-list.md`):
//! a list whose `onscroll` picks the first visible row shows the rows at
//! the new offset once the scroll is settled and the document updated,
//! as a frame does, and changes nothing while it is not scrolled.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::scroll_rows::{ScrollRows, ScrollRowsProps};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// Window points inside the 60 px list, one in each 20 px band.
const BANDS: [[f32; 2]; 3] = [[10.0, 10.0], [10.0, 30.0], [10.0, 50.0]];

fn mounted() -> (Rc<RefCell<Document>>, ScrollRows) {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let rows = ScrollRows::mount(&doc, body, ScrollRowsProps::default());
    frame(&doc);
    doc.borrow_mut().take_changed();
    (doc, rows)
}

/// What a frame does: settle the scrolls owed, then update.
fn frame(doc: &Rc<RefCell<Document>>) {
    Document::settle_scroll(doc);
    doc.borrow_mut().update(VIEWPORT).expect("updates");
}

/// The row element under each band, `None` where no row is.
fn rows_hit(doc: &Rc<RefCell<Document>>) -> Vec<Option<NodeId>> {
    let doc = doc.borrow();
    let painted = doc.painted().expect("updated");
    BANDS
        .iter()
        .map(|point| {
            painted
                .hit(*point)
                .filter(|node| doc.attribute(*node, "class") == Some("row"))
        })
        .collect()
}

/// The text of the row under each band.
fn shown(doc: &Rc<RefCell<Document>>) -> Vec<Option<String>> {
    let rows = rows_hit(doc);
    let doc = doc.borrow();
    rows.into_iter()
        .map(|row| {
            let text = doc.children(row?).first().copied()?;
            doc.text(text).map(str::to_owned)
        })
        .collect()
}

fn texts(rows: [&str; 3]) -> Vec<Option<String>> {
    rows.map(|row| Some(row.to_owned())).to_vec()
}

#[test]
fn the_list_shows_its_first_rows_before_a_scroll() {
    // When
    let (doc, _rows) = mounted();

    // Then
    assert_eq!(shown(&doc), texts(["0", "1", "2"]));
}

#[test]
fn a_wheel_scroll_shows_the_rows_at_the_new_offset_in_the_next_frame() {
    // Given
    let (doc, rows) = mounted();

    // When: 105 px down puts row 5 at the top, 5 px of it above the list
    let scrolled = doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, 105.0]);
    frame(&doc);

    // Then
    assert!(scrolled);
    assert_eq!(rows.first.get(), 5);
    assert_eq!(shown(&doc), texts(["5", "6", "7"]));
}

#[test]
fn a_list_not_scrolled_again_keeps_its_rows_and_reports_no_change() {
    // Given: scrolled and drawn
    let (doc, _rows) = mounted();
    doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, 105.0]);
    frame(&doc);
    let before = rows_hit(&doc);
    doc.borrow_mut().take_changed();

    // When
    frame(&doc);

    // Then: the same row elements, and nothing for an app to redraw
    assert_eq!(rows_hit(&doc), before);
    assert!(!doc.borrow_mut().take_changed());
}
