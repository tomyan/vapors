//! A framed lane moved along its axis (`docs/plans/splice-layout.md`,
//! section 11.2): no row is placed again, every lane frame shifts, so
//! the draw finds the rows now in view, a box positioned against the
//! list moves with it, and a nested lane's ink is found from the outer
//! lane.

use crate::support;

use support::lane_page::{assert_drawn_afresh, header_list};
use support::narrow_list::{ROWS, VIEW};
use support::oracle::assert_matches_full;
use support::{document, element};
use vapors_dom::Dom;

/// The header 40px tall, or 20px when `short`.
const HEAD: &str = "body { margin: 0; } .head { height: 40px; } .head.short { height: 20px; }";

#[test]
fn a_list_moved_up_draws_the_rows_that_came_into_view() {
    // Given: a framed list below a header, drawn once
    let page = header_list(100, &format!("{ROWS} {HEAD}"), VIEW());
    assert_drawn_afresh(&page.doc, VIEW(), "first draw");

    // When: the header shrinks, so the list moves up and rows below
    // the view come into it; no row moves in its frame
    page.doc
        .borrow_mut()
        .set_attribute(page.head, "class", "head short");

    // Then
    assert_drawn_afresh(&page.doc, VIEW(), "after the list moved");
}

#[test]
fn a_box_positioned_against_a_moved_list_moves_with_it() {
    // Given: a box in a row positioned against the list
    let css = format!(
        "{ROWS} {HEAD} .list {{ position: relative; }} \
         .abs {{ position: absolute; top: 3px; left: 200px; width: 10px; height: 10px; }}"
    );
    let page = header_list(100, &css, VIEW());
    {
        let mut doc = page.doc.borrow_mut();
        let cell = doc.children(page.rows[5])[0];
        element(&mut doc, cell, "div", "abs");
        doc.update(VIEW()).expect("updates");
        // the list's lane declines the row that hoists the box, so it
        // is not framed: a framed lane hoists nothing
        assert_eq!(
            doc.update_counts().lanes_framed,
            0,
            "{:?}",
            doc.update_counts()
        );
    }

    // When: the list moves up
    page.doc
        .borrow_mut()
        .set_attribute(page.head, "class", "head short");

    // Then: placed as a full update places it; were a framed lane to
    // hoist, the walk visits its hoisted boxes when its content moves
    // (`placement_frames.rs` `visit_hoisted_along`)
    assert_matches_full(&page.doc, VIEW(), "after the list moved");
}

#[test]
fn a_nested_lanes_ink_past_its_outer_row_is_drawn() {
    // Given: outer rows above the view, each a framed lane of 1px rows
    // whose shadows reach 40px down, into the view
    let css = "body { margin: 0; } .list { margin-top: -64px; } \
               .inner { height: 1px; box-shadow: 0 40px 0 0 red; }";
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..40 {
        let outer = element(&mut doc, list, "div", "outer");
        for _ in 0..32 {
            element(&mut doc, outer, "div", "inner");
        }
    }
    doc.add_stylesheet("page.css", css).expect("parses");
    let doc = std::rc::Rc::new(std::cell::RefCell::new(doc));

    // Then: the draw holds the shadows of the outer rows above the view
    assert_drawn_afresh(&doc, VIEW(), "first draw");
}
