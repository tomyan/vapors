use vapors_scene::{Color, Item, Quad, Rect, Scene, SurfaceId};

use super::Extents;
use super::moves_tests::assert_blocks_total;
use crate::drawn_steps::DrawnSteps;
use crate::frames::PAGE;
use crate::kept_box::{BoxFrom, KeptBox, Part};
use crate::layout::BoxRect;
use crate::paint_order::Step;

pub(super) fn quad(top: f32, bottom: f32) -> Item {
    let rect = Rect::new(0.0, top, 10.0, bottom);
    Item::Quad(Quad::fill(rect, Color::from_rgba8(1, 2, 3, 255)))
}

/// A row of 10 px at `top`.
pub(super) fn row(top: f32) -> Item {
    quad(top, top + 10.0)
}

/// A kept paint in `frame` whose own step paints `items`.
#[allow(clippy::unnecessary_wraps)] // a slot's type, so a slot can be set to `None` beside it
pub(super) fn part(frame: usize, items: &[Item]) -> Option<KeptBox> {
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
        frame,
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

/// Each slot's own step, in slot order, and those that draw.
pub(super) fn drawn(boxes: &[Option<KeptBox>]) -> (Vec<Step>, DrawnSteps) {
    let steps: Vec<Step> = (0..boxes.len()).map(Step::Box).collect();
    let drawn = DrawnSteps::of(&steps, boxes);
    (steps, drawn)
}

pub(super) fn extents(boxes: &[Option<KeptBox>]) -> Extents {
    let (steps, drawn) = drawn(boxes);
    Extents::of(&drawn, steps.len(), boxes)
}

/// The places of `extents`' candidates in `frame` for the band from
/// `top` to `bottom`, in place order.
pub(super) fn candidates(extents: &Extents, frame: usize, (top, bottom): (f32, f32)) -> Vec<usize> {
    let view = Rect::new(0.0, top, 100.0, bottom);
    let mut places: Vec<usize> = extents.candidates(frame, view).map(|e| e.place).collect();
    places.sort_unstable();
    places
}

/// `boxes` with slot `slot` painted again as `now`, `extents` patched.
pub(super) fn painted_again(
    extents: &mut Extents,
    boxes: &mut [Option<KeptBox>],
    (slot, now): (usize, Option<KeptBox>),
) {
    boxes[slot] = now;
    let (steps, drawn) = drawn(boxes);
    let placed: Vec<(usize, Step)> = drawn.places_of(slot).collect();
    extents.patch(&placed, (&drawn, steps.len()), boxes);
}

/// Ten rows of 10 px on the page, the first at 0.
pub(super) fn rows() -> Vec<Option<KeptBox>> {
    (0..10u8)
        .map(|at| part(PAGE, &[row(f32::from(at) * 10.0)]))
        .collect()
}

#[test]
fn the_candidates_of_a_view_are_the_steps_whose_bounds_meet_it() {
    // Given
    let extents = extents(&rows());

    // When: a band from 25 to 45, which the bounds' fringe widens
    let found = candidates(&extents, PAGE, (25.0, 45.0));

    // Then
    assert_eq!(found, [2, 3, 4]);
    assert_eq!(extents.items(), 10);
}

#[test]
fn a_tall_step_starting_above_the_view_is_a_candidate_by_its_reach() {
    // Given: a step from 0 to 100 first, then rows from 200
    let mut boxes = vec![part(PAGE, &[quad(0.0, 100.0)])];
    boxes.extend((0..5u8).map(|at| part(PAGE, &[row(200.0 + f32::from(at) * 10.0)])));
    let extents = extents(&boxes);

    // When / Then
    assert_eq!(candidates(&extents, PAGE, (50.0, 60.0)), [0]);
    assert_eq!(
        candidates(&extents, PAGE, (120.0, 150.0)),
        Vec::<usize>::new()
    );
}

#[test]
fn a_tall_step_between_short_ones_is_a_candidate_below_them_by_its_reach() {
    // Given: a row at 0, a step from 5 to 100, rows from 20 to 50
    let mut boxes = vec![part(PAGE, &[row(0.0)]), part(PAGE, &[quad(5.0, 100.0)])];
    boxes.extend((2..5u8).map(|at| part(PAGE, &[row(f32::from(at) * 10.0)])));
    let extents = extents(&boxes);

    // When / Then
    assert_eq!(candidates(&extents, PAGE, (60.0, 70.0)), [1]);
}

#[test]
fn a_surface_is_a_candidate_in_every_view() {
    // Given
    let surface = Item::Surface {
        surface: SurfaceId(0),
        rect: Rect::new(0.0, 500.0, 10.0, 510.0),
    };
    let extents = extents(&[part(PAGE, &[row(0.0)]), part(PAGE, &[surface])]);

    // When / Then
    assert_eq!(candidates(&extents, PAGE, (0.0, 20.0)), [0, 1]);
}

#[test]
fn each_frame_holds_its_own_steps() {
    // Given: slot 1 in frame 7
    let boxes = [part(PAGE, &[row(0.0)]), part(7, &[row(0.0)])];

    // When
    let extents = extents(&boxes);

    // Then
    assert_eq!(candidates(&extents, PAGE, (0.0, 20.0)), [0]);
    assert_eq!(candidates(&extents, 7, (0.0, 20.0)), [1]);
}

#[test]
fn a_patched_step_moves_and_the_index_equals_one_made_afresh() {
    for (slot, now) in [
        (3, row(95.0)),
        (3, row(30.0)),
        (0, quad(-50.0, 500.0)),
        (9, row(5.0)),
    ] {
        // Given
        let mut boxes = rows();
        let mut patched = extents(&boxes);

        // When
        painted_again(&mut patched, &mut boxes, (slot, part(PAGE, &[now])));

        // Then
        assert_eq!(patched, extents(&boxes), "slot {slot} to {now:?}");
        assert_eq!(patched.items(), 10, "slot {slot} to {now:?}");
        assert_eq!(patched.total_differing(), None, "slot {slot} to {now:?}");
    }
}

#[test]
fn a_freed_step_leaves_the_index_and_its_frame_with_its_last_step() {
    // Given
    let mut boxes = [part(PAGE, &[row(0.0)]), part(7, &[row(0.0)])];
    let mut patched = extents(&boxes);

    // When
    painted_again(&mut patched, &mut boxes, (1, None));

    // Then
    assert_eq!(patched, extents(&boxes));
    assert_eq!(patched.frames().collect::<Vec<_>>(), [PAGE]);
    assert_blocks_total(&mut patched);
}

#[test]
fn more_steps_patched_than_the_index_patches_one_by_one_make_it_afresh() {
    // Given: 200 rows, every one moved down 5 px
    let mut boxes: Vec<_> = (0..200u8)
        .map(|at| part(PAGE, &[row(f32::from(at) * 10.0)]))
        .collect();
    let mut patched = extents(&boxes);
    for (at, slot) in boxes.iter_mut().enumerate() {
        *slot = part(
            PAGE,
            &[row(f32::from(u8::try_from(at).expect("200")) * 10.0 + 5.0)],
        );
    }

    // When
    let (steps, drawn) = drawn(&boxes);
    let placed: Vec<(usize, Step)> = drawn.placed().collect();
    patched.patch(&placed, (&drawn, steps.len()), &boxes);

    // Then
    assert_eq!(patched, extents(&boxes));
}
