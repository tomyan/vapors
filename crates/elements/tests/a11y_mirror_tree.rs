//! The a11y update's placements and removals (R6): a node added is
//! placed before the next node under its parent, one removed or hidden
//! is `removed` with its subtree, and boxes an edit moved emit their new
//! bounds. The placements are proven by a model mirror that applies
//! them in order (`support::mirror::Model`); a node that changes
//! parent or place is `a11y_mirror_reparent.rs`.

mod support;

use support::mirror::{labels, node_of, taken_model, updated_take};
use support::roles_page::button;
use support::text;
use vapors_dom::Dom;

#[test]
fn a_node_inserted_is_placed_before_the_next_node_under_its_parent() {
    // Given
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    let reset = node_of(&doc, "button", "Reset");
    let before = model.children(Some(region));
    let added = {
        let mut doc = doc.borrow_mut();
        let added = doc.create_element("button");
        text(&mut doc, added, "Undo");
        doc.insert_before(region, added, Some(reset));
        added
    };

    // When
    let update = updated_take(&doc).expect("a change");
    model.apply(&update);

    // Then: Reset moved right, so its record is emitted with new bounds
    assert_eq!(
        labels(&update.nodes),
        ["button \"Reset\"", "button \"Undo\""]
    );
    assert_eq!(update.placed.len(), 1);
    assert_eq!(
        (update.placed[0].parent, update.placed[0].before),
        (Some(region), Some(reset))
    );
    let mut expected = before;
    expected.insert(4, added);
    assert_eq!(model.children(Some(region)), expected);
    assert_eq!(update.removed, []);
}

#[test]
fn a_node_appended_last_is_placed_before_nothing() {
    // Given
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    let added = button(&mut doc.borrow_mut(), region, "Undo", &[]);

    // When
    let update = updated_take(&doc).expect("a change");
    model.apply(&update);

    // Then
    assert_eq!(labels(&update.nodes), ["button \"Undo\""]);
    assert_eq!(update.placed[0].before, None);
    assert_eq!(model.children(Some(region)).last(), Some(&added));
}

#[test]
fn a_node_removed_is_removed_and_the_boxes_after_it_move() {
    // Given
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    let apply = node_of(&doc, "button", "Apply");
    doc.borrow_mut().remove(apply);

    // When
    let update = updated_take(&doc).expect("a change");
    model.apply(&update);

    // Then
    assert_eq!(update.removed, [apply]);
    assert_eq!(labels(&update.nodes), ["button \"Reset\""]);
    assert_eq!(update.placed, []);
    assert!(!model.children(Some(region)).contains(&apply));
}

#[test]
fn a_subtree_hidden_by_display_none_is_removed_whole() {
    // Given: the region's row hidden; the unnamed row below it moves up
    let (doc, mut model) = taken_model();
    let dialog = node_of(&doc, "dialog", "Levels");
    let region = node_of(&doc, "region", "Options");
    let mut expected = vec![region];
    for (role, name) in [
        ("checkbox", "Mono"),
        ("checkbox", "Lock"),
        ("button", "Pin"),
        ("button", "Apply"),
        ("button", "Reset"),
    ] {
        expected.push(node_of(&doc, role, name));
    }
    {
        let mut doc = doc.borrow_mut();
        doc.add_stylesheet("hide.css", ".hidden { display: none; }")
            .expect("parses");
        doc.set_attribute(region, "class", "row hidden");
    }

    // When
    let update = updated_take(&doc).expect("a change");
    model.apply(&update);

    // Then
    let mut removed = update.removed.clone();
    removed.sort_unstable();
    expected.sort_unstable();
    assert_eq!(removed, expected);
    // the dialog shrank with its row gone, so its bounds changed
    assert_eq!(
        labels(&update.nodes),
        ["button \"After\"", "dialog \"Levels\""]
    );
    let group = node_of(&doc, "group", "Tone");
    let after = node_of(&doc, "button", "After");
    assert_eq!(model.children(Some(dialog)), [group, after]);
}
