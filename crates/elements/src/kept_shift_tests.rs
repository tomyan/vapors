use vapors_dom::NodeId;
use vapors_scene::{Color, Item, Quad, Rect, Scene};

use super::Shift;
use crate::kept_box::{BoxFrom, KeptBox, Part};
use crate::layout::{BoxRect, Content};
use crate::painting::Painting;
use crate::slab::Slabs;

pub(super) const NODE: NodeId = NodeId(7);

pub(super) fn rect(x: f32, y: f32) -> BoxRect {
    BoxRect {
        x,
        y,
        width: 10.0,
        height: 10.0,
    }
}

pub(super) fn from(at: BoxRect) -> BoxFrom {
    BoxFrom {
        rect: at,
        clip: None,
        padding: taffy::Rect::zero(),
    }
}

/// A quad over `at` and the element's hit there, as paint makes them.
pub(super) fn painting(at: BoxRect) -> Painting {
    let mut painting = Painting::default();
    let device = Rect::new(at.x, at.y, at.x + at.width, at.y + at.height);
    painting
        .scene
        .push(Item::Quad(Quad::fill(device, Color::default())));
    painting.hits.push((NODE, at));
    painting
}

pub(super) fn kept(mut painting: Painting, at: BoxRect) -> KeptBox {
    KeptBox {
        from: from(at),
        own: Part::take(&mut painting),
        outline: Scene::new(),
        thumb: Scene::new(),
    }
}

pub(super) fn shift_of(kept: &KeptBox, to: BoxFrom) -> Option<Shift> {
    Shift::of(0, (kept, &Content::Element(NODE)), to, (1.0, |at| [at, at]))
}

#[test]
fn a_whole_move_shifts_the_quad_and_cuts_the_hit_from_the_new_rect() {
    // Given: a part under a clip whose box moves to cross its bottom
    let mut part = kept(painting(rect(0.0, 0.0)), rect(0.0, 0.0));
    part.from.clip = Some(rect(0.0, 20.0));
    part.own
        .scene
        .clips_mut()
        .fill(Some(Rect::new(0.0, 20.0, 10.0, 30.0)));
    let mut to = from(rect(0.0, 24.0));
    to.clip = part.from.clip;

    // When
    let shift = shift_of(&part, to).expect("a whole move shifts");
    shift.apply_to(&mut part, &Slabs::default(), 1.0);

    // Then
    let expected = Item::Quad(Quad::fill(
        Rect::new(0.0, 24.0, 10.0, 34.0),
        Color::default(),
    ));
    assert_eq!(part.own.scene.items(), [expected]);
    assert_eq!(part.own.hits, [(NODE, rect(0.0, 24.0).cut_to(20.0, 30.0))]);
    assert_eq!(part.from, to);
}

#[test]
fn a_moved_clip_is_set_on_the_items_and_cuts_the_hit() {
    // Given: a part whose clip moves with it, as a clipping box's child's does
    let mut part = kept(painting(rect(0.0, 0.0)), rect(0.0, 0.0));
    part.from.clip = Some(rect(0.0, 5.0));
    part.own
        .scene
        .clips_mut()
        .fill(Some(Rect::new(0.0, 5.0, 10.0, 15.0)));
    let mut to = from(rect(0.0, 24.0));
    to.clip = Some(rect(0.0, 29.0));

    // When
    let shift = shift_of(&part, to).expect("a clip that moved is set anew");
    shift.apply_to(&mut part, &Slabs::default(), 1.0);

    // Then
    assert_eq!(
        part.own.scene.clips(),
        [Some(Rect::new(0.0, 29.0, 10.0, 39.0))]
    );
    assert_eq!(part.own.hits, [(NODE, rect(0.0, 24.0).cut_to(29.0, 39.0))]);
}

#[test]
fn an_unmoved_axis_at_a_fractional_position_shifts_on_the_other() {
    // Given: a box at x 54.336, as a label's sibling is, its quad
    // snapped to whole pixels as paint snaps it
    let mut painting = Painting::default();
    let snapped = Rect::new(54.0, 0.0, 64.0, 10.0);
    painting
        .scene
        .push(Item::Quad(Quad::fill(snapped, Color::default())));
    let mut part = kept(painting, rect(54.336, 0.0));

    // When
    let shift = shift_of(&part, from(rect(54.336, 24.0))).expect("y alone moves");
    shift.apply_to(&mut part, &Slabs::default(), 1.0);

    // Then
    let expected = Item::Quad(Quad::fill(
        Rect::new(54.0, 24.0, 64.0, 34.0),
        Color::default(),
    ));
    assert_eq!(part.own.scene.items(), [expected]);
}

#[test]
fn a_move_at_scale_two_is_in_device_pixels() {
    // Given
    let mut part = kept(painting(rect(0.0, 0.0)), rect(0.0, 0.0));

    // When
    let shift = Shift::of(
        0,
        (&part, &Content::Element(NODE)),
        from(rect(0.5, 1.0)),
        (2.0, |at| [at, at]),
    );
    shift
        .expect("half a CSS pixel is a whole device pixel at 2x")
        .apply_to(&mut part, &Slabs::default(), 2.0);

    // Then
    let expected = Item::Quad(Quad::fill(
        Rect::new(1.0, 2.0, 11.0, 12.0),
        Color::default(),
    ));
    assert_eq!(part.own.scene.items(), [expected]);
}

impl BoxRect {
    /// This rect cut to the rows `top` to `bottom`.
    fn cut_to(self, top: f32, bottom: f32) -> Self {
        Self {
            y: self.y.max(top),
            height: (self.y + self.height).min(bottom) - self.y.max(top),
            ..self
        }
    }
}
