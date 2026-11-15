//! Paint-order labels (`docs/plans/paint-order-labels.md`): each step in
//! paint order holds a `u64` label; labels increase along the order and
//! leave gaps, so a step put in between two others takes a label between
//! theirs and no later label moves. A walk spaces its labels evenly over
//! `u64`, `ROOM` spacings free at each end; an append or a prepend steps
//! by that spacing from the end label; an insert between two takes the
//! midpoint, or leans on the run put in last when it touches it
//! (`Lean`). When a gap runs out, the smallest aligned label range around it whose density is under its
//! threshold is relabelled evenly (Bender et al., "Two simplified
//! algorithms for maintaining order in a list", with T = 1.5, which
//! leaves room for 10^8 labels in `u64`): amortized O(log n) labels a
//! step put in. A relabel that finds no such range relabels every label,
//! counted apart.

/// A step's place in paint order: compared, never counted.
pub(crate) type Label = u64;

/// A range of `2^level` labels relabels when it holds fewer than
/// `(2 / T)^level` steps, T = 1.5: fewer than `(4 / 3)^level`, compared
/// exactly as `held * 3^level < 4^level`.
fn under_threshold(held: usize, level: u32) -> bool {
    let held = u128::try_from(held).expect("under 2^128 steps");
    let scaled = held.checked_mul(3u128.pow(level));
    scaled.is_some_and(|scaled| scaled < 4u128.pow(level))
}

/// What putting steps in did besides labelling them: the positions whose
/// labels it changed, or every label.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Relabelled {
    #[default]
    None,
    /// The labels at these positions, after the insert, changed.
    Range(std::ops::Range<usize>),
    /// Every label changed.
    All,
}

/// The spacings a walk leaves free at each end of its labels, for the
/// appends and prepends that step by the spacing (rows put in at the end
/// of a list, whose outlines end paint order).
const ROOM: u64 = 1 << 19;

/// The spacing of a walk's `n` labels: `u64` divided evenly among them
/// and `ROOM` free spacings at each end.
pub(crate) fn spacing(n: usize) -> u64 {
    let n = u64::try_from(n).expect("under 2^64 steps");
    (u64::MAX / n.saturating_add(2 * ROOM)).max(1)
}

/// A walk's `n` labels, increasing, `spacing(n)` apart, after `ROOM` free
/// spacings.
pub(crate) fn afresh(n: usize) -> Vec<Label> {
    let step = spacing(n);
    (1..=n as u64).map(|i| (ROOM + i) * step).collect()
}

/// Puts `count` labels in `labels` (increasing) at position `at`, between
/// its neighbours there, relabelling a range or every label when they
/// leave no room; what changed besides the new labels. `step` is the
/// spacing an append or a prepend steps by.
#[cfg(test)]
pub(crate) fn insert(labels: &mut Vec<Label>, at: usize, count: usize, step: u64) -> Relabelled {
    labels.splice(at..at, std::iter::repeat_n(0, count));
    label_run(labels, (at..at + count, &[]), (step, Lean::Middle))
}

/// Where in a gap a run's labels go: near the label put in last, when it
/// is one of the gap's ends (rows put in one after another, each after
/// the last, lean on the left), so the next run finds room on its side;
/// else the middle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lean {
    Left,
    Right,
    Middle,
}

/// The share of a gap a run leaning on one side takes: rows put in one
/// after another at one place fill about `LEAN` times as many as by
/// halving before a relabel.
const LEAN: u64 = 16;

/// Labels the positions `run` of `labels` between the labels around it,
/// leaning as `lean` says and stepping `step` from an end label: what
/// changed besides the run's labels. The labels increase but for `run`
/// and the `pending` runs after it, unlabelled yet, which a relabel's
/// search skips and its spread may label.
pub(crate) fn label_run(
    labels: &mut [Label],
    (run, pending): (std::ops::Range<usize>, &[std::ops::Range<usize>]),
    (step, lean): (u64, Lean),
) -> Relabelled {
    if run.is_empty() {
        return Relabelled::None;
    }
    let before = run.start.checked_sub(1).map(|i| labels[i]);
    let after = labels.get(run.end).copied();
    if let Some(new) = fitted((before, after), run.len(), (step, lean)) {
        labels[run].copy_from_slice(&new);
        return Relabelled::None;
    }
    relabel_around(labels, run, pending)
}

