//! Shape items batched: consecutive shapes of one kind share a run in
//! paint order; a quad or the other kind between them splits it.

use super::*;
use crate::{Color, Quad};

fn quad(left: f32) -> Item {
    Item::Quad(Quad::fill(
        Rect::new(left, 0.0, left + 1.0, 1.0),
        Color::from_rgba8(1, 2, 3, 255),
    ))
}

/// The scene's batches on a target that shows every item.
fn batches(scene: &Scene) -> Vec<Batch> {
    scene.batches(Rect::new(0.0, 0.0, 100.0, 100.0)).batches
}

fn shape(left: f32) -> Item {
    Item::Shape(crate::Shape {
        shape: crate::ShapeRef(0),
        kind: crate::ShapeKind::ClosedForm,
        transform: [1.0, 0.0, 0.0, 1.0, left, 0.0],
        colour: Color::from_rgba8(1, 2, 3, 255),
        bounds: Rect::new(left, 0.0, left + 1.0, 1.0),
    })
}

#[test]
fn consecutive_shapes_share_a_batch_and_a_quad_between_them_splits_it() {
    // Given: two shapes, a quad, a shape
    let mut scene = Scene::new();
    for item in [shape(0.0), shape(1.0), quad(2.0), shape(3.0)] {
        scene.push(item);
    }

    // When
    let batches = batches(&scene);

    // Then: in painting order, the shapes' own order kept
    let lefts: Vec<(char, Vec<f32>)> = batches
        .iter()
        .map(|batch| match &batch.run {
            Run::Shapes(shapes) => ('h', shapes.iter().map(|s| s.transform[4]).collect()),
            Run::Quads(quads) => ('q', quads.iter().map(|q| q.rect.left).collect()),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        lefts,
        [('h', vec![0.0, 1.0]), ('q', vec![2.0]), ('h', vec![3.0])]
    );
}

#[test]
fn closed_form_and_mesh_shapes_batch_apart_and_each_kind_together() {
    // Given: a closed form, two meshes, a closed form
    let of = |kind, left: f32| {
        let Item::Shape(shape) = shape(left) else {
            unreachable!("shape() makes a shape")
        };
        Item::Shape(crate::Shape { kind, ..shape })
    };
    let (closed, mesh) = (crate::ShapeKind::ClosedForm, crate::ShapeKind::Mesh);
    let mut scene = Scene::new();
    for item in [
        of(closed, 0.0),
        of(mesh, 1.0),
        of(mesh, 2.0),
        of(closed, 3.0),
    ] {
        scene.push(item);
    }

    // When
    let batches = batches(&scene);

    // Then
    let kinds: Vec<(char, usize)> = batches
        .iter()
        .map(|batch| match &batch.run {
            Run::Shapes(shapes) => ('h', shapes.len()),
            Run::Meshes(meshes) => ('m', meshes.len()),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, [('h', 1), ('m', 2), ('h', 1)]);
}
