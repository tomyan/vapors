//! Random inserts, removes and resizes fold to what a sequential pass
//! over the children gives (`lane_tree_random_children.rs`), for block
//! margin sets (Taffy 0.15.1's running collapse) and for flex items
//! (sums and gaps).

use super::{Handle, LaneTree, Summary};
use children::{Child, MIXED, NEGATIVE, Random, block, block_starts, flex, flex_starts};

#[path = "lane_tree_random_children.rs"]
mod children;

/// Runs 2000 random edits on a tree of `leaf`, checking every start,
/// the order and an interval's children against `starts` after each.
fn check<S: Summary>(
    (seed, margins): (u64, (i32, i32)),
    gap: f64,
    leaf: fn(Child) -> S,
    starts: impl Fn(&[Child]) -> Vec<f64>,
) {
    let mut random = Random::new(seed, margins);
    let mut tree = LaneTree::new(gap);
    let mut children: Vec<(Handle, Child)> = Vec::new();
    for step in 0..2000 {
        edit(&mut random, &mut tree, &mut children, leaf);
        let plain: Vec<Child> = children.iter().map(|&(_, c)| c).collect();
        let expected = starts(&plain);
        check_starts(&tree, &children, &expected, (seed, step));
        check_between(&tree, &children, &expected, (seed, step));
    }
}

fn check_starts<S: Summary>(
    tree: &LaneTree<S>,
    children: &[(Handle, Child)],
    expected: &[f64],
    (seed, step): (u64, usize),
) {
    for (index, &(handle, _)) in children.iter().enumerate() {
        assert_eq!(
            tree.start(handle),
            expected[index],
            "seed {seed}, step {step}, child {index}"
        );
    }
    let order: Vec<Handle> = children.iter().map(|&(h, _)| h).collect();
    assert_eq!(tree.handles(), order, "seed {seed}, step {step}");
}

/// One edit: an insert at `at` (half the time), a remove, or a resize.
fn edit<S: Summary>(
    random: &mut Random,
    tree: &mut LaneTree<S>,
    children: &mut Vec<(Handle, Child)>,
    leaf: fn(Child) -> S,
) {
    let at = random.below(children.len() + 1);
    let last = at.min(children.len().saturating_sub(1));
    match random.below(10) {
        0..=4 => insert(random, tree, children, leaf, at),
        5..=7 if !children.is_empty() => tree.remove(children.remove(last).0),
        _ if !children.is_empty() => {
            let child = random.child();
            tree.set(children[last].0, leaf(child));
            children[last].1 = child;
        }
        _ => {}
    }
}

fn insert<S: Summary>(
    random: &mut Random,
    tree: &mut LaneTree<S>,
    children: &mut Vec<(Handle, Child)>,
    leaf: fn(Child) -> S,
    at: usize,
) {
    let child = random.child();
    let previous = at.checked_sub(1).map(|i| children[i].0);
    children.insert(at, (tree.insert_after(previous, leaf(child)), child));
}

/// The children `between` gives for an interval somewhere along the
/// lane and for the point at each of a few children's starts, where a
/// child that starts above its run's first child (negative margins, or
/// overflow above) is missed if the run's low bound is not kept: every
/// one whose box or overflow meets it, in order.
fn check_between<S: Summary>(
    tree: &LaneTree<S>,
    children: &[(Handle, Child)],
    starts: &[f64],
    (seed, step): (u64, usize),
) {
    let span = starts.last().copied().unwrap_or(0.0);
    let from = span * f64::from(u32::try_from(step % 97).expect("under 97")) / 97.0;
    let mut probes = vec![(from, from + 40.0)];
    probes.extend(starts.iter().step_by(7).map(|&start| (start, start)));
    for (from, to) in probes {
        let meets = |(index, &(handle, child)): (usize, &(Handle, Child))| {
            let (low, reach) = (starts[index] + child.low(), starts[index] + child.reach());
            (low <= to && reach >= from).then_some(handle)
        };
        let expected: Vec<Handle> = children.iter().enumerate().filter_map(meets).collect();
        assert_eq!(
            tree.between(from, to),
            expected,
            "seed {seed}, step {step}, {from}..{to}"
        );
    }
}

#[test]
fn block_margin_sets_fold_as_a_running_collapse_over_random_edits() {
    for seed in [1, 7, 42] {
        check((seed, MIXED), 0.0, block, block_starts);
    }
}

#[test]
fn block_margins_all_negative_fold_as_a_running_collapse() {
    check((5, NEGATIVE), 0.0, block, block_starts);
}

#[test]
fn flex_items_fold_as_sums_and_gaps_over_random_edits() {
    for seed in [3, 11, 99] {
        check((seed, MIXED), 2.5, flex, |children| {
            flex_starts(children, 2.5)
        });
    }
}
