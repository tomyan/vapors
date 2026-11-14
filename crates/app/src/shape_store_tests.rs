//! The store: a table's entries registered in order, each drawn with
//! the paint its canonical bytes begin with; a bad table, an
//! unregistered ref, a mesh and a transform the pipeline refuses are
//! refused, named; a transform with no area draws nothing.

use vapors_renderer::ClosedFormShape;
use vapors_scene::{Color, Rect, Shape, ShapeRef};
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

const RECT: Geometry = Geometry::Rect {
    origin: [1.0, 2.0],
    size: [10.0, 6.0],
    radius: [1.0, 1.0],
};

const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `entries` as a table that lives for the test binary's life.
fn table(entries: Vec<ShapeEntry>) -> ShapeTable {
    ShapeTable::new("demo", TABLE_FORMAT, Box::leak(entries.into_boxed_slice()))
}

/// The canonical bytes of a stroke of width 2: `S` and the width.
const STROKE_2: &[u8] = &[b'S', 0, 0, 0, 64, b'M'];

/// An item drawing `shape` under `transform` in blue.
fn item(shape: u32, transform: [f32; 6]) -> Shape {
    Shape {
        shape: ShapeRef(shape),
        transform,
        colour: Color::from_rgba8(0, 0, 255, 255),
        bounds: Rect::new(0.0, 0.0, 16.0, 16.0),
    }
}

/// `geometry` as the pipeline draws it under the identity in blue.
fn drawn(geometry: &Geometry, stroke: Option<f32>) -> ClosedFormShape {
    let blue = Color::from_rgba8(0, 0, 255, 255);
    ClosedFormShape::new(geometry, stroke, IDENTITY, blue)
        .expect("drawable")
        .expect("has area")
}

/// A store with `entries` registered as one table.
fn store(entries: Vec<ShapeEntry>) -> ShapeStore {
    let mut store = ShapeStore::default();
    store.register(&table(entries)).expect("registered");
    store
}

#[test]
fn a_tables_entries_are_registered_in_order_and_drawn_with_their_paint() {
    // Given
    let mut store = ShapeStore::default();
    let table = table(vec![
        entry("a.wisp:3", b"FM", RECT),
        entry("a.wisp:5", STROKE_2, CIRCLE),
    ]);

    // When
    let refs = store.register(&table).expect("registered");
    let shapes = store.closed_forms(&[item(0, IDENTITY), item(1, IDENTITY)]);

    // Then
    assert_eq!(refs, [ShapeRef(0), ShapeRef(1)]);
    assert_eq!(
        shapes,
        Ok(vec![drawn(&RECT, None), drawn(&CIRCLE, Some(2.0))])
    );
}

#[test]
fn a_second_table_takes_the_refs_after_the_first() {
    // Given
    let mut store = store(vec![entry("a.wisp:3", b"FM", CIRCLE)]);

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
        store.closed_forms(&[item(0, IDENTITY)]),
        Err(ShapeError::Unregistered {
            shape: ShapeRef(0),
            registered: 0
        })
    );
}

#[test]
fn a_ref_past_the_registered_entries_is_refused_naming_it() {
    // Given
    let store = store(vec![entry("a.wisp:3", b"FM", CIRCLE)]);

    // When
    let refused = store
        .closed_forms(&[item(7, IDENTITY)])
        .expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape 7 is not registered in this painter's store, which holds 1; register its crate's table (ShapeStore::register) before drawing it"
    );
}

#[test]
fn a_mesh_entry_is_registered_but_refused_at_draw_naming_its_site() {
    // Given
    let triangles = triangle_mesh();
    let store = store(vec![entry("a.wisp:9", b"FM", Geometry::Mesh { triangles })]);

    // When
    let refused = store
        .closed_forms(&[item(0, IDENTITY)])
        .expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape 0, the mesh at a.wisp:9, is not drawn until the mesh pipeline (one-scene S3a2); draw it as a closed form or wait for S3a2"
    );
}

