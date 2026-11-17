//! The store's mesh side: an item whose kind is not its entry's, either
//! way round, and a mesh drawn before its triangles are uploaded, each
//! refused naming the entry.

use vapors_scene::ShapeKind;
use vapors_shapes::Geometry;

use crate::shape_store::tests::{CIRCLE, IDENTITY, entry, item_of, store, triangle_mesh};

#[test]
fn an_item_whose_kind_is_not_its_entrys_is_refused_naming_both() {
    // Given: a mesh entry, and a closed form
    let triangles = triangle_mesh();
    let store = store(vec![
        entry("a.wisp:9", b"FM", Geometry::Mesh { triangles }),
        entry("a.wisp:3", b"FC", CIRCLE),
    ]);
    let mesh_said_closed = item_of(0, ShapeKind::ClosedForm, IDENTITY);
    let closed_said_mesh = item_of(1, ShapeKind::Mesh, IDENTITY);

    // When
    let as_closed = store
        .closed_forms(&[mesh_said_closed])
        .expect_err("refused");
    let as_mesh = store
        .mesh_instances(&[closed_said_mesh], &[])
        .map(|_| ())
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
    let mesh = item_of(0, ShapeKind::Mesh, IDENTITY);

    // When
    let indices = store
        .mesh_instances(&[mesh], &[])
        .expect("drawable")
        .entries;
    let refused = store.uploaded(&indices).expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape 0, the mesh at a.wisp:9, has no triangles on the GPU: their upload this frame was refused, and the frame's encoder holds why; the store refuses a table its buffer cannot hold, so report this with the frame's error; reproduce with `cargo test -p vapors-app --lib shape_store_meshes`"
    );
}

#[test]
fn a_table_whose_meshes_pass_the_entry_buffers_limit_is_refused_naming_its_crate() {
    // Given: a store whose entry buffer holds one byte less than the
    // triangle's mesh
    let triangles = triangle_mesh();
    let bytes = vapors_renderer::mesh_entry_bytes(triangles.len());
    let mut store = crate::ShapeStore::with_buffer_limit(bytes - 1);
    let mesh = crate::shape_store::tests::table(vec![entry(
        "a.wisp:9",
        b"FM",
        Geometry::Mesh { triangles },
    )]);

    // When
    let refused = store.register(&mesh).expect_err("refused");

    // Then: nothing is registered
    assert_eq!(
        refused.to_string(),
        format!(
            "shape table of crate demo would put {bytes} bytes of mesh triangles in the painter's entry buffer, over the device's max_buffer_size of {}; draw fewer or smaller meshes, or draw some with a second ScenePainter; reproduce with `cargo test -p vapors-app --lib shape_store_meshes`",
            bytes - 1
        )
    );
    let circle = crate::shape_store::tests::table(vec![entry("b.wisp:1", b"FC", CIRCLE)]);
    assert_eq!(
        store.register(&circle),
        Ok(vec![vapors_scene::ShapeRef::Table(0)])
    );
}

#[test]
fn a_mesh_items_bounds_that_miss_its_triangles_are_refused() {
    // Given: the triangle from (0, 0) to (4, 4), its item's bounds 3 wide
    let store = store(vec![entry(
        "a.wisp:9",
        b"FM",
        Geometry::Mesh {
            triangles: triangle_mesh(),
        },
    )]);
    let mut short = item_of(0, ShapeKind::Mesh, IDENTITY);
    short.bounds = vapors_scene::Rect::new(0.0, 0.0, 3.0, 4.0);

    // When
    let refused = store
        .mesh_instances(&[short], &[])
        .map(|_| ())
        .expect_err("refused");

    // Then
    assert!(
        matches!(refused, crate::ShapeError::OutsideBounds { .. }),
        "{refused}"
    );
}

#[test]
fn an_item_built_from_its_entry_takes_the_entrys_kind() {
    // Given
    let mesh = entry(
        "a.wisp:9",
        b"FM",
        Geometry::Mesh {
            triangles: triangle_mesh(),
        },
    );
    let circle = entry("a.wisp:3", b"FC", CIRCLE);
    let at = |entry| {
        let bounds = vapors_scene::Rect::new(0.0, 0.0, 16.0, 16.0);
        let colour = vapors_scene::Color::from_rgba8(0, 0, 0, 255);
        crate::shape_item(
            entry,
            vapors_scene::ShapeRef::Table(0),
            IDENTITY,
            colour,
            bounds,
        )
        .kind()
    };

    // When, Then
    assert_eq!(at(&mesh), ShapeKind::Mesh);
    assert_eq!(at(&circle), ShapeKind::ClosedForm);
}
