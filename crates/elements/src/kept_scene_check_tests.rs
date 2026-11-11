use vapors_scene::{Item, Rect, SurfaceId};

use super::slab_difference;
use crate::slab::Slabs;

/// A surface item naming surface id `id`.
fn surface_item(id: u32) -> Item {
    Item::Surface {
        surface: SurfaceId(id),
        rect: Rect::new(0.0, 0.0, 1.0, 1.0),
    }
}

/// Slabs holding surface ids 0 and 1.
fn two_surfaces() -> Slabs {
    let mut slabs = Slabs::default();
    for name in ["a", "b"] {
        slabs.surfaces.insert(name.to_owned());
    }
    slabs
}

#[test]
fn each_held_id_named_once_passes() {
    // Given
    let (slabs, items) = (two_surfaces(), [surface_item(0), surface_item(1)]);

    // When
    let found = slab_difference(&slabs, [(3, &items[0]), (5, &items[1])].into_iter());

    // Then
    assert_eq!(found, None);
}

#[test]
fn an_id_named_by_two_parts_is_caught() {
    // Given: slots 3 and 5 both name surface 0
    let (slabs, items) = (two_surfaces(), [surface_item(0), surface_item(1)]);
    let parts = [(3, &items[0]), (5, &items[0]), (5, &items[1])];

    // When
    let found = slab_difference(&slabs, parts.into_iter());

    // Then
    assert_eq!(
        found.as_deref(),
        Some("slab id Surface(0) is named by slots [3, 5], not one")
    );
}

#[test]
fn a_free_id_named_is_caught() {
    // Given: surface 1 freed, yet slot 5 names it
    let (mut slabs, items) = (two_surfaces(), [surface_item(0), surface_item(1)]);
    slabs.surfaces.remove(1);

    // When
    let found = slab_difference(&slabs, [(3, &items[0]), (5, &items[1])].into_iter());

    // Then
    assert_eq!(
        found.as_deref(),
        Some("slab id Surface(1) is free, yet named by slots [5]")
    );
}

#[test]
fn a_held_id_no_part_names_is_caught() {
    // Given
    let (slabs, items) = (two_surfaces(), [surface_item(0)]);

    // When
    let found = slab_difference(&slabs, [(3, &items[0])].into_iter());

    // Then
    assert_eq!(
        found.as_deref(),
        Some("slab id Surface(1) is named by slots [], not one")
    );
}
