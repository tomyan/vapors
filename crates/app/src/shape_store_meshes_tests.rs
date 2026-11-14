//! The store's mesh side: an item whose kind is not its entry's, either
//! way round, and a mesh drawn before its vertices are uploaded, each
//! refused naming the entry.

use vapors_scene::ShapeKind;
use vapors_shapes::Geometry;

use crate::shape_store::tests::{CIRCLE, IDENTITY, entry, item, store, triangle_mesh};

#[test]
fn an_item_whose_kind_is_not_its_entrys_is_refused_naming_both() {
    // Given: a mesh entry, and a closed form
    let triangles = triangle_mesh();
    let store = store(vec![
        entry("a.wisp:9", b"FM", Geometry::Mesh { triangles }),
        entry("a.wisp:3", b"FC", CIRCLE),
    ]);
    let mut mesh_said_closed = item(0, IDENTITY);
    mesh_said_closed.kind = ShapeKind::ClosedForm;
    let mut closed_said_mesh = item(1, IDENTITY);
    closed_said_mesh.kind = ShapeKind::Mesh;

    // When
    let as_closed = store
        .closed_forms(&[mesh_said_closed])
        .expect_err("refused");
    let as_mesh = store
        .mesh_instances(&[closed_said_mesh])
        .expect_err("refused");

    // Then
    assert_eq!(
        as_closed.to_string(),
        "shape 0, the entry at a.wisp:9, is a Mesh but its item says ClosedForm; build the item's kind from its entry's geometry; reproduce with `cargo test -p vapors-app --lib shape_store`"
    );
    assert_eq!(
        as_mesh.to_string(),
        "shape 1, the entry at a.wisp:3, is a ClosedForm but its item says Mesh; build the item's kind from its entry's geometry; reproduce with `cargo test -p vapors-app --lib shape_store`"
    );
}

#[test]
fn a_mesh_item_drawn_before_its_upload_is_refused_naming_its_site() {
    // Given: registered, never uploaded (no painter frame ran)
    let triangles = triangle_mesh();
    let store = store(vec![entry("a.wisp:9", b"FM", Geometry::Mesh { triangles })]);
    let mut mesh = item(0, IDENTITY);
    mesh.kind = ShapeKind::Mesh;

    // When
    let refused = store.mesh_entries(&[mesh]).expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape 0, the mesh at a.wisp:9, has no vertices on the GPU; draw through ScenePainter, which uploads every registered entry before a frame; reproduce with `cargo test -p vapors-app --test gpu_shapes`"
    );
}
