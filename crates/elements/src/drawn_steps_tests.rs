use vapors_scene::{Item, Rect, Scene, SurfaceId};

use super::DrawnSteps;
use crate::kept_box::{BoxFrom, KeptBox, Part};
use crate::layout::BoxRect;
use crate::paint_order::Step;

/// A scene of one item, or an empty one.
fn scene(draws: bool) -> Scene {
    let mut scene = Scene::new();
    if draws {
        let rect = Rect::new(0.0, 0.0, 1.0, 1.0);
        scene.push(Item::Surface {
            surface: SurfaceId(0),
            rect,
        });
    }
    scene
}

/// A kept paint whose own step, outline and thumbs draw as given.
fn kept(own: bool, outline: bool, thumb: bool) -> KeptBox {
    let rect = BoxRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    let from = BoxFrom {
        rect,
        clip: None,
        padding: taffy::Rect::zero(),
    };
    let mut own_part = Part::default();
    own_part.scene = scene(own);
    KeptBox {
        from,
        own: own_part,
        outline: scene(outline),
        thumb: scene(thumb),
    }
}

/// Slot 0 with a scroll thumb, slot 1 inside it, then their outlines.
fn steps() -> Vec<Step> {
    use Step::{Box, Outline, Thumb};
    vec![Box(0), Box(1), Thumb(0), Outline(1), Outline(0)]
}

#[test]
fn only_the_steps_whose_part_holds_items_are_listed_in_paint_order() {
    // Given
    let boxes = [
        Some(kept(true, false, true)),
        Some(kept(false, true, false)),
    ];

    // When
    let drawn = DrawnSteps::of(&steps(), &boxes);

    // Then
    let listed: Vec<Step> = drawn.steps().collect();
    assert_eq!(listed, [Step::Box(0), Step::Thumb(0), Step::Outline(1)]);
}

#[test]
fn a_step_that_now_draws_is_put_in_at_its_place() {
    // Given
    let mut boxes = [
        Some(kept(true, false, false)),
        Some(kept(false, false, false)),
    ];
    let mut drawn = DrawnSteps::of(&steps(), &boxes);
    boxes[1] = Some(kept(true, true, false));

    // When
    drawn.patch([1].into_iter(), &boxes);

    // Then
    assert_eq!(drawn, DrawnSteps::of(&steps(), &boxes));
}

#[test]
fn a_step_that_no_longer_draws_and_a_freed_slots_steps_are_taken_out() {
    // Given
    let mut boxes = [Some(kept(true, true, true)), Some(kept(true, true, false))];
    let mut drawn = DrawnSteps::of(&steps(), &boxes);
    (boxes[0], boxes[1]) = (Some(kept(true, false, true)), None);

    // When
    drawn.patch([0, 1].into_iter(), &boxes);

    // Then
    let listed: Vec<Step> = drawn.steps().collect();
    assert_eq!(listed, [Step::Box(0), Step::Thumb(0)]);
}
