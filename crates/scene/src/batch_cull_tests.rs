use super::*;
use crate::{Color, SurfaceId, TextId};

/// The target: 100 x 100 device pixels.
const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 100.0,
    bottom: 100.0,
};

fn quad_at(top: f32) -> Item {
    Item::Quad(Quad::fill(
        Rect::new(10.0, top, 20.0, top + 10.0),
        Color::from_rgba8(1, 2, 3, 255),
    ))
}

fn text_with_ink(ink: Rect) -> Item {
    Item::Text {
        text: TextId(0),
        origin: [ink.left, ink.top],
        ink,
    }
}

fn text_at(top: f32) -> Item {
    text_with_ink(Rect::new(10.0, top, 40.0, top + 10.0))
}

fn scene_of(items: &[Item]) -> Scene {
    let mut scene = Scene::new();
    for item in items {
        scene.push(*item);
    }
    scene
}

fn kinds(batched: &Batched) -> Vec<(char, usize)> {
    let shape = |run: &Run| match run {
        Run::Shadows(shadows) => ('s', shadows.len()),
        Run::Quads(quads) => ('q', quads.len()),
        Run::Texts(texts) => ('t', texts.len()),
        Run::Images(images) => ('i', images.len()),
        Run::Surfaces(surfaces) => ('g', surfaces.len()),
        Run::Icons(icons) => ('v', icons.len()),
        Run::Shapes(shapes) => ('h', shapes.len()),
    };
    batched.batches.iter().map(|b| shape(&b.run)).collect()
}

#[test]
fn an_item_wholly_below_the_view_is_culled_and_its_neighbours_share_a_batch() {
    // Given: a text, a quad below the view, a text
    let scene = scene_of(&[text_at(0.0), quad_at(150.0), text_at(20.0)]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), [('t', 2)]);
    assert_eq!(batched.culled, 1);
}

#[test]
fn an_item_in_view_but_outside_its_clip_is_culled() {
    // Given: a quad at 50..60 under a clip of 0..20
    let mut scene = Scene::new();
    scene.set_clip(Some(Rect::new(0.0, 0.0, 100.0, 20.0)));
    scene.push(quad_at(50.0));

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), []);
    assert_eq!(batched.culled, 1);
}

#[test]
fn an_item_inside_a_clip_that_lies_outside_the_view_is_culled() {
    // Given: a quad at 150..160 inside a clip of 140..200
    let mut scene = Scene::new();
    scene.set_clip(Some(Rect::new(0.0, 140.0, 100.0, 200.0)));
    scene.push(quad_at(150.0));

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), []);
    assert_eq!(batched.culled, 1);
}

#[test]
fn an_item_reaching_into_the_view_by_part_of_a_pixel_is_drawn() {
    // Given: a quad from 99.6 down
    let scene = scene_of(&[quad_at(99.6)]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), [('q', 1)]);
    assert_eq!(batched.culled, 0);
}

#[test]
fn a_shadow_below_the_view_whose_blur_reaches_into_it_is_drawn() {
    // Given: a shadow shaped 104..114 with a 4 px blur, which draws
    // three standard deviations (6 px) out: up to 98
    let rect = Rect::new(10.0, 104.0, 20.0, 114.0);
    let shadow = Shadow {
        rect,
        radii: [0.0; 4],
        blur: 4.0,
        color: Color::from_rgba8(0, 0, 0, 255),
        caster: rect,
        caster_radii: [0.0; 4],
    };
    let scene = scene_of(&[Item::Shadow(shadow)]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), [('s', 1)]);
}

#[test]
fn a_text_above_the_view_whose_ink_reaches_into_it_is_drawn() {
    // Given: a text whose origin is above the view and ink reaches 3 px in
    let scene = scene_of(&[text_with_ink(Rect::new(10.0, -20.0, 40.0, 3.0))]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), [('t', 1)]);
}

#[test]
fn a_culled_surface_keeps_its_place_in_a_batch_marked_culled() {
    // Given: a surface right of the view, between two quads in view
    let surface = Item::Surface {
        surface: SurfaceId(0),
        rect: Rect::new(150.0, 0.0, 160.0, 10.0),
    };
    let scene = scene_of(&[quad_at(0.0), surface, quad_at(20.0)]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), [('q', 1), ('g', 1), ('q', 1)]);
    assert!(batched.batches[1].culled);
    assert_eq!(batched.batches[1].clip, None);
    assert_eq!(batched.culled, 1);
}

#[test]
fn a_culled_surface_does_not_share_a_batch_with_one_in_view() {
    // Given: an unclipped surface in view, then one right of the view
    let surface = |left| Item::Surface {
        surface: SurfaceId(0),
        rect: Rect::new(left, 0.0, left + 10.0, 10.0),
    };
    let scene = scene_of(&[surface(0.0), surface(150.0)]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    let culled: Vec<bool> = batched.batches.iter().map(|batch| batch.culled).collect();
    assert_eq!(culled, [false, true]);
}

#[test]
fn an_item_whose_bounds_cannot_be_placed_is_drawn() {
    // Given: a quad with a NaN edge
    let scene = scene_of(&[quad_at(f32::NAN)]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), [('q', 1)]);
}

#[test]
fn a_shape_is_culled_by_its_bounds_not_its_transform() {
    // Given: two shapes translated into the view, one whose bounds lie
    // below it, between two quads
    let shape = |bounds_top: f32| {
        Item::Shape(crate::Shape {
            shape: crate::ShapeRef(0),
            transform: [1.0, 0.0, 0.0, 1.0, 10.0, 10.0],
            colour: Color::from_rgba8(1, 2, 3, 255),
            bounds: Rect::new(10.0, bounds_top, 20.0, bounds_top + 10.0),
        })
    };
    let scene = scene_of(&[shape(10.0), quad_at(0.0), shape(150.0), quad_at(20.0)]);

    // When
    let batched = scene.batches(VIEW);

    // Then
    assert_eq!(kinds(&batched), [('h', 1), ('q', 2)]);
    assert_eq!(batched.culled, 1);
}
