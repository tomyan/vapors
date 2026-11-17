//! The first item or clip with a coordinate that is not finite, named
//! by its kind.

use crate::{Color, Item, Quad, Rect, Scene, Shadow};

fn quad(right: f32) -> Item {
    Item::Quad(Quad::fill(
        Rect::new(0.0, 0.0, right, 4.0),
        Color::default(),
    ))
}

fn shadow(rect: Rect, blur: f32) -> Item {
    Item::Shadow(Shadow {
        rect,
        radii: [0.0; 4],
        blur,
        color: Color::default(),
        caster: Rect::new(0.0, 0.0, 4.0, 4.0),
        caster_radii: [0.0; 4],
    })
}

#[test]
fn a_scene_of_finite_items_has_none() {
    // Given
    let mut scene = Scene::new();
    scene.push(quad(4.0));

    // Then
    assert_eq!(scene.non_finite(), None);
}

#[test]
fn the_first_item_with_an_infinite_or_nan_coordinate_is_named_by_kind() {
    for (item, kind) in [
        (quad(f32::INFINITY), "quad"),
        (quad(f32::NAN), "quad"),
        (
            shadow(Rect::new(0.0, 0.0, f32::INFINITY, 4.0), 0.0),
            "shadow",
        ),
        (
            shadow(Rect::new(0.0, 0.0, 4.0, 4.0), f32::INFINITY),
            "shadow",
        ),
    ] {
        // Given: a finite item, then the one that is not
        let mut scene = Scene::new();
        scene.push(quad(4.0));
        scene.push(item);

        // When
        let found = scene.non_finite().map(|found| (found.index, found.kind));

        // Then
        assert_eq!(found, Some((1, kind)), "{:?}", scene.items()[1]);
    }
}

#[test]
fn a_finite_item_under_an_infinite_clip_is_named_as_the_clip() {
    // Given
    let mut scene = Scene::new();
    scene.set_clip(Some(Rect::new(0.0, 0.0, f32::INFINITY, 4.0)));
    scene.push(quad(4.0));

    // Then
    assert_eq!(scene.non_finite().map(|found| found.kind), Some("clip"));
}
