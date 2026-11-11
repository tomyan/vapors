use taffy::Style;

use super::*;

/// Element boxes in pre-order from `(node, children)` pairs, the
/// children given by pre-order index.
fn tree(spec: &[(u64, &[usize])]) -> Vec<LayoutBox> {
    spec.iter()
        .map(|(node, children)| {
            let mut layout_box = LayoutBox::new(Content::Element(NodeId(*node)), Style::default());
            layout_box.children = children.to_vec();
            layout_box
        })
        .collect()
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

/// The slots a walk from the root reaches.
fn reached(boxes: &[LayoutBox]) -> Vec<bool> {
    let mut seen = vec![false; boxes.len()];
    let mut stack = vec![0];
    while let Some(slot) = stack.pop() {
        seen[slot] = true;
        stack.extend(&boxes[slot].children);
    }
    seen
}

/// Three updates: boxes inserted, removed, and inserted again.
fn churned() -> Placed {
    place_each(vec![
        tree(&[(1, &[1, 2, 3]), (10, &[]), (11, &[]), (12, &[])]),
        tree(&[(1, &[1, 2]), (12, &[]), (13, &[])]),
        tree(&[(1, &[1]), (14, &[2]), (12, &[])]),
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

#[test]
fn the_slot_map_agrees_with_a_scan_of_the_boxes() {
    // Given
    let placed = churned();

    // When
    let scanned: Vec<(Key, usize)> = (placed.boxes.iter().enumerate())
        .filter_map(|(slot, b)| match b.content {
            Content::Element(node) => Some((Key::Element(node), slot)),
            _ => None,
        })
        .collect();

    // Then
    for &(key, slot) in &scanned {
        assert_eq!(placed.slots.of(key), Some(slot), "{key:?}");
        assert_eq!(placed.slots.key(slot), Some(key), "slot {slot}");
    }
    assert_eq!(placed.slots.of.len(), scanned.len());
}

#[test]
fn a_new_root_starts_from_slot_zero_with_no_vacant_slots() {
    // Given
    let first = place_each(vec![tree(&[(1, &[1, 2]), (2, &[]), (3, &[])])]);

    // When
    let second = first.slots.place(tree(&[(9, &[1]), (2, &[])]));

    // Then
    assert_eq!(slot_of(&second, 9), Some(0));
    assert_eq!(second.boxes.len(), 2);
}
