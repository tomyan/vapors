//! The store: a table's entries registered in order, each closed form
//! with its paint read from its canonical bytes; a bad table, an
//! unregistered ref and a mesh refused, named.

use vapors_renderer::ClosedForm;
use vapors_scene::ShapeRef;
use vapors_shapes::{
    FillRule, Geometry, IconPath, MeshVertex, Paint, ShapeEntry, ShapeTable, TABLE_FORMAT,
    hash_bytes, tessellate,
};

use super::{ShapeError, ShapeStore};

fn entry(site: &'static str, canonical: &'static [u8], geometry: Geometry) -> ShapeEntry {
    ShapeEntry {
        site,
        canonical,
        hash: hash_bytes(canonical),
        geometry,
    }
}

const CIRCLE: Geometry = Geometry::Ellipse {
    centre: [8.0, 8.0],
    radii: [4.0, 4.0],
};

const fn rect(radius: [f32; 2]) -> Geometry {
    Geometry::Rect {
        origin: [1.0, 2.0],
        size: [10.0, 6.0],
        radius,
    }
}

/// `entries` as a table that lives for the test binary's life.
fn table(entries: Vec<ShapeEntry>) -> ShapeTable {
    ShapeTable::new("demo", TABLE_FORMAT, Box::leak(entries.into_boxed_slice()))
}

/// The canonical bytes of a stroke of width 2: `S` and the width.
const STROKE_2: &[u8] = &[b'S', 0, 0, 0, 64, b'M'];

#[test]
fn a_tables_entries_are_registered_in_order_with_their_paint() {
    // Given
    let mut store = ShapeStore::default();
    let table = table(vec![
        entry("a.wisp:3", b"FM", rect([1.0, 1.0])),
        entry("a.wisp:5", STROKE_2, CIRCLE),
    ]);

    // When
    let refs = store.register(&table).expect("registered");

    // Then
    assert_eq!(refs, [ShapeRef(0), ShapeRef(1)]);
    let filled = ClosedForm::Rect {
        origin: [1.0, 2.0],
        size: [10.0, 6.0],
        radius: 1.0,
    };
    assert_eq!(store.closed_form(ShapeRef(0)), Ok((filled, None)));
    let circle = ClosedForm::Ellipse {
        centre: [8.0, 8.0],
        radii: [4.0, 4.0],
    };
    assert_eq!(store.closed_form(ShapeRef(1)), Ok((circle, Some(2.0))));
}

#[test]
fn a_second_table_takes_the_refs_after_the_first() {
    // Given
    let mut store = ShapeStore::default();
    store
        .register(&table(vec![entry("a.wisp:3", b"FM", CIRCLE)]))
        .expect("registered");

    // When
    let refs = store.register(&table(vec![entry("b.wisp:1", b"EM", CIRCLE)]));

    // Then
    assert_eq!(refs, Ok(vec![ShapeRef(1)]));
}

#[test]
fn a_table_its_check_refuses_registers_nothing() {
    // Given: an entry whose bytes begin with no paint
    let mut store = ShapeStore::default();
    let table = table(vec![entry("a.wisp:3", b"M0", CIRCLE)]);

    // When
    let refused = store.register(&table).expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape table of crate demo, entry at a.wisp:3: the canonical bytes begin [77, 48], not a paint (F, E, or S and a width); the table is emitted by vapors-compiler from the crate's templates, so fix the emitter"
    );
    assert_eq!(
        store.closed_form(ShapeRef(0)),
        Err(ShapeError::Unregistered {
            shape: ShapeRef(0),
            registered: 0
        })
    );
}

#[test]
fn a_ref_past_the_registered_entries_is_refused_naming_it() {
    // Given
    let mut store = ShapeStore::default();
    store
        .register(&table(vec![entry("a.wisp:3", b"FM", CIRCLE)]))
        .expect("registered");

    // When
    let refused = store.closed_form(ShapeRef(7)).expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape 7 is not registered in this painter's store, which holds 1; register its crate's table (ShapeStore::register) before drawing it"
    );
}

#[test]
fn a_mesh_entry_is_registered_but_refused_at_draw_naming_its_site() {
    // Given
    let mut store = ShapeStore::default();
    let triangles = triangle_mesh();
    let table = table(vec![entry("a.wisp:9", b"FM", Geometry::Mesh { triangles })]);
    store.register(&table).expect("registered");

    // When
    let refused = store.closed_form(ShapeRef(0)).expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape 0, the mesh at a.wisp:9, is not drawn until the mesh pipeline (one-scene S3a2); draw it as a closed form or wait for S3a2"
    );
}

/// A filled triangle's mesh, as the tessellator builds it.
fn triangle_mesh() -> &'static [[MeshVertex; 3]] {
    let path = IconPath::parse("M0 0L4 0L0 4z").expect("parses");
    let mesh =
        tessellate("a.wisp:9", &path, Paint::Fill(FillRule::NonZero), 0.05).expect("tessellated");
    Box::leak(mesh.triangles.into_boxed_slice())
}
