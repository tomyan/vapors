//! Labelling what a splice put in (`kept_order_splice.rs`,
//! `paint_labels.rs`), and what it did, for the drawing steps, the
//! extents index and the hits to follow (`paint_partial.rs`
//! `drawn_updated`) rather than be made afresh.

use std::ops::Range;

use crate::paint_labels::{Label, Lean, Relabelled};
use crate::paint_order::Step;

/// What the last splice did to paint order: the steps taken out, with the
/// labels they had; those put in, with theirs; those relabelled to make
/// room, with old and new labels; and whether every label was, which
/// makes the drawing steps, the extents index and the hits afresh.
#[derive(Debug, Default)]
pub(crate) struct SpliceEdits {
    pub(crate) removed: Vec<(Label, Step)>,
    pub(crate) inserted: Vec<(Label, Step)>,
    pub(crate) relabelled: Vec<(Step, Label, Label)>,
    pub(crate) all: bool,
}

/// A splice's labels made, what changed, and each run's end labels.
pub(super) struct Labelled {
    pub(super) labels: Vec<Label>,
    pub(super) edits: SpliceEdits,
    /// The first and last labels of each run put in.
    pub(super) put_in: Vec<(Label, Label)>,
}

impl Labelled {
    /// `labels` with each run of `runs` labelled (`paint_labels::label_run`,
    /// stepping `spacing` at the ends, leaning on a run the last splice put
    /// in, `recent`, when it touches one), `steps` the spliced steps.
    /// After a relabel of every label, `spacing` is theirs.
    pub(super) fn of(
        (steps, mut labels): (&[Step], Vec<Label>),
        runs: &[Range<usize>],
        (spacing, recent): (&mut u64, &[(Label, Label)]),
    ) -> Self {
        let mut edits = label_runs((steps, &mut labels), runs, (spacing, recent));
        for run in runs {
            let put = run.clone().map(|at| (labels[at], steps[at]));
            edits.inserted.extend(put);
        }
        let put_in = runs
            .iter()
            .map(|run| (labels[run.start], labels[run.end - 1]))
            .collect();
        Self {
            labels,
            edits,
            put_in,
        }
    }
}

/// Labels each run of `runs` in `labels`, the later runs not yet
/// labelled skipped by a relabel's search; what the relabels did
/// besides. After a relabel of every label, `spacing` is theirs.
fn label_runs(
    (steps, labels): (&[Step], &mut [Label]),
    runs: &[Range<usize>],
    (spacing, recent): (&mut u64, &[(Label, Label)]),
) -> SpliceEdits {
    let leans: Vec<Lean> = runs
        .iter()
        .map(|run| lean_of(labels, run, recent))
        .collect();
    let mut edits = SpliceEdits::default();
    for (at, (run, lean)) in runs.iter().zip(leans).enumerate() {
        let pending = &runs[at + 1..];
        let done = crate::paint_labels::label_run(labels, (run.clone(), pending), (*spacing, lean));
        note(&mut edits, (steps, labels, runs), done);
    }
    if edits.all {
        // every label laid out afresh, at the spacing for their count
        *spacing = crate::paint_labels::spacing(labels.len());
    }
    edits
}

/// Notes in `edits` what labelling a run did besides labelling it: the
/// steps of `steps` it relabelled, those of no run of `runs`, with old and
/// new labels; every label.
fn note(
    edits: &mut SpliceEdits,
    (steps, labels, runs): (&[Step], &[Label], &[Range<usize>]),
    done: Relabelled,
) {
    match done {
        Relabelled::None => {}
        Relabelled::All => edits.all = true,
        Relabelled::Range(range, before) => {
            let put_in = |at: &usize| runs.iter().any(|run| run.contains(at));
            for (at, old) in range.zip(before) {
                if !put_in(&at) {
                    edits.relabelled.push((steps[at], old, labels[at]));
                }
            }
        }
    }
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
