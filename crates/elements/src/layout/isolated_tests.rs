use taffy::{Layout, RunMode};
use vapors_text::FontSystem;

use super::*;
use crate::layout::slots::tests::{TEXT, tree};
use crate::layout::{Calcs, Content, LayoutWork, Placed, TextCache, lay_out};

/// Root 1 holding rows 2, 3 and 4, each with a text run: slots 0 to 6,
/// row 3 at slot 3 with its text at slot 4.
pub(super) const ROW: usize = 3;
pub(super) const ROW_TEXT: usize = 4;

pub(super) fn rows() -> Placed {
    let spec: &[(u64, &[usize])] = &[
        (1, &[1, 3, 5]),
        (2, &[2]),
        (TEXT, &[]),
        (3, &[4]),
        (TEXT, &[]),
        (4, &[6]),
        (TEXT, &[]),
    ];
    Placed::default().slots.place(tree(spec))
}

pub(super) struct Texts(pub(super) FontSystem, pub(super) TextCache);

impl Texts {
    pub(super) fn new() -> Self {
        Self(FontSystem::bundle_only(), TextCache::default())
    }

    pub(super) fn lay_out(
        &mut self,
        placed: Placed,
        changed: Vec<Changed>,
    ) -> (Placed, LayoutWork) {
        let texts = (&mut self.0, &mut self.1);
        lay_out(
            placed,
            texts,
            1.0,
            (200.0, 100.0),
            &Calcs::default(),
            changed,
        )
        .unwrap_or_else(|failed| panic!("refused: {}", failed.0))
    }
}

/// Puts fresh boxes for row 3 and its text, the text `text`, in their
/// slots; what the row's old box held.
pub(super) fn rebuild_row(placed: &mut Placed, text: &str) -> Before {
    let mut row = tree(&[(3, &[1]), (TEXT, &[])]);
    row[0].children = vec![ROW_TEXT];
    if let Content::Text(run) = &mut row[1].content {
        run.text = text.to_owned();
    }
    let old = std::mem::replace(&mut placed.boxes[ROW], row.remove(0));
    placed.boxes[ROW_TEXT] = row.remove(0);
    Before::of(old)
}

/// Puts a fresh element box for `node` with `children` in `slot`; what
/// the old box held.
pub(super) fn rebuild_box(
    placed: &mut Placed,
    slot: usize,
    node: u64,
    children: &[usize],
) -> Before {
    let mut fresh = tree(&[(node, &[])]).remove(0);
    fresh.children = children.to_vec();
    Before::of(std::mem::replace(&mut placed.boxes[slot], fresh))
}

pub(super) fn changed_row(before: Option<Before>) -> Vec<Changed> {
    vec![Changed {
        slot: ROW,
        ancestors: vec![0],
        before,
    }]
}

pub(super) fn layouts(placed: &Placed) -> Vec<Layout> {
    placed.boxes.iter().map(|b| b.layout).collect()
}

pub(super) fn sorted(mut slots: Vec<usize>) -> Vec<usize> {
    slots.sort_unstable();
    slots.dedup();
    slots
}

#[test]
fn a_layout_records_the_inputs_each_box_was_asked_and_a_second_asks_nothing_new() {
    // Given
    let mut texts = Texts::new();
    let (placed, _) = texts.lay_out(rows(), Vec::new());
    let asked: Vec<usize> = placed.boxes.iter().map(|b| b.asked.len()).collect();

    // When: nothing changed, every box is a cache hit
    let (again, work) = texts.lay_out(placed, Vec::new());

    // Then
    assert!(
        asked.iter().all(|&n| n > 0),
        "every box was asked: {asked:?}"
    );
    assert_eq!(again.boxes[0].asked.len(), 1, "the root is asked once");
    assert_eq!(
        again.boxes[0].asked[0].input.run_mode,
        RunMode::PerformLayout,
        "and laid out"
    );
    let after: Vec<usize> = again.boxes.iter().map(|b| b.asked.len()).collect();
    assert_eq!((after, work.laid_out), (asked, 0));
}

#[test]
fn a_rebuilt_row_that_answers_as_before_is_laid_out_alone_and_the_root_keeps_its_cache() {
    // Given: row 3 built again with the same text
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(rows(), Vec::new());
    let before = layouts(&placed);
    let old = rebuild_row(&mut placed, "text");
    let asked = old.asked.clone();

    // When
    let (placed, work) = texts.lay_out(placed, changed_row(Some(old)));

    // Then
    let expected = Settled {
        isolated: 1,
        ..Settled::default()
    };
    assert_eq!(work.settled, expected);
    assert_eq!(
        sorted(work.written.laid_out),
        [ROW, ROW_TEXT],
        "only the row"
    );
    assert_eq!(layouts(&placed), before, "every layout as before");
    assert_eq!(
        placed.boxes[ROW].asked, asked,
        "the row keeps what it was asked"
    );
}
