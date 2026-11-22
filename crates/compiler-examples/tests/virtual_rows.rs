//! `src/virtual_rows.wisp` (virtual list V2, `docs/plans/virtual-list.md`):
//! vapors' `VirtualList` over 20 rows of 20 px in a 60 px window with one
//! row of overscan holds only the rows in and next to the window, keyed by
//! the identity its `keys` give, and changes nothing while not scrolled.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::virtual_rows::{VirtualRows, VirtualRowsProps};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};
use vapors_signals::Signal;
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// Window points inside the 60 px list, one in each 20 px band.
const BANDS: [[f32; 2]; 3] = [[10.0, 10.0], [10.0, 30.0], [10.0, 50.0]];

struct Mounted {
    doc: Rc<RefCell<Document>>,
    keys: Signal<Vec<usize>>,
    _rows: VirtualRows,
}

fn mounted() -> Mounted {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let keys = Signal::new((0..20).collect());
    let rows = VirtualRows::mount(&doc, body, VirtualRowsProps { keys });
    frame(&doc);
    doc.borrow_mut().take_changed();
    Mounted {
        doc,
        keys,
        _rows: rows,
    }
}

/// What a frame does: settle the scrolls owed, then update.
fn frame(doc: &Rc<RefCell<Document>>) {
    Document::settle_scroll(doc);
    doc.borrow_mut().update(VIEWPORT).expect("updates");
}

fn by_class(doc: &Document, class: &str) -> Vec<NodeId> {
    let has = |node: &NodeId| doc.attribute(*node, "class") == Some(class);
    doc.elements().into_iter().filter(has).collect()
}

/// The text of a row element.
fn label(doc: &Document, row: NodeId) -> String {
    let text = doc.children(row).first().copied().expect("a text child");
    doc.text(text).expect("text").to_owned()
}

/// The labels of every row element that exists, in document order.
fn existing(m: &Mounted) -> Vec<String> {
    let doc = m.doc.borrow();
    by_class(&doc, "row")
        .into_iter()
        .map(|row| label(&doc, row))
        .collect()
}

/// The row element whose label is `key`.
fn row_of(m: &Mounted, key: &str) -> NodeId {
    let doc = m.doc.borrow();
    let rows = by_class(&doc, "row");
    let found = rows.into_iter().find(|row| label(&doc, *row) == key);
    found.unwrap_or_else(|| panic!("no row {key}"))
}

/// The label of the row under each band.
fn shown(m: &Mounted) -> Vec<String> {
    let doc = m.doc.borrow();
    let painted = doc.painted().expect("updated");
    let hit = |point: &[f32; 2]| {
        let node = painted.hit(*point).expect("something under the band");
        assert_eq!(
            doc.attribute(node, "class"),
            Some("row"),
            "a row under {point:?}"
        );
        label(&doc, node)
    };
    BANDS.iter().map(hit).collect()
}

fn list(m: &Mounted) -> NodeId {
    let doc = m.doc.borrow();
    by_class(&doc, "virtual-list")[0]
}

fn strings(labels: &[&str]) -> Vec<String> {
    labels.iter().map(|label| (*label).to_owned()).collect()
}

#[test]
fn before_a_scroll_the_window_and_one_row_below_exist() {
    // When
    let m = mounted();

    // Then
    assert_eq!(shown(&m), strings(&["0", "1", "2"]));
    assert_eq!(existing(&m), strings(&["0", "1", "2", "3"]));
}

#[test]
fn a_wheel_scroll_shows_and_holds_the_rows_at_the_new_offset() {
    // Given
    let m = mounted();

    // When: 105 px down puts row 5 at the top, 5 px of it above the list
    m.doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, 105.0]);
    frame(&m.doc);

    // Then: rows 5 to 8 touch the window, 4 and 9 are the overscan
    assert_eq!(shown(&m), strings(&["5", "6", "7"]));
    assert_eq!(existing(&m), strings(&["4", "5", "6", "7", "8", "9"]));
}

#[test]
fn the_last_row_is_reachable_at_the_end_of_the_range() {
    // Given
    let m = mounted();
    let list = list(&m);

    // When: past the end, which the container clamps to 340 px
    m.doc.borrow_mut().scroll_to(list, [0.0, 10_000.0]);
    frame(&m.doc);

    // Then
    assert_eq!(m.doc.borrow().scroll_offset(list), Some([0.0, 340.0]));
    assert_eq!(shown(&m), strings(&["17", "18", "19"]));
}

#[test]
fn a_row_kept_across_a_scroll_is_the_same_element() {
    // Given
    let m = mounted();
    let row_two = row_of(&m, "2");

    // When: one row down
    m.doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, 20.0]);
    frame(&m.doc);

    // Then
    assert_eq!(row_of(&m, "2"), row_two);
}

#[test]
fn a_reorder_moves_each_kept_row_element_with_its_key() {
    // Given
    let m = mounted();
    let (zero, one) = (row_of(&m, "0"), row_of(&m, "1"));

    // When: keys 0 and 1 swap places
    m.keys.update(|keys| keys.swap(0, 1));
    frame(&m.doc);

    // Then: the same two elements, in the new order
    assert_eq!(shown(&m), strings(&["1", "0", "2"]));
    assert_eq!((row_of(&m, "0"), row_of(&m, "1")), (zero, one));
}

#[test]
fn rows_whose_keys_leave_the_window_stop_existing() {
    // Given
    let m = mounted();

    // When: the first four keys move to the end
    m.keys.update(|keys| keys.rotate_left(4));
    frame(&m.doc);

    // Then
    assert_eq!(existing(&m), strings(&["4", "5", "6", "7"]));
}

#[test]
fn a_list_not_scrolled_again_keeps_its_rows_and_reports_no_change() {
    // Given: scrolled and drawn
    let m = mounted();
    m.doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, 105.0]);
    frame(&m.doc);
    let before = by_class(&m.doc.borrow(), "row");
    m.doc.borrow_mut().take_changed();

    // When
    frame(&m.doc);

    // Then: the same row elements, and nothing for an app to redraw
    assert_eq!(by_class(&m.doc.borrow(), "row"), before);
    assert!(!m.doc.borrow_mut().take_changed());
}
