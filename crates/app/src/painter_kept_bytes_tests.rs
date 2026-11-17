//! A shape or mesh run over a lane's rows whose item has no area: the
//! item draws nothing, and its row goes with it, so each drawn item
//! keeps its own row (`run_bytes`).

use vapors_renderer::{ClosedFormPipeline, MeshPipeline};
use vapors_scene::{Run, ShapeKind};
use vapors_shapes::Geometry;

use super::run_bytes;
use crate::shape_store::tests::{CIRCLE, IDENTITY, entry, item, item_of, store, triangle_mesh};

/// The `u32` at byte `at` of `bytes`.
fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

/// The row each of `bytes`' instances names, `one` bytes an instance,
/// its row `row_at` bytes in.
fn rows_named(bytes: &[u8], one: usize, row_at: usize) -> Vec<u32> {
    bytes
        .chunks(one)
        .map(|instance| word(instance, row_at))
        .collect()
}

#[test]
fn a_closed_form_with_no_area_in_a_rowed_run_takes_its_row_with_it() {
    // Given: rows 0, 1 and 2, the middle item scaled to nothing
    let store = store(vec![entry("a.wisp:3", b"FC", CIRCLE)]);
    let items = vec![item(0, IDENTITY), item(0, [0.0; 6]), item(0, IDENTITY)];

    // When
    let built = run_bytes((&Run::Shapes(items), (1, &[]), &[0, 1, 2]), &store).expect("builds");

    // Then
    let one = ClosedFormPipeline::INSTANCE_BYTES;
    assert_eq!(rows_named(&built.bytes, one, one - 16), [0, 2]);
}

#[test]
fn a_mesh_with_no_area_in_a_rowed_run_takes_its_row_with_it() {
    // Given
    let triangles = triangle_mesh();
    let store = store(vec![entry("a.wisp:9", b"FM", Geometry::Mesh { triangles })]);
    let mesh = |transform| item_of(0, ShapeKind::Mesh, transform);
    let items = vec![mesh(IDENTITY), mesh([0.0; 6]), mesh(IDENTITY)];

    // When
    let built = run_bytes((&Run::Meshes(items), (1, &[]), &[0, 1, 2]), &store).expect("builds");

    // Then
    assert_eq!(
        rows_named(&built.bytes, MeshPipeline::INSTANCE_BYTES, 48),
        [0, 2]
    );
    assert_eq!(built.meshes.len(), 2);
}
