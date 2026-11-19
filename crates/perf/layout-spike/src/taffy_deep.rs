//! Option (b)'s deep tree: `levels` built from the last level up.

use taffy::prelude::*;

use crate::spec::{CHANGED_LEAF, DEEP_SIDE, LEAF, PAD};
use crate::taffy_engine::{Taffy, container, fixed, rest};

/// Level `level`'s container style: the root has the fixed size, the
/// others take the rest of the level above.
fn level_style(level: usize) -> Style {
    let direction = if level.is_multiple_of(2) {
        FlexDirection::Column
    } else {
        FlexDirection::Row
    };
    let mut style = if level == 0 {
        container(
            direction,
            Size {
                width: length(DEEP_SIDE),
                height: length(DEEP_SIDE),
            },
        )
    } else {
        rest()
    };
    style.display = Display::Flex;
    style.flex_direction = direction;
    let pad = length(PAD);
    style.padding = taffy::Rect {
        left: pad,
        right: pad,
        top: pad,
        bottom: pad,
    };
    style
}

/// Levels from the last up: each a leaf, then the level built before.
fn levels(tree: &mut TaffyTree<()>, depth: usize) -> (Vec<NodeId>, Vec<NodeId>) {
    let (mut nested, mut leaves, mut containers) = (None, vec![], vec![]);
    for level in (0..depth).rev() {
        let leaf = tree.new_leaf(fixed(LEAF, LEAF)).expect("leaf");
        let kids: Vec<NodeId> = [Some(leaf), nested].into_iter().flatten().collect();
        let node = tree
            .new_with_children(level_style(level), &kids)
            .expect("level");
        nested = Some(node);
        leaves.push(leaf);
        containers.push(node);
    }
    leaves.reverse();
    containers.reverse();
    (containers, leaves)
}

pub(crate) fn deep(depth: usize) -> Taffy {
    let mut tree = TaffyTree::new();
    let (containers, leaves) = levels(&mut tree, depth);
    let path = |level: usize| [&containers[..=level], &leaves[level..=level]].concat();
    let paths = [path(depth - 2), path(depth - 1)];
    let sizes = [
        fixed(LEAF, LEAF).size,
        fixed(CHANGED_LEAF, CHANGED_LEAF).size,
    ];
    Taffy {
        tree,
        root: containers[0],
        changed: leaves[depth - 2],
        paths,
        flipped: false,
        sizes,
    }
}
