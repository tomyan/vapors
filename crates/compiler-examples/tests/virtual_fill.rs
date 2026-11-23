//! `src/virtual_fill.wisp` (virtual list V4, `docs/plans/virtual-list.md`):
//! vapors' `VirtualList` given no height, embedded in a panel from 30 px
//! below the window's top to 10 px above its bottom, fills the panel and
//! holds the rows of the panel's laid-out height: at the first height,
//! after a resize to a taller one and to a shorter one, at the top, deep
//! in the list and at its end. 100 rows of 20 px, one row of overscan.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::virtual_fill::{VirtualFill, VirtualFillProps};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};
use vapors_signals::Signal;
use vapors_text::FontSystem;

/// Where the panel, and so the list, starts.
const TOP: f32 = 30.0;

struct Mounted {
    doc: Rc<RefCell<Document>>,
    _list: VirtualFill,
}

/// The list mounted and drawn in a window `height` px tall, frames run
/// until the document asks for none.
fn mounted(height: f32) -> Mounted {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let keys = Signal::new((0..100).collect());
    let list = VirtualFill::mount(&doc, body, VirtualFillProps { keys });
    let m = Mounted { doc, _list: list };
    settled(&m, height);
    m
}

/// What a frame does in a window `height` px tall: settle the scrolls
/// and resizes owed, then update.
fn frame(m: &Mounted, height: f32) {
    Document::settle_scroll(&m.doc);
    let viewport = Viewport {
        width: 200.0,
        height,
        scale: 1.0,
    };
    m.doc.borrow_mut().update(viewport).expect("updates");
}

/// Frames in a window `height` px tall until the document asks for no
/// more: the frames an app that redraws on change draws. Refused past
/// three, more than a resize needs (the frame that lays it out, the one
/// that fires its `resize`).
fn settled(m: &Mounted, height: f32) {
    for _ in 0..3 {
        frame(m, height);
        if !m.doc.borrow_mut().take_changed() {
            return;
        }
    }
    panic!("the list still asks for a frame after three at {height} px");
}

/// The elements one of whose classes is `class`.
fn by_class(doc: &Document, class: &str) -> Vec<NodeId> {
    let has = |node: &NodeId| {
        doc.attribute(*node, "class")
            .is_some_and(|names| names.split(' ').any(|name| name == class))
    };
    doc.elements().into_iter().filter(has).collect()
}

fn label(doc: &Document, row: NodeId) -> String {
    let text = doc.children(row).first().copied().expect("a text child");
    doc.text(text).expect("text").to_owned()
}

/// The labels of every row element that exists, in document order.
fn existing(m: &Mounted) -> Vec<usize> {
    let doc = m.doc.borrow();
    let rows = by_class(&doc, "row").into_iter();
    rows.map(|row| label(&doc, row).parse().expect("a number"))
        .collect()
}

/// The label of the row under window `y`.
fn shown_at(m: &Mounted, y: f32) -> String {
    let doc = m.doc.borrow();
    let node = doc.painted().expect("updated").hit([10.0, y]);
    let node = node.unwrap_or_else(|| panic!("nothing under y {y}"));
    assert_eq!(doc.attribute(node, "class"), Some("row"), "a row at y {y}");
    label(&doc, node)
}

fn scroll_to(m: &Mounted, y: f32, height: f32) {
    let list = by_class(&m.doc.borrow(), "virtual-list")[0];
    m.doc.borrow_mut().scroll_to(list, [0.0, y]);
    settled(m, height);
}

#[test]
fn the_list_fills_the_panel_with_its_rows_and_one_below() {
    // When: a 200 px window, a 160 px panel
    let m = mounted(200.0);

    // Then: rows 0..8 fill it, row 8 is the overscan
    assert_eq!(shown_at(&m, TOP), "0");
    assert_eq!(shown_at(&m, 189.0), "7");
    assert_eq!(existing(&m), (0..9).collect::<Vec<_>>());
}

#[test]
fn a_taller_window_shows_the_rows_down_to_the_panels_new_bottom() {
    // Given
    let m = mounted(200.0);

    // When: 300 px, a 260 px panel
    settled(&m, 300.0);

    // Then
    assert_eq!(shown_at(&m, 289.0), "12");
    assert_eq!(existing(&m), (0..14).collect::<Vec<_>>());
}

#[test]
fn a_shorter_window_holds_only_the_rows_of_the_panels_new_height() {
    // Given
    let m = mounted(200.0);

    // When: 120 px, an 80 px panel
    settled(&m, 120.0);

    // Then
    assert_eq!(shown_at(&m, 109.0), "3");
    assert_eq!(existing(&m), (0..5).collect::<Vec<_>>());
}

#[test]
fn deep_in_the_list_a_taller_window_shows_the_rows_below_the_offset() {
    // Given: 400 px down, row 20 at the top
    let m = mounted(200.0);
    scroll_to(&m, 400.0, 200.0);

    // When
    settled(&m, 300.0);

    // Then: rows 20..33 fill the 260 px panel, 19 and 33 the overscan
    assert_eq!(shown_at(&m, TOP), "20");
    assert_eq!(shown_at(&m, 289.0), "32");
    assert_eq!(existing(&m), (19..34).collect::<Vec<_>>());
}

#[test]
fn after_a_shorter_window_the_last_row_is_reachable_at_the_end() {
    // Given
    let m = mounted(200.0);
    settled(&m, 120.0);

    // When: past the end, which the 80 px list clamps to 2000 - 80 px
    scroll_to(&m, 10_000.0, 120.0);

    // Then
    let list = by_class(&m.doc.borrow(), "virtual-list")[0];
    assert_eq!(m.doc.borrow().scroll_offset(list), Some([0.0, 1920.0]));
    assert_eq!(shown_at(&m, TOP), "96");
    assert_eq!(shown_at(&m, 109.0), "99");
}

#[test]
fn at_the_end_a_taller_window_keeps_the_last_row_at_the_bottom() {
    // Given: at the end of the 160 px list, 1840 px down
    let m = mounted(200.0);
    scroll_to(&m, 10_000.0, 200.0);

    // When: 300 px, which clamps the offset to 2000 - 260 px
    settled(&m, 300.0);

    // Then
    assert_eq!(shown_at(&m, TOP), "87");
    assert_eq!(shown_at(&m, 289.0), "99");
}
