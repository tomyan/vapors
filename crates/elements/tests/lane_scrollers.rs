//! Scroll containers in a framed lane's rows (`docs/plans/splice-layout.md`,
//! section 11.2, "Scroll frames under a lane frame", 4e): a lane with a
//! scroller in a row is framed, however the scroller got there; an
//! insert above the row moves no row and leaves the scroller's frame
//! where it shows; a scroll inside a row, nested or not, moves that
//! frame alone. Each case matches a full update and draws as the whole
//! scene does.

mod support;

use support::lane_page::{HeaderList, assert_drawn_afresh, header_list};
use support::narrow_list::ROWS;
use support::viewport;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Viewport;

/// The header a whole 10px; scrollers 20px tall over 60px of content,
/// and a nested scroller of 10px over 40px.
const CSS: &str = "body { margin: 0; } .head { height: 10px; } \
    .scroll { overflow: auto; height: 20px; width: 40px; } \
    .tall { height: 60px; } \
    .inner { overflow: auto; height: 10px; } .inner > .tall { height: 40px; }";

fn view() -> Viewport {
    viewport(400.0, 300.0)
}

/// A header list of 100 rows, each row's cell a scroller over a tall
/// child; with `nested`, a scroller inside each one, over its own.
fn scrolling_rows(nested: bool) -> HeaderList {
    let page = header_list(100, &format!("{ROWS} {CSS}"), view());
    {
        let mut doc = page.doc.borrow_mut();
        for &row in &page.rows {
            let cell = doc.children(row)[0];
            doc.set_attribute(cell, "class", "cell scroll");
            let tall = support::element(&mut doc, cell, "div", "tall");
            if nested {
                let inner = support::element(&mut doc, tall, "div", "inner");
                support::element(&mut doc, inner, "div", "tall");
            }
        }
        doc.update(view()).expect("updates");
    }
    page
}

/// The scroller in row `at`'s cell, or the one nested in it.
fn scroller(page: &HeaderList, at: usize, nested: bool) -> NodeId {
    let doc = page.doc.borrow();
    let cell = doc.children(page.rows[at])[0];
    if !nested {
        return cell;
    }
    let tall = doc.children(cell)[1];
    doc.children(tall)[0]
}

#[test]
fn a_list_with_a_scroller_in_a_row_is_framed() {
    // Given
    let page = header_list(
        100,
        &format!("{ROWS} {CSS} .plain {{ overflow: auto; }}"),
        view(),
    );

    // When: one row's cell made a scroller
    let counts = page.edited(view(), |page| {
        let mut doc = page.doc.borrow_mut();
        let cell = doc.children(page.rows[10])[0];
        doc.set_attribute(cell, "class", "cell plain");
    });

    // Then
    assert_eq!(counts.lane_unframed, None, "{counts:?}");
    assert!(counts.lanes_framed > 0, "{counts:?}");
}

#[test]
fn a_scroller_put_in_a_fixed_size_cell_leaves_the_list_framed() {
    // Given: a cell of a fixed size holding a label and a spacer; a cell
    // given a new child that answers as before is laid out alone
    let css = format!(
        "{ROWS} {CSS} .cell {{ height: 20px; display: flex; flex-direction: column; }} \
         .label, .spacer {{ width: 40px; height: 10px; }} .spacer.plain {{ overflow: auto; }}"
    );
    let page = header_list(100, &css, view());
    let spacer = {
        let mut doc = page.doc.borrow_mut();
        let cell = doc.children(page.rows[10])[0];
        let text = doc.children(cell)[0];
        doc.remove(text);
        let label = support::element(&mut doc, cell, "div", "label");
        support::text(&mut doc, label, "x");
        let spacer = support::element(&mut doc, cell, "div", "spacer");
        doc.update(view()).expect("updates");
        spacer
    };

    // When: the spacer replaced by a scroll container of its size
    let counts = page.edited(view(), |page| {
        let mut doc = page.doc.borrow_mut();
        let cell = doc.children(page.rows[10])[0];
        doc.remove(spacer);
        support::element(&mut doc, cell, "div", "spacer plain");
    });

    // Then: laid out alone, and the list stays framed
    assert_eq!(counts.owners_isolated, 1, "{counts:?}");
    assert_eq!(counts.lane_unframed, None, "{counts:?}");
}

#[test]
fn a_row_inserted_above_scrolling_rows_moves_no_row() {
    // Given
    let page = scrolling_rows(false);

    // When
    let counts = page.edited(view(), |page| _ = page.insert_at(3));

    // Then: framed; the list, body and html grow and no row moves
    assert_eq!(counts.lane_unframed, None, "{counts:?}");
    assert!(counts.boxes_moved <= 3, "{counts:?}");
    assert_drawn_afresh(&page.doc, view(), "after the insert");
}

#[test]
fn a_scroll_in_a_framed_row_moves_that_scroller_alone() {
    // Given: an insert above first, so the row's lane frame has moved
    let page = scrolling_rows(false);
    page.edited(view(), |page| _ = page.insert_at(3));

    // When
    let counts = page.edited(view(), |page| {
        let node = scroller(page, 5, false);
        assert!(page.doc.borrow_mut().scroll_node_by(node, [0.0, 15.0]));
    });

    // Then
    assert_eq!(counts.lane_unframed, None, "{counts:?}");
    assert_drawn_afresh(&page.doc, view(), "after the scroll");
}

#[test]
fn a_scroll_of_a_scroller_nested_in_a_framed_row_moves_it_alone() {
    // Given: each row's scroller holds another
    let page = scrolling_rows(true);
    page.edited(view(), |page| _ = page.insert_at(3));

    // When: the outer one scrolled, then the nested one
    page.edited(view(), |page| {
        let node = scroller(page, 5, false);
        assert!(page.doc.borrow_mut().scroll_node_by(node, [0.0, 5.0]));
    });
    let counts = page.edited(view(), |page| {
        let node = scroller(page, 5, true);
        assert!(page.doc.borrow_mut().scroll_node_by(node, [0.0, 7.0]));
    });

    // Then
    assert_eq!(counts.lane_unframed, None, "{counts:?}");
    assert_drawn_afresh(&page.doc, view(), "after the nested scroll");
}
