//! The run-time cache: one live entry per canonical form and scale
//! step, counted per use; the last release frees the slot for reuse
//! under a new generation; the changed slots are taken once; a bad or
//! too-large scale is refused naming the site.

use super::*;
use crate::canonical::FillRule;
use crate::geometry::Geometry;

const STROKE: Paint = Paint::Stroke { width: 1.5 };
const ARROW: &str = "M2 8h11M9 4l4 4-4 4";

fn path(data: &str) -> IconPath {
    IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"))
}

fn acquire(cache: &mut ShapeCache, data: &str, paint: Paint, scale: f32) -> CacheRef {
    cache
        .acquire("a.wisp:2", &path(data), paint, scale)
        .unwrap_or_else(|e| panic!("{data:?}: {e}"))
}

#[test]
fn a_path_used_twice_is_one_entry_tessellated_once() {
    // Given
    let mut cache = ShapeCache::new(1);

    // When
    let first = acquire(&mut cache, ARROW, STROKE, 1.0);
    let second = acquire(&mut cache, ARROW, STROKE, 1.0);

    // Then
    assert_eq!(first, second);
    assert_eq!(cache.counts(), (1, 1));
    assert_eq!(cache.take_tessellated(), 1);
}

#[test]
fn the_last_release_frees_the_slot_and_its_reuse_has_a_new_generation() {
    // Given: one entry used twice
    let mut cache = ShapeCache::new(1);
    let first = acquire(&mut cache, ARROW, STROKE, 1.0);
    acquire(&mut cache, ARROW, STROKE, 1.0);

    // When: released twice, then another path
    cache.release(first);
    assert!(cache.get(first).is_some(), "one use left");
    cache.release(first);
    let other = acquire(&mut cache, "M2 2h12", STROKE, 1.0);

    // Then: the slot is reused, and the old ref names nothing
    assert_eq!(
        (other.index, other.generation),
        (first.index, first.generation + 1)
    );
    assert!(cache.get(first).is_none());
    assert_eq!(cache.counts(), (1, 1));
}

#[test]
#[should_panic(expected = "which is not a live entry of document 1")]
fn releasing_a_freed_entry_is_a_fault() {
    let mut cache = ShapeCache::new(1);
    let at = acquire(&mut cache, ARROW, STROKE, 1.0);
    cache.release(at);
    cache.release(at);
}

#[test]
fn the_changed_slots_are_taken_once_each_with_their_state() {
    // Given: an entry made, then another made and freed
    let mut cache = ShapeCache::new(1);
    let kept = acquire(&mut cache, ARROW, STROKE, 1.0);
    let gone = acquire(&mut cache, "M2 2h12", STROKE, 1.0);
    cache.release(gone);

    // When
    let changes = cache.take_changes();

    // Then
    let states: Vec<_> = changes
        .iter()
        .map(|(index, now)| (*index, now.as_ref().map(|(g, _)| *g)))
        .collect();
    assert_eq!(states, [(kept.index, Some(0)), (gone.index, None)]);
    assert_eq!(cache.take_changes().len(), 0);
}

#[test]
fn a_rect_is_a_closed_form_shared_by_every_scale_step() {
    // Given
    let mut cache = ShapeCache::new(1);
    let fill = Paint::Fill(FillRule::NonZero);

    // When
    let small = acquire(&mut cache, "M3 4h10v8h-10z", fill, 1.0);
    let large = acquire(&mut cache, "M3 4h10v8h-10z", fill, 64.0);

    // Then
    assert_eq!(small, large);
    assert!(matches!(
        cache.get(small).expect("live").geometry(),
        Geometry::Rect { .. }
    ));
    assert_eq!(cache.take_tessellated(), 0);
}

#[test]
fn scales_within_one_power_of_two_share_an_entry() {
    // Given: an arc at 3x, which rounds up to the step 4
    let mut cache = ShapeCache::new(1);
    let arc = "M2 8a6 6 0 1 0 12 0";
    let three = acquire(&mut cache, arc, STROKE, 3.0);

    // When
    let four = acquire(&mut cache, arc, STROKE, 4.0);
    let five = acquire(&mut cache, arc, STROKE, 5.0);

    // Then
    assert_eq!(three, four);
    assert_ne!(four, five);
    assert_eq!(
        cache.get(three).expect("live").tolerance(),
        TOLERANCE_PX / 4.0
    );
    assert_eq!(cache.take_tessellated(), 2);
}

#[test]
fn a_hash_hit_with_other_bytes_is_a_new_entry() {
    // Given: entry 0 filed under the hash another path has, as a
    // collision would file it
    let mut cache = ShapeCache::new(1);
    acquire(&mut cache, "M2 8h12", STROKE, 1.0);
    let other = Canonical::of("b.wisp:1", &path("M2 4h12"), STROKE).expect("canonical");
    cache.by_hash.insert(other.hash(), vec![0]);

    // When
    let at = acquire(&mut cache, "M2 4h12", STROKE, 1.0);

    // Then
    assert_eq!(at.index, 1);
}

#[test]
fn a_scale_not_above_zero_or_past_the_cap_is_refused_naming_the_site() {
    for (scale, says) in [
        (0.0, "not a finite number above zero"),
        (8192.0, "past the 4096"),
    ] {
        // Given
        let mut cache = ShapeCache::new(1);

        // When
        let refused = cache.acquire("a.wisp:2", &path("M2 8h12"), STROKE, scale);

        // Then
        let reason = refused.expect_err("refused");
        assert!(
            reason.starts_with("the shape at a.wisp:2 is drawn at") && reason.contains(says),
            "{reason}"
        );
        assert_eq!(cache.counts(), (0, 0));
    }
}

#[test]
fn a_failed_tessellation_counts_nothing() {
    // Given: a stroke the tessellator refuses (a width of no length is
    // refused by the canonical form first; a path of no area by lyon)
    let mut cache = ShapeCache::new(1);

    // When
    let refused = cache.acquire(
        "a.wisp:2",
        &path("M2 2L2 2"),
        Paint::Fill(FillRule::NonZero),
        1.0,
    );

    // Then
    assert!(refused.is_err());
    assert_eq!(cache.take_tessellated(), 0);
}
