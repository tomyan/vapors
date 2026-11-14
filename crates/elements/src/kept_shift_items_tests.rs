//! A shifted item: a shape moves by its transform's translation and its
//! bounds, exactly, its entry, linear part and colour unchanged.

use vapors_scene::{Color, Item, Rect, Shape, ShapeRef};

use super::shifted_item;

#[test]
fn a_shifted_shape_moves_its_translation_and_bounds() {
    // Given
    let shape = Shape {
        shape: ShapeRef(4),
        transform: [2.0, 0.5, -0.5, 2.0, 10.0, 20.0],
        colour: Color::from_rgba8(1, 2, 3, 255),
        bounds: Rect::new(8.0, 18.0, 30.0, 44.0),
    };

    // When
    let moved = shifted_item(Item::Shape(shape), [3.0, -4.0]);

    // Then
    let expected = Shape {
        transform: [2.0, 0.5, -0.5, 2.0, 13.0, 16.0],
        bounds: Rect::new(11.0, 14.0, 33.0, 40.0),
        ..shape
    };
    assert_eq!(moved, Item::Shape(expected));
}