/// `count` labels between `before` and `after`: stepped from the end label
/// for an append or a prepend, else spread evenly over the gap; `None`
/// when they do not fit.
fn fitted(
    (before, after): (Option<Label>, Option<Label>),
    count: usize,
    (step, lean): (u64, Lean),
) -> Option<Vec<Label>> {
    if before.is_none() && after.is_none() {
        return Some(afresh(count));
    }
    let count = u64::try_from(count).expect("under 2^64 steps");
    match (before, after) {
        (None, None) => None,
        (Some(a), None) => {
            a.checked_add(step.checked_mul(count)?)?;
            Some((1..=count).map(|i| a + step * i).collect())
        }
        (None, Some(b)) => {
            let first = b.checked_sub(step.checked_mul(count)?)?;
            Some((0..count).map(|i| first + step * i).collect())
        }
        (Some(a), Some(b)) => between((a, b), count, lean),
    }
}

/// `count` labels between `a` and `b`: spread over the share of the gap
/// next to the end `lean` names, or over all of it, evenly.
fn between((a, b): (Label, Label), count: u64, lean: Lean) -> Option<Vec<Label>> {
    let gap = b - a;
    let share = match lean {
        Lean::Middle => gap,
        Lean::Left | Lean::Right => gap / LEAN,
    };
    let step = share / (count + 1);
    if step == 0 {
        return (lean != Lean::Middle)
            .then(|| between((a, b), count, Lean::Middle))
            .flatten();
    }
    let first = match lean {
        Lean::Right => b - step * (count + 1),
        Lean::Left | Lean::Middle => a,
    };
    Some((1..=count).map(|i| first + step * i).collect())
}

/// Relabels the smallest aligned label range around the positions `new`
/// (put in, unlabelled) whose density is under its threshold, every
/// label in it spread evenly; every label when none is. The `pending`
/// runs, after `new` and unlabelled, are not searched.
fn relabel_around(
    labels: &mut [Label],
    new: std::ops::Range<usize>,
    pending: &[std::ops::Range<usize>],
) -> Relabelled {
    let anchor = new.start.checked_sub(1).map_or(0, |i| labels[i]);
    for level in 1..64u32 {
        let size = 1u64 << level;
        let low = anchor & !(size - 1);
        let high = low.saturating_add(size - 1);
        let range = positions_in(labels, (low, high), (&new, pending));
        if under_threshold(range.len(), level) {
            spread(&mut labels[range.clone()], low, size);
            return Relabelled::Range(range);
        }
    }
    let n = labels.len();
    labels.copy_from_slice(&afresh(n));
    Relabelled::All
}

/// The positions whose labels fall in `low..=high`, the unlabelled `new`
/// ones among them (they sit where the range's labels are), and any of
/// the unlabelled `pending` runs between those.
fn positions_in(
    labels: &[Label],
    (low, high): (u64, u64),
    (new, pending): (&std::ops::Range<usize>, &[std::ops::Range<usize>]),
) -> std::ops::Range<usize> {
    let start = labels[..new.start].partition_point(|&label| label < low);
    start..first_above(labels, high, new.end, pending)
}

/// The first position from `from` whose label is above `high`, the
/// `pending` runs skipped, as their placeholders are no labels to
/// search; `labels.len()` when there is none.
fn first_above(
    labels: &[Label],
    high: u64,
    mut from: usize,
    pending: &[std::ops::Range<usize>],
) -> usize {
    let end = labels.len()..labels.len();
    for run in pending.iter().chain(std::iter::once(&end)) {
        let kept = &labels[from..run.start];
        let at = kept.partition_point(|&label| label <= high);
        if at < kept.len() {
            return from + at;
        }
        from = run.end;
    }
    labels.len()
}

/// Spreads `labels` evenly over the `size` labels from `low`.
fn spread(labels: &mut [Label], low: u64, size: u64) {
    let step = size / (labels.len() as u64 + 1);
    for (i, label) in labels.iter_mut().enumerate() {
        *label = low + step * (i as u64 + 1);
    }
}

#[cfg(test)]
#[path = "paint_labels_tests.rs"]
mod tests;
