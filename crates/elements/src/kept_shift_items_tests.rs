//! A shifted item: a shape moves by its transform's translation and its
//! bounds, exactly, its entry, linear part and colour unchanged.

use vapors_scene::{Color, Item, Rect, Shape, ShapeRef};

use super::shifted_item;

#[test]
fn a_shifted_shape_moves_its_translation_and_bounds() {
    // Given
    let shape = Shape::new(
        ShapeRef::Table(4),
        vapors_scene::ShapeKind::ClosedForm,
        [2.0, 0.5, -0.5, 2.0, 10.0, 20.0],
        Color::from_rgba8(1, 2, 3, 255),
        Rect::new(8.0, 18.0, 30.0, 44.0),
    );

    // When
    let moved = shifted_item(Item::Shape(shape), [3.0, -4.0]);

    // Then
    let expected = Shape::new(
        shape.shape,
        shape.kind(),
        [2.0, 0.5, -0.5, 2.0, 13.0, 16.0],
        shape.colour,
        Rect::new(11.0, 14.0, 33.0, 40.0),
    );
    assert_eq!(moved, Item::Shape(expected));
}

#[test]
#[cfg(debug_assertions)]
fn the_repaint_check_takes_only_whole_edges_an_infinite_one_included() {
    // Given
    let quad = |right: f32| {
        Item::Quad(vapors_scene::Quad::fill(
            Rect::new(1.0, 2.0, right, 4.0),
            Color::default(),
        ))
    };

    // When
    let answers =
        [f32::INFINITY, f32::NAN, 3.5, 3.0].map(|right| super::whole_snapped(&quad(right)));

    // Then: an infinite edge is not whole; paint refuses its box first
    // (issue #449), so none reaches the check
    assert_eq!(answers, [false, false, false, true]);
}
