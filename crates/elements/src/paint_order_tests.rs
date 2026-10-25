use taffy::Style;
use vapors_dom::NodeId;

use super::*;
use crate::layout::Content;

/// Boxes whose parents are `parents` (box 0 the root, `None`); each
/// element box is node `index`.
fn tree(parents: &[Option<usize>]) -> Vec<LayoutBox> {
    let mut boxes: Vec<LayoutBox> = (0..parents.len())
        .map(|i| LayoutBox::new(Content::Element(NodeId(i as u64)), Style::default()))
        .collect();
    for (child, parent) in parents.iter().enumerate() {
        if let Some(parent) = parent {
            boxes[*parent].children.push(child);
        }
    }
    boxes
}

/// The boxes alone, thumbs left out.
fn order(parents: &[Option<usize>], stacking: &[(usize, Stacking)]) -> Vec<usize> {
    steps(parents, stacking)
        .into_iter()
        .filter_map(|step| match step {
            Step::Box(index) => Some(index),
            Step::Thumb(_) => None,
        })
        .collect()
}

fn positioned(z: Option<i32>) -> Stacking {
    Stacking {
        positioned: true,
        z,
        ..Stacking::default()
    }
}

#[test]
fn boxes_with_nothing_positioned_paint_in_tree_order() {
    // Then: root > 1 > 2, root > 3
    assert_eq!(order(&[None, Some(0), Some(1), Some(0)], &[]), [0, 1, 2, 3]);
}

#[test]
fn a_positioned_box_paints_after_the_in_flow_boxes_of_its_context() {
    // Given: root > 1 (relative), 2
    let found = order(&[None, Some(0), Some(0)], &[(1, positioned(None))]);

    // Then
    assert_eq!(found, [0, 2, 1]);
}

#[test]
fn z_index_orders_below_level_and_above() {
    // Given: root > 1 (z 2), 2 (z 1), 3 (z -1), 4 (in flow)
    let stacking = [
        (1, positioned(Some(2))),
        (2, positioned(Some(1))),
        (3, positioned(Some(-1))),
    ];

    // When
    let found = order(&[None, Some(0), Some(0), Some(0), Some(0)], &stacking);

    // Then: the negative first, the in-flow box, then 1 and 2 by z
    assert_eq!(found, [0, 3, 4, 2, 1]);
}

#[test]
fn a_positioned_box_with_z_auto_lends_its_positioned_children_to_its_context() {
    // Given: root > 1 (relative, z auto) > 2 (z -1), 3; root > 4
    let stacking = [(1, positioned(None)), (2, positioned(Some(-1)))];

    // When
    let found = order(&[None, Some(0), Some(1), Some(1), Some(0)], &stacking);

    // Then: 2 below the root's in-flow box 4; 1 painted whole after it
    assert_eq!(found, [0, 2, 4, 1, 3]);
}

#[test]
fn a_box_with_a_z_index_keeps_its_positioned_children_in_its_own_context() {
    // Given: root > 1 (relative, z 0) > 2 (z -1), 3; root > 4
    let stacking = [(1, positioned(Some(0))), (2, positioned(Some(-1)))];

    // When
    let found = order(&[None, Some(0), Some(1), Some(1), Some(0)], &stacking);

    // Then: 2 stays inside 1, above 1's own box
    assert_eq!(found, [0, 4, 1, 2, 3]);
}

#[test]
fn a_fixed_box_roots_a_context_without_a_z_index() {
    // Given: as above, with 1 fixed and z auto
    let fixed = Stacking {
        positioned: true,
        fixed: true,
        ..Stacking::default()
    };
    let stacking = [(1, fixed), (2, positioned(Some(-1)))];

    // When
    let found = order(&[None, Some(0), Some(1), Some(1), Some(0)], &stacking);

    // Then
    assert_eq!(found, [0, 4, 1, 2, 3]);
}

fn steps(parents: &[Option<usize>], stacking: &[(usize, Stacking)]) -> Vec<Step> {
    let boxes = tree(parents);
    paint_steps(&boxes, &|index| {
        stacking
            .iter()
            .find(|(i, _)| *i == index)
            .map_or(Stacking::default(), |(_, s)| *s)
    })
}
