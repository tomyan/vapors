use taffy::Style;

use super::*;
use crate::layout::InlineRun;
use crate::style::{Face, TextStyle};

/// The node number `tree` reads as a text box.
const TEXT: u64 = 0;

/// Boxes in pre-order from `(node, children)` pairs, the children
/// given by pre-order index: an element box, or a text box for `TEXT`.
fn tree(spec: &[(u64, &[usize])]) -> Vec<LayoutBox> {
    spec.iter()
        .map(|(node, children)| {
            let content = match *node {
                TEXT => Content::Text(run()),
                node => Content::Element(NodeId(node)),
            };
            let mut layout_box = LayoutBox::new(content, Style::default());
            layout_box.children = children.to_vec();
            layout_box
        })
        .collect()
}

fn run() -> InlineRun {
    let style = TextStyle {
        face: Face::Geist,
        size: 16.0,
        weight: 400.0,
        line_height: vapors_text::LineHeight::Normal,
        color: vapors_scene::Color::default(),
        letter_spacing: 0.0,
        align: vapors_text::Align::Start,
        italic: false,
    };
    InlineRun {
        text: "text".to_owned(),
        style,
        visible: true,
        spans: Vec::new(),
    }
}

/// `trees` placed one after another, each into the slots the last left.
fn place_each(trees: Vec<Vec<LayoutBox>>) -> Placed {
    trees
        .into_iter()
        .fold(Placed::default(), |placed, built| placed.slots.place(built))
}

fn slot_of(placed: &Placed, node: u64) -> Option<usize> {
    placed.slots.of(Key::Element(NodeId(node)))
}

/// Three updates: element and text boxes inserted, removed, and
/// inserted again; node 1's first text box is in every update.
fn churned() -> Placed {
    place_each(vec![
        tree(&[
            (1, &[1, 2, 3, 5]),
            (10, &[]),
            (TEXT, &[]),
            (11, &[4]),
            (TEXT, &[]),
            (12, &[]),
        ]),
        tree(&[
            (1, &[1, 2, 3]),
            (TEXT, &[]),
            (12, &[]),
            (13, &[4]),
            (TEXT, &[]),
        ]),
        tree(&[
            (1, &[1, 3, 4]),
            (14, &[2]),
            (TEXT, &[]),
            (TEXT, &[]),
            (12, &[]),
        ]),
    ])
}

#[test]
fn a_kept_box_keeps_its_slot_when_a_box_is_inserted_before_it() {
    // Given
    let first = place_each(vec![tree(&[(1, &[1]), (2, &[])])]);

    // When
    let second = first.slots.place(tree(&[(1, &[1, 2]), (3, &[]), (2, &[])]));

    // Then
    assert_eq!(slot_of(&second, 2), Some(1));
    assert_eq!(slot_of(&second, 3), Some(2));
    assert_eq!(second.boxes[0].children, vec![2, 1]);
}

#[test]
fn a_text_box_keeps_its_slot_by_its_place_among_its_parents_text_boxes() {
    // Given: node 1's first text box is at pre-order index 2, then 1, then 3
    let first = place_each(vec![tree(&[
        (1, &[1, 2, 3, 5]),
        (10, &[]),
        (TEXT, &[]),
        (11, &[4]),
        (TEXT, &[]),
        (12, &[]),
    ])]);

    // When
    let placed = churned();

    // Then
    let text = Key::Text(NodeId(1), 0);
    assert_eq!(first.slots.of(text), Some(2));
    assert_eq!(placed.slots.of(text), Some(2));
    assert!(matches!(placed.boxes[2].content, Content::Text(_)));
}

#[test]
fn a_removed_box_leaves_its_slot_vacant_and_a_new_box_takes_it() {
    // Given: the second update removes node 3's box from slot 2
    let removed = place_each(vec![
        tree(&[(1, &[1, 2]), (2, &[]), (3, &[])]),
        tree(&[(1, &[1]), (2, &[])]),
    ]);
    assert!(matches!(removed.boxes[2].content, Content::Vacant));
    assert_eq!(removed.slots.key(2), None);

    // When
    let added = removed
        .slots
        .place(tree(&[(1, &[1, 2]), (2, &[]), (4, &[])]));

    // Then
    assert_eq!(slot_of(&added, 4), Some(2));
    assert_eq!(added.boxes.len(), 3);
}

#[test]
fn a_walk_from_the_root_reaches_every_box_and_no_vacant_slot() {
    // Given
    let placed = churned();

    // When
    let seen = reached(&placed.boxes);

    // Then
    for (slot, layout_box) in placed.boxes.iter().enumerate() {
        let vacant = matches!(layout_box.content, Content::Vacant);
        assert_eq!(seen[slot], !vacant, "slot {slot}");
    }
    assert!(
        placed
            .boxes
            .iter()
            .any(|b| matches!(b.content, Content::Vacant))
    );
}

/// Every box's key and slot, read from the boxes: an element box's own,
/// and its text children's by their order among them.
fn scanned(boxes: &[LayoutBox]) -> Vec<(Key, usize)> {
    let mut scanned = Vec::new();
    for (slot, layout_box) in boxes.iter().enumerate() {
        let Content::Element(node) = layout_box.content else {
            continue;
        };
        scanned.push((Key::Element(node), slot));
        let texts = layout_box
            .children
            .iter()
            .filter(|&&child| matches!(boxes[child].content, Content::Text(_)));
        scanned.extend(
            texts
                .enumerate()
                .map(|(nth, &child)| (Key::Text(node, nth), child)),
        );
    }
    scanned
}

#[test]
fn the_slot_map_agrees_with_a_scan_of_the_boxes() {
    // Given
    let placed = churned();

    // When
    let scanned = scanned(&placed.boxes);

    // Then
    assert!(scanned.iter().any(|(key, _)| matches!(key, Key::Text(..))));
    for &(key, slot) in &scanned {
        assert_eq!(placed.slots.of(key), Some(slot), "{key:?}");
        assert_eq!(placed.slots.key(slot), Some(key), "slot {slot}");
    }
    assert_eq!(placed.slots.of.len(), scanned.len());
}

#[test]
#[should_panic(expected = "box key Element(NodeId(2)) placed twice")]
fn a_key_placed_twice_is_refused() {
    // Given
    let built = tree(&[(1, &[1, 2]), (2, &[]), (2, &[])]);

    // When
    Slots::default().place(built);
}

#[path = "slots_root_tests.rs"]
mod root;
