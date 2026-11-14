//! A keyed `{for}` list, as templates compile it (an empty marker text
//! before each row, an empty anchor after the last, new rows built last
//! first), takes the edit on an insert and a remove: empty text makes
//! no box and is passed over (review of #285, M2).

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::narrow_list::{ROWS, VIEW};
use support::oracle::assert_matches_full;
use support::{document, element, text};
use vapors_dom::blocks::keyed;
use vapors_dom::{Dom, NodeId, Root};
use vapors_elements::{Document, UpdateCounts};
use vapors_signals::{Owner, Signal};

type Shared = Rc<RefCell<Document>>;

/// A row, `div.row > div.cell > text`, before `before`.
fn row(
    doc: &Shared,
    parent: NodeId,
    before: Option<NodeId>,
    _index: Signal<usize>,
    item: Signal<u32>,
) -> Vec<Root> {
    let mut doc = doc.borrow_mut();
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", "row");
    doc.insert_before(parent, node, before);
    let cell = element(&mut doc, node, "div", "cell");
    text(&mut doc, cell, &format!("row {}", item.get_untracked()));
    vec![Root::Node(node)]
}

/// A `{for}` over `items` in a list under body, updated once.
fn for_list(items: Signal<Vec<u32>>) -> Shared {
    let doc = Rc::new(RefCell::new(document()));
    let list = {
        let mut doc = doc.borrow_mut();
        let body = doc.body();
        let list = element(&mut doc, body, "div", "list");
        let anchor = doc.create_text("");
        doc.append(list, anchor);
        doc.add_stylesheet("rows.css", ROWS).expect("parses");
        (list, anchor)
    };
    let source = move || items.get().into_iter().map(|item| (item, item)).collect();
    keyed(&doc, list.0, list.1, "test.wisp:1:1", source, row);
    doc.borrow_mut().update(VIEW()).expect("updates");
    doc
}

fn updated(doc: &Shared) -> UpdateCounts {
    let mut doc = doc.borrow_mut();
    doc.update(VIEW()).expect("updates");
    doc.update_counts()
}

#[test]
fn an_insert_into_a_for_list_edits_the_lists_children() {
    // Given
    let owner = Owner::new();
    owner.set();
    let items = Signal::new((0..10).collect::<Vec<u32>>());
    let doc = for_list(items);

    // When: two rows in the middle
    items.set(vec![0, 1, 2, 3, 4, 100, 101, 5, 6, 7, 8, 9]);
    let counts = updated(&doc);

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_eq!(counts.children_edits_declined, 0);
    assert_matches_full(&doc, VIEW(), "for insert");
}

#[test]
fn a_remove_from_a_for_list_edits_the_lists_children() {
    // Given
    let owner = Owner::new();
    owner.set();
    let items = Signal::new((0..10).collect::<Vec<u32>>());
    let doc = for_list(items);

    // When
    items.set(vec![0, 1, 2, 3, 5, 6, 7, 8, 9]);
    let counts = updated(&doc);

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_eq!(counts.children_edits_declined, 0);
    assert_matches_full(&doc, VIEW(), "for remove");
}
