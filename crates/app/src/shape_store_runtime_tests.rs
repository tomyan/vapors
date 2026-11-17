//! A document's run-time shapes in the store: each changed slot taken
//! as it is now, named by its document, slot and generation; a freed
//! slot's ref refused, another document's changes refused, and meshes
//! past the buffer's limit refused, each named.

use vapors_scene::{Color, Rect, RuntimeRef, Shape, ShapeKind, ShapeRef};
use vapors_shapes::{CacheRef, FillRule, IconPath, Paint, ShapeCache};

use super::{ShapeError, ShapeStore};
use crate::shape_store::tests::IDENTITY;

const FILL: Paint = Paint::Fill(FillRule::NonZero);

fn acquire(cache: &mut ShapeCache, data: &str) -> CacheRef {
    let path = IconPath::parse(data).expect("parses");
    cache
        .acquire("page.wisp:2", &path, FILL, 1.0)
        .expect("cached")
}

fn item(cache: &ShapeCache, at: CacheRef, kind: ShapeKind) -> Shape {
    let blue = Color::from_rgba8(0, 0, 255, 255);
    let shape = ShapeRef::Runtime(RuntimeRef {
        document: cache.document(),
        index: at.index,
        generation: at.generation,
    });
    Shape::new(shape, kind, IDENTITY, blue, Rect::new(0.0, 0.0, 16.0, 16.0))
}

/// The store with `cache`'s changes taken.
fn take(store: &mut ShapeStore, cache: &mut ShapeCache) -> Result<(), ShapeError> {
    store
        .take_runtime(cache.document(), take_changes(cache))
        .map(|_| ())
}

#[test]
fn a_documents_shapes_are_drawn_by_their_runtime_refs() {
    // Given: a triangle (a mesh) and a rect (a closed form)
    let (mut store, mut cache) = (ShapeStore::default(), ShapeCache::new(7));
    let triangle = acquire(&mut cache, "M0 0L4 0L0 4z");
    let rect = acquire(&mut cache, "M3 4h10v8h-10z");

    // When
    take(&mut store, &mut cache).expect("taken");

    // Then
    let mesh = item(&cache, triangle, ShapeKind::Mesh);
    let closed = item(&cache, rect, ShapeKind::ClosedForm);
    assert!(store.mesh_instances(&[mesh], &[]).is_ok());
    assert_eq!(
        store.closed_forms(&[closed]).map(|drawn| drawn.len()),
        Ok(1)
    );
}

#[test]
fn a_freed_entrys_ref_is_refused_and_its_slots_new_entry_drawn() {
    // Given: the triangle taken, then freed and its slot reused
    let (mut store, mut cache) = (ShapeStore::default(), ShapeCache::new(7));
    let old = acquire(&mut cache, "M0 0L4 0L0 4z");
    take(&mut store, &mut cache).expect("taken");
    cache.release(old);
    let new = acquire(&mut cache, "M0 0L6 0L0 6z");
    take(&mut store, &mut cache).expect("taken again");

    // When
    let stale = item(&cache, old, ShapeKind::Mesh);
    let fresh = item(&cache, new, ShapeKind::Mesh);

    // Then
    assert_eq!(new.index, old.index);
    assert!(matches!(
        store.mesh_instances(&[stale], &[]),
        Err(ShapeError::Unregistered { .. })
    ));
    assert!(store.mesh_instances(&[fresh], &[]).is_ok());
}

#[test]
fn another_documents_changes_are_refused() {
    // Given
    let mut store = ShapeStore::default();
    let mut first = ShapeCache::new(7);
    acquire(&mut first, "M0 0L4 0L0 4z");
    take(&mut store, &mut first).expect("taken");

    // When
    let mut second = ShapeCache::new(8);
    acquire(&mut second, "M0 0L4 0L0 4z");
    let refused = take(&mut store, &mut second);

    // Then
    assert_eq!(
        refused,
        Err(ShapeError::OtherDocument { held: 7, given: 8 })
    );
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("a ScenePainter of its own")
    );
}

#[test]
fn run_time_meshes_past_the_buffers_limit_are_refused_naming_the_document() {
    // Given: a buffer of 16 bytes, less than one triangle
    let (mut store, mut cache) = (ShapeStore::with_buffer_limit(16), ShapeCache::new(7));
    acquire(&mut cache, "M0 0L4 0L0 4z");

    // When
    let refused = take(&mut store, &mut cache).expect_err("refused");

    // Then
    assert!(
        refused
            .to_string()
            .starts_with("the document's run-time shapes would put"),
        "{refused}"
    );
    assert_eq!(store.document, None);
}

fn take_changes(cache: &mut ShapeCache) -> Vec<vapors_shapes::SlotChange> {
    let changes = cache.changes();
    cache.clear_changes();
    changes
}
