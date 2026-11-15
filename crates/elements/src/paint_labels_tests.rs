use super::{Label, Lean, Relabelled, afresh, insert, label_run, spacing};

/// Whether `labels` strictly increase.
fn increasing(labels: &[Label]) -> bool {
    labels.windows(2).all(|pair| pair[0] < pair[1])
}

/// Puts one label in, `times` times, at the position `at` gives for the
/// labels held, from `start` labels; the labels relabelled, summed.
fn inserted(start: usize, times: usize, at: impl Fn(usize) -> usize) -> (Vec<Label>, usize) {
    let mut labels = afresh(start);
    let step = spacing(start);
    let mut relabelled = 0;
    for _ in 0..times {
        let position = at(labels.len());
        relabelled += match insert(&mut labels, position, 1, step) {
            Relabelled::None => 0,
            Relabelled::Range(range) => range.len(),
            Relabelled::All => labels.len(),
        };
        assert!(
            increasing(&labels),
            "labels out of order after an insert at {position}"
        );
    }
    (labels, relabelled)
}

/// The amortized bound the note states, over `inserts` steps put in:
/// `4 log2(n) + 8` labels a step (log2 rounded down), `n` the labels held.
fn bound(n: usize, inserts: usize) -> usize {
    (4 * n.ilog2() as usize + 8) * inserts
}

#[test]
fn a_walk_spaces_its_labels_evenly_and_increasing() {
    let labels = afresh(5);
    assert!(increasing(&labels));
    assert_eq!(labels[1] - labels[0], spacing(5));
    assert!(
        labels[0] >= spacing(5) << 19,
        "no room before the first label"
    );
    assert!(
        u64::MAX - labels[4] >= spacing(5) << 19,
        "no room after the last label"
    );
}

#[test]
fn ten_thousand_inserts_into_one_gap_relabel_within_the_bound() {
    // When: always between the first two labels of 16
    let (labels, relabelled) = inserted(16, 10_000, |_| 1);

    // Then
    assert_eq!(labels.len(), 10_016);
    let bound = bound(labels.len(), 10_000);
    assert!(
        relabelled <= bound,
        "{relabelled} labels relabelled in 10,000 inserts, over {bound}"
    );
}

#[test]
fn ten_thousand_prepends_relabel_within_the_bound() {
    let (labels, relabelled) = inserted(16, 10_000, |_| 0);
    let bound = bound(labels.len(), 10_000);
    assert!(
        relabelled <= bound,
        "{relabelled} labels relabelled in 10,000 inserts, over {bound}"
    );
}

#[test]
fn ten_thousand_appends_step_by_the_spacing_and_relabel_within_the_bound() {
    let (labels, relabelled) = inserted(16, 10_000, |len| len);
    let bound = bound(labels.len(), 10_000);
    assert!(
        relabelled <= bound,
        "{relabelled} labels relabelled in 10,000 inserts, over {bound}"
    );
}

#[test]
fn a_run_put_in_between_two_labels_sits_between_them() {
    // Given
    let mut labels = vec![10, 20];

    // When
    let relabelled = insert(&mut labels, 1, 3, 100);

    // Then
    assert_eq!(relabelled, Relabelled::None);
    assert_eq!(labels, vec![10, 12, 14, 16, 20]);
}

#[test]
fn a_full_gap_relabels_a_range_around_it() {
    // Given: no room between 10 and 11
    let mut labels = vec![10, 11, 1_000_000];

    // When
    let relabelled = insert(&mut labels, 1, 1, 100);

    // Then: a range relabelled, the order kept
    assert!(matches!(relabelled, Relabelled::Range(_)), "{relabelled:?}");
    assert!(increasing(&labels), "{labels:?}");
}

/// Puts one label in `times` times beside the newest, leaning on it as
/// a splice does (`kept_order_splice.rs` `lean_of`): after it when
/// `after(i)` for the `i`th insert (leaning left), else before it
/// (leaning right); from 16 labels, the first put in between the first
/// two; the labels relabelled, summed.
fn leaned(times: usize, after: impl Fn(usize) -> bool) -> (Vec<Label>, usize) {
    let mut labels = afresh(16);
    let step = spacing(16);
    let (mut newest, mut relabelled) = (1, 0);
    insert(&mut labels, newest, 1, step);
    for i in 0..times {
        let (at, lean) = if after(i) {
            (newest + 1, Lean::Left)
        } else {
            (newest, Lean::Right)
        };
        labels.insert(at, 0);
        relabelled += match label_run(&mut labels, (at..at + 1, &[]), (step, lean)) {
            Relabelled::None => 0,
            Relabelled::Range(range) => range.len(),
            Relabelled::All => labels.len(),
        };
        assert!(increasing(&labels), "labels out of order after insert {i}");
        newest = at;
    }
    (labels, relabelled)
}

#[test]
fn ten_thousand_inserts_each_after_the_newest_relabel_within_the_bound() {
    let (labels, relabelled) = leaned(10_000, |_| true);
    let bound = bound(labels.len(), 10_000);
    assert!(
        relabelled <= bound,
        "{relabelled} labels relabelled in 10,000 inserts, over {bound}"
    );
}

#[test]
fn ten_thousand_inserts_each_before_the_newest_relabel_within_the_bound() {
    let (labels, relabelled) = leaned(10_000, |_| false);
    let bound = bound(labels.len(), 10_000);
    assert!(
        relabelled <= bound,
        "{relabelled} labels relabelled in 10,000 inserts, over {bound}"
    );
}

#[test]
fn ten_thousand_inserts_alternating_after_and_before_the_newest_relabel_within_the_bound() {
    let (labels, relabelled) = leaned(10_000, |i| i % 2 == 0);
    let bound = bound(labels.len(), 10_000);
    assert!(
        relabelled <= bound,
        "{relabelled} labels relabelled in 10,000 inserts, over {bound}"
    );
}

#[test]
fn a_relabel_does_not_search_the_runs_still_to_be_labelled() {
    // Given: a run between 10 and 11, with no room, and a later run
    // still holding its placeholder between 1000 and 5000
    let mut labels = vec![10, 0, 11, 1000, 0, 5000];

    // When
    let pending = std::slice::from_ref(&(4..5));
    let relabelled = label_run(&mut labels, (1..2, pending), (1, Lean::Middle));

    // Then: the range ends at 1000, the first label above it
    assert_eq!(relabelled, Relabelled::Range(0..3));
    assert_eq!(labels[3..], [1000, 0, 5000]);
    assert!(increasing(&labels[..4]), "{labels:?}");
}
