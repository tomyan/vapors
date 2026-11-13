use vapors_scene::{Batched, Color, Item, Quad, Rect, Scene, SurfaceId};

use super::{KeptBatches, Sources};
use crate::drawn_steps::DrawnSteps;
use crate::frame_extents::Extents;
use crate::frames::{PAGE, on_page};
use crate::kept_box::{BoxFrom, KeptBox, Part};
use crate::layout::BoxRect;
use crate::paint_order::Step;

/// The target: 100 x 100 device pixels.
const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 100.0,
    bottom: 100.0,
};

fn quad_at(top: f32) -> Item {
    let rect = Rect::new(10.0, top, 20.0, top + 10.0);
    Item::Quad(Quad::fill(rect, Color::from_rgba8(1, 2, 3, 255)))
}

fn surface_at(top: f32) -> Item {
    let rect = Rect::new(10.0, top, 20.0, top + 10.0);
    Item::Surface {
        surface: SurfaceId(0),
        rect,
    }
}

/// A kept paint whose own step paints `items`, as a slot holds it.
#[allow(clippy::unnecessary_wraps)] // a slot's type, so a slot can be set to `None` beside it
fn part_of(items: &[Item]) -> Option<KeptBox> {
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
        frame: PAGE,
    };
    let mut own = Part::default();
    for item in items {
        own.scene.push(*item);
    }
    let (outline, thumb) = (Scene::new(), Scene::new());
    Some(KeptBox {
        from,
        own,
        outline,
        thumb,
    })
}

/// Each slot's own step, in slot order.
fn steps(boxes: &[Option<KeptBox>]) -> Vec<Step> {
    (0..boxes.len()).map(Step::Box).collect()
}

/// `boxes`' drawing steps and extents index, as a paint keeps them.
struct Steps {
    drawn: DrawnSteps,
    extents: Extents,
    places: usize,
}

impl Steps {
    fn of(boxes: &[Option<KeptBox>]) -> Self {
        let places = boxes.len();
        let drawn = DrawnSteps::of(&steps(boxes), boxes);
        let extents = Extents::of(&drawn, places, boxes);
        Self {
            drawn,
            extents,
            places,
        }
    }

    /// The batches' sources for `boxes`, every part on the page.
    fn sources<'a>(&'a self, boxes: &'a [Option<KeptBox>]) -> Sources<'a> {
        let (boxes, frames) = on_page(boxes);
        Sources {
            drawn: &self.drawn,
            extents: &self.extents,
            boxes,
            frames,
        }
    }
}

/// `boxes` kept for `VIEW`, and their steps.
fn kept_batches(boxes: &[Option<KeptBox>]) -> (KeptBatches, Steps) {
    let steps = Steps::of(boxes);
    (KeptBatches::of(VIEW, steps.sources(boxes)), steps)
}

/// `kept`'s batches of `boxes`, made when none are current.
fn made(kept: &mut KeptBatches, steps: &Steps, boxes: &[Option<KeptBox>]) -> Batched {
    kept.batches(steps.sources(boxes)).batched.clone()
}

/// Batching every item of `boxes` afresh.
fn afresh(boxes: &[Option<KeptBox>]) -> Batched {
    let steps = steps(boxes);
    let drawn = DrawnSteps::of(&steps, boxes);
    crate::SceneView::new(&steps, &drawn, on_page(boxes)).batches(VIEW)
}

/// Slot `slot` painted again as `part`, and its steps and the kept
/// batches patched.
fn painted_again(
    (kept, steps): (&mut KeptBatches, &mut Steps),
    boxes: &mut [Option<KeptBox>],
    (slot, part): (usize, Option<KeptBox>),
) {
    boxes[slot] = part;
    steps.drawn.patch([slot].into_iter(), boxes);
    let placed: Vec<(usize, Step)> = steps.drawn.places_of(slot).collect();
    steps
        .extents
        .patch(&placed, (&steps.drawn, steps.places), boxes);
    kept.patch(placed.into_iter(), steps.sources(boxes));
}

#[test]
fn the_kept_batches_hold_the_shown_steps_and_count_the_hidden_ones_culled() {
    // Given: slot 1 below the view, slot 2 half in it
    let boxes = [
        part_of(&[quad_at(0.0)]),
        part_of(&[quad_at(200.0), quad_at(300.0)]),
        part_of(&[quad_at(150.0), quad_at(50.0)]),
    ];

    // When
    let (mut kept, steps) = kept_batches(&boxes);

    // Then
    assert_eq!(kept.shown(), [(0, Step::Box(0)), (2, Step::Box(2))]);
    assert_eq!(made(&mut kept, &steps, &boxes), afresh(&boxes));
    assert_eq!(made(&mut kept, &steps, &boxes).culled, 3);
}

#[test]
fn a_hidden_step_painted_again_out_of_view_keeps_the_batches() {
    // Given
    let mut boxes = [part_of(&[quad_at(0.0)]), part_of(&[quad_at(200.0)])];
    let (mut kept, mut steps) = kept_batches(&boxes);
    _ = kept.batches(steps.sources(&boxes));

    // When
    let again = part_of(&[quad_at(300.0), quad_at(400.0)]);
    painted_again((&mut kept, &mut steps), &mut boxes, (1, again));

    // Then
    assert!(kept.holds_batches());
    assert_eq!(made(&mut kept, &steps, &boxes), afresh(&boxes));
}

#[test]
fn a_step_painted_again_into_or_out_of_view_makes_the_batches_again() {
    for (before, after) in [(200.0, 50.0), (50.0, 200.0), (50.0, 60.0)] {
        // Given
        let mut boxes = [part_of(&[quad_at(0.0)]), part_of(&[quad_at(before)])];
        let (mut kept, mut steps) = kept_batches(&boxes);
        _ = kept.batches(steps.sources(&boxes));

        // When
        let again = part_of(&[quad_at(after)]);
        painted_again((&mut kept, &mut steps), &mut boxes, (1, again));

        // Then
        assert!(!kept.holds_batches(), "{before} to {after}");
        assert_eq!(
            made(&mut kept, &steps, &boxes),
            afresh(&boxes),
            "{before} to {after}"
        );
    }
}

#[test]
fn a_freed_slot_leaves_the_kept_steps_and_its_culled_count() {
    for top in [50.0, 200.0] {
        // Given
        let mut boxes = [part_of(&[quad_at(0.0)]), part_of(&[quad_at(top)])];
        let (mut kept, mut steps) = kept_batches(&boxes);
        _ = kept.batches(steps.sources(&boxes));

        // When
        painted_again((&mut kept, &mut steps), &mut boxes, (1, None));

        // Then
        assert_eq!(made(&mut kept, &steps, &boxes), afresh(&boxes), "at {top}");
    }
}

#[test]
fn a_surface_out_of_view_is_kept_shown_for_its_culled_batch() {
    // Given
    let boxes = [part_of(&[quad_at(0.0)]), part_of(&[surface_at(200.0)])];

    // When
    let (mut kept, steps) = kept_batches(&boxes);

    // Then
    assert_eq!(kept.shown().len(), 2);
    assert_eq!(made(&mut kept, &steps, &boxes), afresh(&boxes));
}
