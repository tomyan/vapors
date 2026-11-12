//! The order changed owners settle in (`isolated.rs`): a changed
//! owner above another lays its subtree out in isolation, which places
//! the owner below; settled after that, the lower owner's `Same` would
//! put its old layout back. Block boxes, as a page's are: a block parent
//! asks a child once, with `PerformLayout`, so the parent's answer is
//! cached and the root's layout does not ask it again.

use super::tests::{Texts, layouts, rebuild_box};
use super::*;
use crate::layout::slots::tests::{TEXT, tree};
use crate::layout::{Content, Placed};
use taffy::Display;

/// Root 1 holding body 2, which holds rows 3 and 4, each with a text
/// run: slots 0 to 5, the body at slot 1, row 3's text at slot 3, row
/// 4 at slot 4.
const BODY: usize = 1;
const FIRST_ROW: usize = 2;
const FIRST_TEXT: usize = 3;
const LAST_ROW: usize = 4;

/// Wraps at the 200px the tests lay out in, so the row after it moves
/// down.
const LONGER: &str = "a text long enough to wrap onto a second line in the width laid out";

fn nested(first_text: &str) -> Placed {
    let spec: &[(u64, &[usize])] = &[
        (1, &[1]),
        (2, &[2, 4]),
        (3, &[3]),
        (TEXT, &[]),
        (4, &[5]),
        (TEXT, &[]),
    ];
    let mut boxes = tree(spec);
    boxes[FIRST_TEXT] = text_box(first_text);
    for element in [0, BODY, FIRST_ROW, LAST_ROW] {
        boxes[element].style.display = Display::Block;
    }
    Placed::default().slots.place(boxes)
}

/// A fresh block box for element `node` with `children` in `slot`;
/// what the old box held.
fn rebuild_block(placed: &mut Placed, slot: usize, node: u64, children: &[usize]) -> Before {
    let old = rebuild_box(placed, slot, node, children);
    placed.boxes[slot].style.display = Display::Block;
    old
}

fn text_box(text: &str) -> LayoutBox {
    let mut text_box = tree(&[(TEXT, &[])]).remove(0);
    if let Content::Text(run) = &mut text_box.content {
        run.text = text.to_owned();
    }
    text_box
}

#[test]
fn a_changed_row_under_a_changed_body_is_settled_before_the_body_places_it() {
    // Given: the body built again with its first row's text longer,
    // and its last row built again as it was; the body first
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(nested("text"), Vec::new());
    let body = Changed {
        slot: BODY,
        ancestors: vec![0],
        before: Some(rebuild_block(&mut placed, BODY, 2, &[FIRST_ROW, LAST_ROW])),
    };
    rebuild_block(&mut placed, FIRST_ROW, 3, &[FIRST_TEXT]);
    placed.boxes[FIRST_TEXT] = text_box(LONGER);
    let last_row = Changed {
        slot: LAST_ROW,
        ancestors: vec![BODY, 0],
        before: Some(rebuild_block(&mut placed, LAST_ROW, 4, &[5])),
    };
    let (fresh, _) = texts.lay_out(nested(LONGER), Vec::new());
    let moved = |placed: &Placed| placed.boxes[LAST_ROW].layout.location;
    assert_ne!(moved(&fresh), moved(&placed), "the last row moves down");

    // When
    let (placed, work) = texts.lay_out(placed, vec![body, last_row]);

    // Then: the last row answered as before, the body did not
    let expected = Settled {
        isolated: 1,
        resized: 1,
        ..Settled::default()
    };
    assert_eq!(work.settled, expected);
    assert_eq!(
        layouts(&placed),
        layouts(&fresh),
        "the layout of a fresh tree"
    );
}
