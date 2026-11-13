//! Thumb order on pages of many scroll containers: the cost of
//! placing them, and their order when many share one target.

use super::{positioned, scrolling, steps};
use crate::paint_order::{Stacking, Step};

/// Parents of a chain of `depth` boxes under the root, each with a leaf
/// child: box `1 + 2i` is the chain's i-th box, `2 + 2i` its leaf.
fn chain_with_leaves(depth: usize) -> Vec<Option<usize>> {
    let mut parents = vec![None];
    for level in 0..depth {
        let above = if level == 0 { 0 } else { 2 * level - 1 };
        parents.push(Some(above));
        parents.push(Some(2 * level + 1));
    }
    parents
}

#[test]
fn each_box_is_marked_at_most_once_however_deep_the_scrollers_nest() {
    // Given: 200 nested scroll containers, each with a leaf; walking up
    // from every leaf to the root would mark 20,000 times
    let parents = chain_with_leaves(200);
    let scrollers: Vec<(usize, Stacking)> = (0..200).map(|i| (2 * i + 1, scrolling())).collect();
    crate::thumb_order::MARKS.with(|marks| marks.set(0));

    // When
    steps(&parents, &scrollers);

    // Then
    let marked = crate::thumb_order::MARKS.with(std::cell::Cell::get);
    assert!(
        marked <= parents.len(),
        "{marked} marks for {} boxes",
        parents.len()
    );
}

#[test]
fn many_thumbs_moved_after_one_box_keep_their_order_among_out_of_order_targets() {
    // Given: root > 1 (scrolls) > 2 > ... > 26 (each scrolls) > 27
    // (relative, painted last); root > 28..=37 (each scrolls) > a leaf.
    // Every thumb of 1..=26 moves after 27; the later siblings' thumbs
    // stay before it, so the targets are not in order.
    let mut parents: Vec<Option<usize>> = vec![None, Some(0)];
    parents.extend((2..=27).map(|slot| Some(slot - 1)));
    parents.extend((28..=37).map(|_| Some(0)));
    parents.extend((28..=37).map(Some));
    let mut stacking: Vec<(usize, Stacking)> =
        (1..=26).chain(28..=37).map(|s| (s, scrolling())).collect();
    stacking.push((27, positioned(None)));

    // When
    let found = steps(&parents, &stacking);

    // Then: 27, then the thumbs innermost first
    let expected: Vec<Step> = std::iter::once(Step::Box(27))
        .chain((1..=26).rev().map(Step::Thumb))
        .collect();
    assert_eq!(found[found.len() - expected.len()..], expected[..]);
}
