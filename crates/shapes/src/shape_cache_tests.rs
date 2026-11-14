//! The run-time cache: one entry per canonical form, fine enough for
//! the scale asked; a closed form tessellates nothing; a hash hit with
//! other bytes is a new entry; a bad scale is refused naming the site.

use super::*;
use crate::canonical::FillRule;
use crate::geometry::Geometry;

const STROKE: Paint = Paint::Stroke { width: 1.5 };

fn path(data: &str) -> IconPath {
    IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"))
}

fn entry(cache: &mut ShapeCache, data: &str, paint: Paint, scale: f32) -> u32 {
    cache
        .entry("a.wisp:2", &path(data), paint, scale)
        .unwrap_or_else(|e| panic!("{data:?}: {e}"))
}

#[test]
fn a_path_drawn_twice_is_one_entry_tessellated_once() {
    // Given
    let mut cache = ShapeCache::default();

    // When
    let first = entry(&mut cache, "M2 8h11M9 4l4 4-4 4", STROKE, 1.0);
    let second = entry(&mut cache, "M2 8h11M9 4l4 4-4 4", STROKE, 1.0);

    // Then
    assert_eq!((first, second), (0, 0));
    assert_eq!(cache.shapes().len(), 1);
    assert_eq!(cache.take_tessellated(), 1);
    assert_eq!(cache.take_tessellated(), 0);
}

#[test]
fn a_rect_is_a_closed_form_and_tessellates_nothing() {
    // Given
    let mut cache = ShapeCache::default();

    // When
    let index = entry(
        &mut cache,
        "M3 4h10v8h-10z",
        Paint::Fill(FillRule::NonZero),
        2.0,
    );

    // Then
    assert!(matches!(
        cache.shapes()[index as usize].geometry(),
        Geometry::Rect { .. }
    ));
    assert_eq!(cache.take_tessellated(), 0);
}

#[test]
fn another_paint_is_another_entry() {
    // Given
    let mut cache = ShapeCache::default();

    // When
    let thin = entry(&mut cache, "M2 8h12", STROKE, 1.0);
    let wide = entry(&mut cache, "M2 8h12", Paint::Stroke { width: 3.0 }, 1.0);

    // Then
    assert_ne!(thin, wide);
}

#[test]
fn a_larger_scale_tessellates_finer_and_a_smaller_one_reuses_it() {
    // Given: an arc, flattened within 0.1 px at 1x
    let mut cache = ShapeCache::default();
    let arc = "M2 8a6 6 0 1 0 12 0";
    let at_1x = entry(&mut cache, arc, STROKE, 1.0);

    // When
    let at_4x = entry(&mut cache, arc, STROKE, 4.0);
    let at_2x = entry(&mut cache, arc, STROKE, 2.0);

    // Then
    assert_ne!(at_1x, at_4x);
    assert_eq!(at_2x, at_4x);
    assert_eq!(
        cache.shapes()[at_4x as usize].tolerance(),
        TOLERANCE_PX / 4.0
    );
    assert_eq!(cache.take_tessellated(), 2);
}

#[test]
fn a_hash_hit_with_other_bytes_is_a_new_entry() {
    // Given: entry 0 filed under the hash another path has, as a
    // collision would file it
    let mut cache = ShapeCache::default();
    entry(&mut cache, "M2 8h12", STROKE, 1.0);
    let other = Canonical::of("b.wisp:1", &path("M2 4h12"), STROKE).expect("canonical");
    cache.by_hash.insert(other.hash(), vec![0]);

    // When
    let index = entry(&mut cache, "M2 4h12", STROKE, 1.0);

    // Then
    assert_eq!(index, 1);
}

#[test]
fn a_scale_that_is_not_above_zero_is_refused_naming_the_site() {
    // Given
    let mut cache = ShapeCache::default();

    // When
    let refused = cache.entry("a.wisp:2", &path("M2 8h12"), STROKE, 0.0);

    // Then
    assert!(
        refused
            .as_ref()
            .is_err_and(|e| e.starts_with("the shape at a.wisp:2 is drawn at 0 device pixels")),
        "{refused:?}"
    );
}
