use vapors_scene::Rect;

use super::NestedIndex;

/// A 100 x 40 clip at `top`.
fn clip(top: f32) -> Rect {
    Rect::new(0.0, top, 100.0, top + 40.0)
}

/// Containers 1 to 100, each 50 px below the last; container 200 with
/// no clip, 201 taller than the tall limit.
fn index() -> NestedIndex {
    let stacked = (1..=100u16).map(|slot| (usize::from(slot), Some(clip(50.0 * f32::from(slot)))));
    let apart = [(200, None), (201, Some(Rect::new(0.0, 0.0, 10.0, 5000.0)))];
    NestedIndex::of(stacked.chain(apart))
}

fn meeting(index: &NestedIndex, top: f32, bottom: f32) -> Vec<usize> {
    let mut found: Vec<usize> = index.meeting(Rect::new(0.0, top, 100.0, bottom)).collect();
    found.sort_unstable();
    found
}

#[test]
fn a_view_meets_only_the_containers_whose_clip_it_crosses_and_those_kept_apart() {
    // When: a view over containers 10 and 11 (500 to 590)
    let found = meeting(&index(), 520.0, 560.0);

    // Then
    assert_eq!(found, [10, 11, 200, 201]);
}

#[test]
fn a_clip_touching_the_view_at_an_edge_does_not_meet_it() {
    // When: container 10 ends at 540, container 11 starts at 550
    let found = meeting(&index(), 540.0, 550.0);

    // Then
    assert_eq!(found, [200, 201]);
}

#[test]
fn a_container_entered_again_is_found_where_its_clip_now_is() {
    // Given
    let mut index = index();

    // When: container 10 moves from 500 to 9000, below every other
    index.remove(10, Some(clip(500.0)));
    index.insert(10, Some(clip(9000.0)));

    // Then
    assert_eq!(meeting(&index, 510.0, 520.0), [200, 201]);
    assert_eq!(meeting(&index, 9010.0, 9020.0), [10, 200, 201]);
}

#[test]
fn an_index_patched_equals_one_made_afresh() {
    // Given
    let mut patched = index();

    // When: container 10 moves below every other, and the tall one
    // gets a short clip
    patched.remove(10, Some(clip(500.0)));
    patched.insert(10, Some(clip(9000.0)));
    patched.remove(201, Some(Rect::new(0.0, 0.0, 10.0, 5000.0)));
    patched.insert(201, Some(clip(0.0)));

    // Then
    let moved = (1..=100u16).map(|slot| {
        let top = if slot == 10 {
            9000.0
        } else {
            50.0 * f32::from(slot)
        };
        (usize::from(slot), Some(clip(top)))
    });
    let fresh = NestedIndex::of(moved.chain([(200, None), (201, Some(clip(0.0)))]));
    assert_eq!(patched, fresh);
}

#[test]
#[should_panic(expected = "scroll container 10 is not in its frame's index")]
fn taking_out_a_container_under_another_clip_is_refused_naming_it() {
    // Given
    let mut index = index();

    // When
    index.remove(10, Some(clip(999.0)));
}
