use vapors_scene::{Batched, Color, Item, Quad, Rect, Scene, SurfaceId};

use super::KeptBatches;
use crate::drawn_steps::DrawnSteps;
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

/// `boxes` kept for `VIEW`, and the steps that draw.
fn kept_batches(boxes: &[Option<KeptBox>]) -> (KeptBatches, DrawnSteps) {
    let steps = steps(boxes);
    let drawn = DrawnSteps::of(&steps, boxes);
    (KeptBatches::of(VIEW, (&drawn, steps.len()), boxes), drawn)
}

/// Batching every item of `boxes` afresh.
fn afresh(boxes: &[Option<KeptBox>]) -> Batched {
    crate::SceneView::new(&steps(boxes), &DrawnSteps::of(&steps(boxes), boxes), boxes).batches(VIEW)
}

/// Slot `slot` painted again as `items`, and both kept for the patch.
fn painted_again(
    (kept, drawn): (&mut KeptBatches, &mut DrawnSteps),
    boxes: &mut [Option<KeptBox>],
    (slot, part): (usize, Option<KeptBox>),
) {
    boxes[slot] = part;
    drawn.patch([slot].into_iter(), boxes);
    kept.patch(drawn.places_of(slot), boxes);
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
    let (mut kept, _) = kept_batches(&boxes);

    // Then
    assert_eq!(kept.shown, [(0, Step::Box(0)), (2, Step::Box(2))]);
    assert_eq!(*kept.batches(&boxes), afresh(&boxes));
    assert_eq!(kept.batches(&boxes).culled, 3);
}

#[test]
fn a_hidden_step_painted_again_out_of_view_keeps_the_batches() {
    // Given
    let mut boxes = [part_of(&[quad_at(0.0)]), part_of(&[quad_at(200.0)])];
    let (mut kept, mut drawn) = kept_batches(&boxes);
    _ = kept.batches(&boxes);

    // When
    let again = part_of(&[quad_at(300.0), quad_at(400.0)]);
    painted_again((&mut kept, &mut drawn), &mut boxes, (1, again));

    // Then
    assert!(kept.holds_batches());
    assert_eq!(*kept.batches(&boxes), afresh(&boxes));
}

#[test]
fn a_step_painted_again_into_or_out_of_view_makes_the_batches_again() {
    for (before, after) in [(200.0, 50.0), (50.0, 200.0), (50.0, 60.0)] {
        // Given
        let mut boxes = [part_of(&[quad_at(0.0)]), part_of(&[quad_at(before)])];
        let (mut kept, mut drawn) = kept_batches(&boxes);
        _ = kept.batches(&boxes);

        // When
        let again = part_of(&[quad_at(after)]);
        painted_again((&mut kept, &mut drawn), &mut boxes, (1, again));

        // Then
        assert!(!kept.holds_batches(), "{before} to {after}");
        assert_eq!(*kept.batches(&boxes), afresh(&boxes), "{before} to {after}");
    }
}

#[test]
fn a_freed_slot_leaves_the_kept_steps_and_its_culled_count() {
    for top in [50.0, 200.0] {
        // Given
        let mut boxes = [part_of(&[quad_at(0.0)]), part_of(&[quad_at(top)])];
        let (mut kept, mut drawn) = kept_batches(&boxes);
        _ = kept.batches(&boxes);

        // When
        painted_again((&mut kept, &mut drawn), &mut boxes, (1, None));

        // Then
        assert_eq!(*kept.batches(&boxes), afresh(&boxes), "at {top}");
    }
}

#[test]
fn a_surface_out_of_view_is_kept_shown_for_its_culled_batch() {
    // Given
    let boxes = [part_of(&[quad_at(0.0)]), part_of(&[surface_at(200.0)])];

    // When
    let (mut kept, _) = kept_batches(&boxes);

    // Then
    assert_eq!(kept.shown.len(), 2);
    assert_eq!(*kept.batches(&boxes), afresh(&boxes));
}

/// A document with one drawn row, updated once.
fn drawn_document() -> crate::document::Document {
    use vapors_dom::Dom;
    let mut doc = crate::document::Document::new(vapors_text::FontSystem::bundle_only());
    doc.add_stylesheet("app.css", ".row { height: 20px; background-color: red; }")
        .expect("parses");
    let (body, row) = (doc.body(), doc.create_element("div"));
    doc.set_attribute(row, "class", "row");
    doc.append(body, row);
    doc.update(UPDATE_VIEWPORT).expect("updates");
    doc
}

const UPDATE_VIEWPORT: crate::update::Viewport = crate::update::Viewport {
    width: 100.0,
    height: 100.0,
    scale: 1.0,
};

#[test]
fn a_second_draw_at_the_same_view_after_an_idle_update_makes_no_batches() {
    // Given: drawn once
    let mut doc = drawn_document();
    _ = doc.painted.as_ref().expect("painted").batches(VIEW);
    let before = super::BUILDS.with(std::cell::Cell::get);

    // When: an update that changes nothing, and a draw at the same view
    let painted = doc.update(UPDATE_VIEWPORT).expect("updates");
    _ = painted.batches(VIEW);

    // Then
    assert_eq!(super::BUILDS.with(std::cell::Cell::get), before);
}

#[test]
#[should_panic(expected = "drop the Ref before drawing")]
fn making_batches_while_a_ref_is_held_names_the_fix() {
    // Given: a draw whose Ref is held
    let doc = drawn_document();
    let painted = doc.painted.as_ref().expect("painted");
    let _held = painted.batches(VIEW);

    // When: a draw at another view
    _ = painted.batches(Rect::new(0.0, 0.0, 50.0, 50.0));
}
