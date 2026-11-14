use super::{Label, Relabelled, afresh, insert, spacing};

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
            Relabelled::Range(range, _) => range.len(),
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
    assert!(
        matches!(relabelled, Relabelled::Range(..)),
        "{relabelled:?}"
    );
    assert!(increasing(&labels), "{labels:?}");
}
