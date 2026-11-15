//! The kept order spliced (`order_splice.rs`): the steps edited, the
//! new slots put below their parents and the freed ones taken out of
//! the clip tree and clips, the new elements joined to their parent's
//! scroll chain and the freed ones left, then the clips redone under
//! the changed slots as on an update that kept the shape.

use std::ops::Range;

use crate::changed_slots::ChangedSlots;
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, Content, Key, LayoutBox};
use crate::order_splice::{Edit, Splice};
use crate::paint::PaintInput;
use crate::paint_labels::{Label, Lean, Relabelled};
use crate::paint_order::Step;

impl KeptOrder {
    /// `self` with `splice` made, for the `changed` slots of `boxes`.
    pub(crate) fn spliced(
        mut self,
        splice: Splice,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
    ) -> Self {
        let (steps, labels, runs) = spliced_steps((&self.steps, &self.labels), splice.edits);
        let recent = std::mem::take(&mut self.recent);
        let ((relabelled, relabel_all), recent) = labelled_runs(
            &mut self.labels,
            (labels, runs),
            (&mut self.spacing, &recent),
        );
        self.recent = recent;
        self.steps = steps;
        let freed = || changed.freed.iter().map(|&(slot, _)| slot);
        self.tree.spliced(boxes.len(), &splice.added, freed());
        self.clips.spliced(boxes.len(), freed());
        self.chains_spliced(boxes, &splice.added, &changed.freed);
        let mut order = self.reclipped((boxes, rects), input, changed);
        order.walks.splices = 1;
        (order.walks.relabelled, order.walks.relabel_all) = (relabelled, relabel_all);
        order
    }

    /// Each `freed` element left its scroll chain; each `added`
    /// element, under its parent, joined the parent's.
    fn chains_spliced(
        &mut self,
        boxes: &[LayoutBox],
        added: &[(usize, usize)],
        freed: &[(usize, Key)],
    ) {
        for (_, key) in freed {
            if let Key::Element(node) = key {
                self.chains.left(*node);
            }
        }
        for &(slot, parent) in added {
            if let (Content::Element(node), Content::Element(above)) =
                (&boxes[slot].content, &boxes[parent].content)
            {
                self.chains.joined(*node, *above);
            }
        }
    }
}

/// `steps` and their `labels` with `edits` made, each at its place in
/// `steps`, and the runs of positions the inserted steps took, unlabelled
/// (0) until `labelled_runs`. At one place an insertion comes before a
/// removal, so steps put in where a removed run starts are not taken out
/// with it.
fn spliced_steps(
    (steps, labels): (&[Step], &[Label]),
    mut edits: Vec<Edit>,
) -> (Vec<Step>, Vec<Label>, Vec<Range<usize>>) {
    edits.sort_by_key(|edit| (edit.at, edit.removed));
    let grown: usize = edits.iter().map(|edit| edit.inserted.len()).sum();
    let mut out_steps = Vec::with_capacity(steps.len() + grown);
    let mut out_labels = Vec::with_capacity(steps.len() + grown);
    let mut runs: Vec<Range<usize>> = Vec::new();
    let mut from = 0;
    for edit in edits {
        assert!(
            edit.at >= from,
            "order splice edits overlap at step {}: two runs share a place (order_splice.rs \
             anchors); reproduce with `cargo xtask test`",
            edit.at
        );
        out_steps.extend_from_slice(&steps[from..edit.at]);
        out_labels.extend_from_slice(&labels[from..edit.at]);
        let at = out_steps.len();
        note_run(&mut runs, at..at + edit.inserted.len());
        out_labels.extend(std::iter::repeat_n(0, edit.inserted.len()));
        out_steps.extend(edit.inserted);
        from = edit.at + edit.removed;
    }
    out_steps.extend_from_slice(&steps[from..]);
    out_labels.extend_from_slice(&labels[from..]);
    (out_steps, out_labels, runs)
}

/// Notes the run `run` of inserted positions, joined to the last when they
/// touch, so every run's neighbours are labelled.
fn note_run(runs: &mut Vec<Range<usize>>, run: Range<usize>) {
    if run.is_empty() {
        return;
    }
    match runs.last_mut() {
        Some(last) if last.end == run.start => last.end = run.end,
        _ => runs.push(run),
    }
}

/// `labels` made the spliced `spliced` with each inserted run labelled
/// (`paint_labels::label_run`, stepping `spacing` at the ends, leaning
/// on a run the last splice put in, `recent`, when it touches one); the
/// kept labels relabelled to make room, the times every label was, and
/// the labels of each run put in. After a relabel of every label,
/// `spacing` is theirs.
fn labelled_runs(
    labels: &mut Vec<Label>,
    (spliced, runs): (Vec<Label>, Vec<Range<usize>>),
    (spacing, recent): (&mut u64, &[(Label, Label)]),
) -> ((usize, usize), Vec<(Label, Label)>) {
    *labels = spliced;
    let (mut relabelled, mut all) = (0, 0);
    let leans: Vec<Lean> = runs
        .iter()
        .map(|run| lean_of(labels, run, recent))
        .collect();
    for (at, (run, lean)) in runs.iter().zip(leans).enumerate() {
        let pending = &runs[at + 1..];
        match crate::paint_labels::label_run(labels, (run.clone(), pending), (*spacing, lean)) {
            Relabelled::None => {}
            Relabelled::Range(range) => relabelled += kept_in(&range, &runs),
            Relabelled::All => {
                relabelled += kept_in(&(0..labels.len()), &runs);
                all += 1;
            }
        }
    }
    if all > 0 {
        // every label laid out afresh, at the spacing for their count.
        // No test reaches this: a relabel of every label takes about
        // 7.4e7 labels, (4/3)^63, too many for a unit test's memory.
        *spacing = crate::paint_labels::spacing(labels.len());
    }
    let put_in = runs
        .iter()
        .map(|run| (labels[run.start], labels[run.end - 1]))
        .collect();
    ((relabelled, all), put_in)
}

/// How many positions of `range` hold kept steps: not in one of the
/// `runs` this splice put in, whose labels a relabel sets, not moves.
fn kept_in(range: &Range<usize>, runs: &[Range<usize>]) -> usize {
    let overlap = |run: &Range<usize>| {
        let (start, end) = (run.start.max(range.start), run.end.min(range.end));
        end.saturating_sub(start)
    };
    range.len() - runs.iter().map(overlap).sum::<usize>()
}

/// Where `run`'s labels lean: on the left when the label before it ends
/// a run of `recent`, on the right when the label after it starts one,
/// else the middle.
fn lean_of(labels: &[Label], run: &Range<usize>, recent: &[(Label, Label)]) -> Lean {
    let before = run.start.checked_sub(1).map(|i| labels[i]);
    let after = labels.get(run.end).copied();
    if recent.iter().any(|&(_, last)| before == Some(last)) {
        Lean::Left
    } else if recent.iter().any(|&(first, _)| after == Some(first)) {
        Lean::Right
    } else {
        Lean::Middle
    }
}

#[cfg(test)]
#[path = "kept_order_splice_tests.rs"]
mod tests;
