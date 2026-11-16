//! Rows a lane start moved, in the mirror, on the shapes that reach them
//! by other paths (#434's review, H1, M1 and M2): each update's take held
//! against a first take (`Model::assert_mirrors`), on rows that are not
//! nodes (a div holding a label, a value and a button, vapors-perf's rows
//! page): an edit inside a later row in the same update as an insert
//! above; a removal in the middle; two updates before one take, the
//! second removing the first row the first update moved; the lane gone
//! before the take; lanes inside rows of a lane; and a node hoisted out
//! of a node under a moved row.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror_model::Model;
use support::rows_page::{Edit, RowsPage, row_before, rows_page, rows_viewport};
use support::{document, element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// `count` rows, taken: the page and the mirror's model.
fn taken(count: usize) -> (RowsPage, Model) {
    let page = rows_page(count);
    let model = first_take(&page.doc);
    assert!(page.doc.borrow().update_counts().lanes_framed > 0);
    (page, model)
}

/// The first take of `doc`, as the mirror's model.
fn first_take(doc: &Rc<RefCell<Document>>) -> Model {
    let first = doc.borrow_mut().take_ax_update().expect("updated");
    Model::from_every(&first.expect("every"))
}

/// Updates `doc`, takes, applies the take to `model` and holds it against
/// a first take.
fn update_and_take(doc: &Rc<RefCell<Document>>, model: &mut Model) {
    doc.borrow_mut().update(rows_viewport()).expect("updates");
    take(doc, model);
}

/// Takes, applies the take to `model` and holds it against a first take.
fn take(doc: &Rc<RefCell<Document>>, model: &mut Model) {
    let update = doc.borrow_mut().take_ax_update().expect("updated");
    if let Some(update) = &update {
        model.apply(update);
    }
    model.assert_mirrors(&doc.borrow());
}

/// A row put in at the top of `page`.
fn insert_at_top(page: &RowsPage) {
    let mut doc = page.doc.borrow_mut();
    row_before(&mut doc, 99, Some(page.rows[0].element));
}

#[test]
fn an_edit_inside_a_later_row_with_an_insert_above_keeps_every_bound() {
    // Given
    let (page, mut model) = taken(40);

    // When: one update puts a row in at the top and edits the middle
    // row's value text, the same length
    insert_at_top(&page);
    page.edit(Edit::FixedSizeText);

    // Then
    update_and_take(&page.doc, &mut model);
}

#[test]
fn a_removal_in_the_middle_keeps_every_bound() {
    // Given
    let (page, mut model) = taken(40);

    // When
    page.doc.borrow_mut().remove(page.rows[20].element);

    // Then
    update_and_take(&page.doc, &mut model);
}

#[test]
fn two_updates_before_a_take_the_second_removing_the_first_moved_row_keep_every_bound() {
    // Given
    let (page, mut model) = taken(40);

    // When: two rows put in at the top, updated, then the row after
    // them, the first the inserts moved, removed and updated (a row down
    // from where it began), then one take
    insert_at_top(&page);
    insert_at_top(&page);
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    page.doc.borrow_mut().remove(page.rows[0].element);

    // Then
    update_and_take(&page.doc, &mut model);
}

#[test]
fn a_lane_gone_before_the_take_keeps_every_bound() {
    // Given
    let (page, mut model) = taken(40);

    // When: an insert at the top, updated, then rows of a fractional
    // height, which no lane frames, then one take
    insert_at_top(&page);
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    page.doc
        .borrow_mut()
        .add_stylesheet("fraction.css", ".row { height: 19.2px; }")
        .expect("parses");
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    assert_eq!(page.doc.borrow().update_counts().lanes_framed, 0);

    // Then
    take(&page.doc, &mut model);
}

/// A list of `groups` divs, each holding `rows` rows, a block div with
/// no role holding a button: lanes inside the rows of a lane, as
/// `lane_frames_moved.rs` builds them. The groups and their rows.
/// A group and its rows.
type Group = (NodeId, Vec<NodeId>);

fn nested(groups: usize, rows: usize) -> (Rc<RefCell<Document>>, Vec<Group>) {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    let made = (0..groups)
        .map(|_| {
            let group = element(&mut doc, list, "div", "group");
            let inner = (0..rows)
                .map(|_| nested_row(&mut doc, group, None))
                .collect();
            (group, inner)
        })
        .collect();
    let css = "body { margin: 0; } .row { height: 24px; } \
               button { display: block; width: 60px; height: 20px; }";
    doc.add_stylesheet("nested.css", css).expect("parses");
    doc.update(rows_viewport()).expect("updates");
    (Rc::new(RefCell::new(doc)), made)
}

/// A row holding a button, put in `group` before `before`.
fn nested_row(doc: &mut Document, group: NodeId, before: Option<NodeId>) -> NodeId {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(group, row, before);
    let button = element(doc, row, "button", "");
    text(doc, button, "Edit");
    row
}

#[test]
fn lanes_inside_the_rows_of_a_lane_keep_every_bound() {
    // Given: 40 groups of 40 rows
    let (doc, made) = nested(40, 40);
    let framed = doc.borrow().update_counts().lanes_framed;
    assert!(framed > 1, "the list's lane and the groups': {framed}");
    let mut model = first_take(&doc);

    // When: a row put in at the top of the second group, and one removed
    // from the third, in one update
    {
        let mut doc = doc.borrow_mut();
        nested_row(&mut doc, made[1].0, Some(made[1].1[0]));
        doc.remove(made[2].1[5]);
    }

    // Then
    update_and_take(&doc, &mut model);
}

/// 40 rows, each positioned, with role `role` if given, holding a group
/// that holds an absolutely positioned button: the button's containing
/// block is the row, so it is hoisted out of the group. A row put in at
/// the top, the lane framed after it, taken and held against a first take.
fn hoisted_under_moved_rows(role: Option<&str>) {
    let mut doc = document();
    let body = doc.body();
    let rows: Vec<NodeId> = (0..40)
        .map(|_| hoisting_row(&mut doc, (body, None), role))
        .collect();
    let css = "body { margin: 0; } .row { position: relative; height: 24px; } \
               .floating { display: block; position: absolute; left: 100px; top: 2px; }";
    doc.add_stylesheet("rows.css", css).expect("parses");
    doc.update(rows_viewport()).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    assert!(doc.borrow().update_counts().lanes_framed > 0);
    let mut model = first_take(&doc);
    hoisting_row(&mut doc.borrow_mut(), (body, Some(rows[0])), role);
    doc.borrow_mut().update(rows_viewport()).expect("updates");
    assert!(
        doc.borrow().update_counts().lanes_framed > 0,
        "still framed"
    );
    take(&doc, &mut model);
}

#[test]
fn a_node_hoisted_out_of_a_node_under_a_moved_row_keeps_its_bounds() {
    hoisted_under_moved_rows(None);
}

#[test]
fn a_node_hoisted_out_of_a_node_under_a_moved_row_that_is_a_node_keeps_its_bounds() {
    hoisted_under_moved_rows(Some("listitem"));
}

/// A positioned row, with role `role` if given, holding a group that
/// holds a floating button, put in `parent` before `before`.
fn hoisting_row(
    doc: &mut Document,
    (parent, before): (NodeId, Option<NodeId>),
    role: Option<&str>,
) -> NodeId {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    if let Some(role) = role {
        doc.set_attribute(row, "role", role);
    }
    doc.insert_before(parent, row, before);
    let group = element(doc, row, "div", "");
    doc.set_attribute(group, "role", "group");
    doc.set_attribute(group, "aria-label", "Actions");
    let button = element(doc, group, "button", "floating");
    text(doc, button, "Edit");
    row
}