#[test]
fn a_transform_the_pipeline_refuses_is_refused_naming_the_shape() {
    // Given
    let store = store(vec![entry("a.wisp:3", b"FM", CIRCLE)]);

    // When
    let nan = [1.0, 0.0, 0.0, f32::NAN, 0.0, 0.0];
    let refused = store.closed_forms(&[item(0, nan)]).expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        "shape 0, the entry at a.wisp:3, cannot be drawn: transform [1.0, 0.0, 0.0, NaN, 0.0, 0.0] is not finite; give the element a transform whose values and scale are finite; reproduce with `cargo test -p vapors-renderer --lib closed_form_shape`"
    );
}

#[test]
fn an_item_scaled_to_nothing_draws_nothing() {
    // Given
    let store = store(vec![entry("a.wisp:3", b"FM", CIRCLE)]);

    // When: CSS's scale(0) between two drawn items
    let shapes = store.closed_forms(&[item(0, IDENTITY), item(0, [0.0; 6]), item(0, IDENTITY)]);

    // Then
    assert_eq!(shapes.map(|s| s.len()), Ok(2));
}

/// A filled triangle's mesh, as the tessellator builds it.
fn triangle_mesh() -> &'static [[MeshVertex; 3]] {
    let path = IconPath::parse("M0 0L4 0L0 4z").expect("parses");
    let mesh =
        tessellate("a.wisp:9", &path, Paint::Fill(FillRule::NonZero), 0.05).expect("tessellated");
    Box::leak(mesh.triangles.into_boxed_slice())
}

#[test]
fn an_item_whose_bounds_miss_what_it_draws_is_refused_naming_both() {
    // Given: the circle (4 to 12 across) moved 10 right, its bounds left
    // where the entry is unmoved
    let store = store(vec![entry("a.wisp:3", b"FM", CIRCLE)]);
    let mut moved = item(0, [1.0, 0.0, 0.0, 1.0, 10.0, 0.0]);
    moved.bounds = Rect::new(4.0, 4.0, 12.0, 12.0);

    // When
    let refused = store.closed_forms(&[moved]).expect_err("refused");

    // Then: drawn 14 to 22 with the ramp's half pixel either side
    assert_eq!(
        refused.to_string(),
        "shape 0, the entry at a.wisp:3, draws over Rect { left: 13.5, top: 3.5, right: 22.5, bottom: 12.5 } but its item's bounds are Rect { left: 4.0, top: 4.0, right: 12.0, bottom: 12.0 }, so culling would drop pixels it draws; give the item bounds that hold its transformed entry"
    );
}

#[test]
fn bounds_that_hold_the_stroked_entry_under_its_transform_pass() {
    // Given: the circle stroked 2 wide, scaled 2: 6 to 26 across, and
    // bounds exactly that (the fringe holds the ramp)
    let store = store(vec![entry("a.wisp:5", STROKE_2, CIRCLE)]);
    let mut scaled = item(0, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    scaled.bounds = Rect::new(6.0, 6.0, 26.0, 26.0);

    // When, Then
    assert_eq!(store.closed_forms(&[scaled]).map(|s| s.len()), Ok(1));
}

#[test]
fn bounds_that_miss_only_the_stroke_are_refused() {
    // Given: the circle (4 to 12) stroked 2 wide draws 3 to 13, and 2.5
    // to 13.5 with the ramp; bounds of 3.6 to 12.4 hold the fill and
    // ramp (2.6 to 13.4 with the fringe) but not the stroke
    let store = store(vec![entry("a.wisp:5", STROKE_2, CIRCLE)]);
    let mut stroked = item(0, IDENTITY);
    stroked.bounds = Rect::new(3.6, 3.6, 12.4, 12.4);

    // When
    let refused = store.closed_forms(&[stroked]);

    // Then
    assert!(
        matches!(refused, Err(ShapeError::OutsideBounds { .. })),
        "{refused:?}"
    );
}
