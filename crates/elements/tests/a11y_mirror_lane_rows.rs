//! Rows a lane start moved, in the mirror: an insert above moves every
//! later row's start, so each row's nodes have new bounds from their
//! parent node, though nothing else about them changed; a row that is not
//! a node itself (a div with no role) has its nearest nodes read again.
//! The mirror kept by takes holds what a first take would
//! (`Model::assert_mirrors`), on the shapes of vapors-perf's rows pages.

use crate::support;

use support::flex_list::{ROW_MARGIN, flex_list, list_viewport};
use support::mirror_model::Model;
use support::rows_page::{RowsPage, row_before, rows_page, rows_viewport};
use vapors_dom::Dom;
use vapors_elements::AxTakeCounts;

/// `count` rows, each a div holding a button, with role `role` on each
/// row if given, the first take taken: the page and the mirror's model.
fn taken(count: usize, role: Option<&str>) -> (RowsPage, Model) {
    let page = rows_page(count);
    if let Some(role) = role {
        let mut doc = page.doc.borrow_mut();
        for row in &page.rows {
            doc.set_attribute(row.element, "role", role);
        }
        doc.update(rows_viewport()).expect("updates");
    }
    let first = page.doc.borrow_mut().take_ax_update().expect("updated");
    let model = Model::from_every(&first.expect("every"));
    assert!(page.doc.borrow().update_counts().lanes_framed > 0);
    (page, model)
}

/// A row put in at the top and taken, the model kept in step and held
/// against a first take; the take's counts.
fn insert_at_top(page: &RowsPage, model: &mut Model) -> AxTakeCounts {
    {
        let mut doc = page.doc.borrow_mut();
        row_before(&mut doc, 99, Some(page.rows[0].element));
        doc.update(rows_viewport()).expect("updates");
        // the rows start elsewhere in a framed lane, not placed again
        assert!(doc.update_counts().lanes_framed > 0, "the lane is framed");
    }
    let update = page.doc.borrow_mut().take_ax_update().expect("updated");
    if let Some(update) = &update {
        model.apply(update);
    }
    model.assert_mirrors(&page.doc.borrow());
    page.doc.borrow().ax_take_counts()
}

#[test]
fn rows_a_lane_start_moved_have_their_buttons_bounds_in_the_mirror() {
    // Given: 40 rows, each a div with no role holding a button
    let (page, mut model) = taken(40, None);

    // When, Then: every button is where the insert put it
    insert_at_top(&page, &mut model);
}

#[test]
fn rows_that_are_nodes_have_their_own_bounds_in_the_mirror() {
    // Given: 40 rows, each a listitem holding a button
    let (page, mut model) = taken(40, Some("listitem"));

    // When, Then: every row is where the insert put it
    insert_at_top(&page, &mut model);
}

#[test]
fn rows_in_a_scrolled_container_have_their_bounds_from_it_in_the_mirror() {
    // Given: 40 rows in a body that scrolls, so each button's parent node
    // is the body, scrolled down by two rows
    let page = rows_page(40);
    {
        let mut doc = page.doc.borrow_mut();
        doc.add_stylesheet(
            "scroll.css",
            "html { overflow: hidden; } body { height: 400px; overflow: auto; }",
        )
        .expect("parses");
        doc.update(rows_viewport()).expect("updates");
    }
    let first = page.doc.borrow_mut().take_ax_update().expect("updated");
    let mut model = Model::from_every(&first.expect("every"));
    {
        let mut doc = page.doc.borrow_mut();
        let body = doc.body();
        assert!(doc.scroll_node_by(body, [0.0, 48.0]), "the body scrolls");
    }

    // When, Then: every button is where the insert put it, from the body
    insert_at_top(&page, &mut model);
}

#[test]
fn flex_column_rows_a_lane_start_moved_are_in_the_mirror() {
    // Given: FlexInsert's page: 40 rows of a flex-column body, each a div
    // with no role holding its text
    let list = flex_list(40, ROW_MARGIN);
    let first = list.doc.borrow_mut().take_ax_update().expect("updated");
    let mut model = Model::from_every(&first.expect("every"));

    // When: a row is put in before the middle one
    list.insert_before_middle();
    list.doc
        .borrow_mut()
        .update(list_viewport())
        .expect("updates");
    let framed = list.doc.borrow().update_counts().lanes_framed;
    assert!(framed > 0, "the list's lane is framed");
    let update = list.doc.borrow_mut().take_ax_update().expect("updated");

    // Then
    if let Some(update) = &update {
        model.apply(update);
    }
    model.assert_mirrors(&list.doc.borrow());
}
