use taffy::Style;
use vapors_dom::NodeId;

use crate::layout::{Content, LayoutBox};
use crate::paint_order::{Stacking, Step, paint_steps};

/// Boxes whose parents are `parents` (box 0 the root, `None`).
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

fn steps(parents: &[Option<usize>], stacking: &[(usize, Stacking)]) -> Vec<Step> {
    let boxes = tree(parents);
    paint_steps(&boxes, &|index| {
        stacking
            .iter()
            .find(|(i, _)| *i == index)
            .map_or(Stacking::default(), |(_, s)| *s)
    })
}

fn positioned(z: Option<i32>) -> Stacking {
    Stacking {
        positioned: true,
        z,
        ..Stacking::default()
    }
}

fn scrolling() -> Stacking {
    Stacking {
        scrolls: true,
        ..Stacking::default()
    }
}

#[test]
fn a_scroll_containers_thumb_paints_after_its_in_flow_content() {
    // Given: root > 1 (scrolls) > 2 > 3, root > 4
    let found = steps(
        &[None, Some(0), Some(1), Some(2), Some(0)],
        &[(1, scrolling())],
    );

    // Then
    assert_eq!(
        found,
        [
            Step::Box(0),
            Step::Box(1),
            Step::Box(2),
            Step::Box(3),
            Step::Thumb(1),
            Step::Box(4)
        ]
    );
}

#[test]
fn a_scroll_container_that_roots_a_context_paints_its_thumb_after_its_layers() {
    // Given: root > 1 (scrolls, z-index 1) > 2 (relative), root > 3
    let context = Stacking {
        scrolls: true,
        ..positioned(Some(1))
    };
    let found = steps(
        &[None, Some(0), Some(1), Some(0)],
        &[(1, context), (2, positioned(None))],
    );

    // Then
    assert_eq!(
        found,
        [
            Step::Box(0),
            Step::Box(3),
            Step::Box(1),
            Step::Box(2),
            Step::Thumb(1)
        ]
    );
}

#[test]
fn a_thumb_paints_over_its_containers_positioned_descendants() {
    // Given: root > 1 (scrolls) > 2 (relative, so painted in the root's
    // context after the in-flow boxes), root > 3; Chromium paints a
    // scroll container's scrollbars after its positioned descendants
    let found = steps(
        &[None, Some(0), Some(1), Some(0)],
        &[(1, scrolling()), (2, positioned(None))],
    );

    // Then: the thumb after 2, its descendant, and before nothing else
    assert_eq!(
        found,
        [
            Step::Box(0),
            Step::Box(1),
            Step::Box(3),
            Step::Box(2),
            Step::Thumb(1)
        ]
    );
}

#[test]
fn a_thumb_stays_under_what_paints_after_its_container_and_is_not_inside_it() {
    // Given: root > 1 (scrolls) > 2 (relative), root > 3 (relative):
    // 3 paints after 2 and is not inside 1
    let found = steps(
        &[None, Some(0), Some(1), Some(0)],
        &[
            (1, scrolling()),
            (2, positioned(None)),
            (3, positioned(None)),
        ],
    );

    // Then
    assert_eq!(
        found,
        [
            Step::Box(0),
            Step::Box(1),
            Step::Box(2),
            Step::Thumb(1),
            Step::Box(3)
        ]
    );
}

#[test]
fn nested_thumbs_moved_after_one_box_keep_the_inner_thumb_first() {
    // Given: root > 1 (scrolls) > 2 (scrolls) > 3 (relative), root > 4:
    // both thumbs move to just after 3, the last box inside each
    let found = steps(
        &[None, Some(0), Some(1), Some(2), Some(0)],
        &[(1, scrolling()), (2, scrolling()), (3, positioned(None))],
    );

    // Then
    assert_eq!(
        found,
        [
            Step::Box(0),
            Step::Box(1),
            Step::Box(2),
            Step::Box(4),
            Step::Box(3),
            Step::Thumb(2),
            Step::Thumb(1)
        ]
    );
}

#[test]
fn sibling_scroll_containers_each_paint_their_thumb_after_their_own_content() {
    // Given: root > 1 (scrolls) > 2, root > 3 (scrolls) > 4, root > 5
    let found = steps(
        &[None, Some(0), Some(1), Some(0), Some(3), Some(0)],
        &[(1, scrolling()), (3, scrolling())],
    );

    // Then
    assert_eq!(
        found,
        [
            Step::Box(0),
            Step::Box(1),
            Step::Box(2),
            Step::Thumb(1),
            Step::Box(3),
            Step::Box(4),
            Step::Thumb(3),
            Step::Box(5)
        ]
    );
}

#[test]
fn an_empty_scroll_containers_thumb_stays_where_the_walk_put_it() {
    // Given: root > 1 (scrolls, no children), root > 2 (relative)
    let found = steps(
        &[None, Some(0), Some(0)],
        &[(1, scrolling()), (2, positioned(None))],
    );

    // Then
    assert_eq!(
        found,
        [Step::Box(0), Step::Box(1), Step::Thumb(1), Step::Box(2)]
    );
}
