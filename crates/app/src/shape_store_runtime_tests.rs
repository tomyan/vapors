//! A document's run-time shapes in the store: taken past those held,
//! named by run-time refs, drawn by the same paths as a table's; another
//! document's cache, a ref the store did not issue and meshes past the
//! buffer's limit are refused, named.

use vapors_scene::{Color, Rect, Shape, ShapeKind, ShapeRef};
use vapors_shapes::{FillRule, IconPath, Paint, ShapeCache};

use super::{ShapeError, ShapeStore};
use crate::shape_store::tests::{CIRCLE, IDENTITY, entry, store};

const FILL: Paint = Paint::Fill(FillRule::NonZero);

/// A cache holding a triangle (a mesh) and a rect (a closed form).
fn cache() -> ShapeCache {
    let mut cache = ShapeCache::default();
    for data in ["M0 0L4 0L0 4z", "M3 4h10v8h-10z"] {
        let path = IconPath::parse(data).expect("parses");
        cache
            .entry("page.wisp:2", &path, FILL, 1.0)
            .expect("cached");
    }
    cache
}

fn item(shape: ShapeRef, kind: ShapeKind) -> Shape {
    let blue = Color::from_rgba8(0, 0, 255, 255);
    Shape::new(shape, kind, IDENTITY, blue, Rect::new(0.0, 0.0, 16.0, 16.0))
}

#[test]
fn a_documents_shapes_are_drawn_by_their_runtime_refs() {
    // Given
    let (mut store, cache) = (ShapeStore::default(), cache());

    // When
    store.take_runtime(cache.shapes()).expect("taken");

    // Then
    let mesh = item(ShapeRef::runtime(0), ShapeKind::Mesh);
    let rect = item(ShapeRef::runtime(1), ShapeKind::ClosedForm);
    assert_eq!(store.mesh_instances(&[mesh]).map(|(_, at)| at), Ok(vec![0]));
    assert_eq!(store.closed_forms(&[rect]).map(|drawn| drawn.len()), Ok(1));
}

#[test]
fn shapes_are_taken_once_and_later_ones_after() {
    // Given: a store that took the cache's two
    let (mut store, mut cache) = (ShapeStore::default(), cache());
    store.take_runtime(cache.shapes()).expect("taken");

    // When: taken again, then with a third
    store.take_runtime(cache.shapes()).expect("taken again");
    let path = IconPath::parse("M0 0h4v2z").expect("parses");
    cache
        .entry("page.wisp:5", &path, FILL, 1.0)
        .expect("cached");
    store.take_runtime(cache.shapes()).expect("the third");

    // Then
    assert_eq!(store.runtime.len(), 3);
    let third = item(ShapeRef::runtime(2), ShapeKind::Mesh);
    assert_eq!(
        store.mesh_instances(&[third]).map(|(_, at)| at),
        Ok(vec![2])
    );
}

#[test]
fn another_documents_cache_is_refused() {
    // Given
    let mut store = ShapeStore::default();
    store.take_runtime(cache().shapes()).expect("taken");

    // When
    let refused = store.take_runtime(cache().shapes());

    // Then
    assert_eq!(
        refused,
        Err(ShapeError::OtherDocument { held: 2, given: 2 })
    );
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("a ScenePainter of its own")
    );
}

#[test]
fn a_table_ref_at_a_runtime_entrys_index_is_refused() {
    // Given: the run-time triangle at entry 1, after a table's entry 0
    let mut store = store(vec![entry("a.wisp:3", b"FN", CIRCLE)]);
    store.take_runtime(cache().shapes()).expect("taken");

    // When
    let forged = item(ShapeRef(1), ShapeKind::Mesh);

    // Then
    assert_eq!(
        store.mesh_instances(&[forged]).map(|_| ()),
        Err(ShapeError::Unregistered {
            shape: ShapeRef(1),
            registered: 3
        })
    );
}

#[test]
fn run_time_meshes_past_the_buffers_limit_are_refused_naming_the_document() {
    // Given: a buffer of 16 bytes, less than one triangle
    let mut store = ShapeStore::with_buffer_limit(16);

    // When
    let refused = store.take_runtime(cache().shapes()).expect_err("refused");

    // Then
    assert!(
        refused
            .to_string()
            .starts_with("the document's run-time shapes would put"),
        "{refused}"
    );
    assert_eq!(store.runtime, Vec::<usize>::new());
}
