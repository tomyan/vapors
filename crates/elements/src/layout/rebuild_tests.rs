use taffy::{LayoutInput, LayoutOutput, RunMode};
use vapors_dom::NodeId;

use super::*;
use crate::layout::LayoutBox;
use crate::layout::slots::tests::tree;

/// A cache entry, a laid-out width and a hoisted box, as a box keeps
/// after layout.
fn laid_out(layout_box: &mut LayoutBox) {
    let input = LayoutInput {
        run_mode: RunMode::PerformLayout,
        ..LayoutInput::HIDDEN
    };
    layout_box.cache.store(&input, LayoutOutput::HIDDEN);
    layout_box.layout.size.width = 10.0;
    layout_box.hoisted.push(1);
    assert!(!layout_box.cache.is_empty(), "the cache holds the entry");
}

/// Nodes 1 > 2 > 3, each box laid out.
fn chain() -> Placed {
    let mut placed = Placed::default()
        .slots
        .place(tree(&[(1, &[1]), (2, &[2]), (3, &[])]));
    placed.boxes.iter_mut().for_each(laid_out);
    placed
}

#[test]
fn clearing_ancestors_clears_their_caches_and_hoisted_lists_and_not_the_box() {
    // Given
    let mut placed = chain();

    // When: node 3's ancestors, nodes 2 and 1
    clear_ancestors(&mut placed, [NodeId(2), NodeId(1)].into_iter());

    // Then
    for ancestor in [0, 1] {
        assert!(placed.boxes[ancestor].cache.is_empty(), "slot {ancestor}");
        assert!(placed.boxes[ancestor].hoisted.is_empty(), "slot {ancestor}");
    }
    assert!(!placed.boxes[2].cache.is_empty());
    assert_eq!(placed.boxes[2].hoisted, [1]);
}

/// Builds node 2's subtree again in `placed` as `built`.
fn rebuilt_at_node_2(placed: &mut Placed, built: &[(u64, &[usize])]) -> Taken {
    let mut taken = Taken::default();
    placed.take_subtree(1, &mut taken);
    placed.put_subtree(tree(built), &mut taken);
    placed.free_taken(&taken);
    taken
}

#[test]
fn a_new_key_in_a_freed_slot_starts_with_an_empty_cache_layout_and_hoisted_list() {
    // Given: node 3's laid-out box leaves slot 2, then node 4 takes it
    let mut placed = chain();
    rebuilt_at_node_2(&mut placed, &[(2, &[])]);
    let taken = rebuilt_at_node_2(&mut placed, &[(2, &[1]), (4, &[])]);
    assert_eq!(placed.slots.of(Key::Element(NodeId(4))), Some(2));

    // When
    lend_subtree(&mut placed, 1, &taken, &|_| true);

    // Then
    let fresh = &placed.boxes[2];
    assert!(fresh.cache.is_empty(), "cache");
    assert!(fresh.hoisted.is_empty(), "hoisted");
    assert!(fresh.layout.size.width == 0.0, "layout");
}
