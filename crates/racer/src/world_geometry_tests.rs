//! The racer's geometry, tagged by part (`world_geometry.rs`).

use super::{Part, Sink, car, scenery};

/// Each triangle's part and its corners' heights.
#[derive(Default)]
struct Triangles(Vec<(Part, [f32; 3])>);

impl Sink for Triangles {
    fn triangle(&mut self, part: Part, points: [[f32; 3]; 3], _light: f32) {
        self.0.push((part, points.map(|[_, _, z]| z)));
    }
}

#[test]
fn every_car_triangle_is_a_car_part_and_the_car_stands_14_high() {
    // Given
    let mut triangles = Triangles::default();

    // When
    car(&mut triangles);

    // Then
    assert!(triangles.0.iter().all(|(part, _)| part.on_car()));
    let top = triangles.0.iter().flat_map(|(_, z)| *z).fold(0.0, f32::max);
    assert!(top >= 14.0, "{top}");
}

#[test]
fn no_scenery_triangle_moves_with_the_car() {
    // Given
    let mut triangles = Triangles::default();

    // When
    scenery(&mut triangles);

    // Then
    assert_ne!(triangles.0.len(), 0, "no scenery");
    assert!(triangles.0.iter().all(|(part, _)| !part.on_car()));
}
