use super::differing;
use crate::kept_order::KeptOrder;
use crate::paint_order::Step;

/// A kept order of boxes 1, 2 and 3 with `labels`, and a fresh one of
/// the same steps.
fn orders(labels: Vec<u64>) -> (KeptOrder, KeptOrder) {
    let steps = vec![Step::Box(1), Step::Box(2), Step::Box(3)];
    let kept = KeptOrder {
        steps: steps.clone(),
        labels,
        ..KeptOrder::default()
    };
    let fresh = KeptOrder {
        steps,
        labels: crate::paint_labels::afresh(3),
        ..KeptOrder::default()
    };
    (kept, fresh)
}

#[test]
fn two_equal_labels_are_named_with_their_steps() {
    // Given
    let (kept, fresh) = orders(vec![10, 20, 20]);

    // When
    let parts = differing(&kept, &fresh);

    // Then
    assert_eq!(
        parts,
        vec!["labels out of order at step 1: Box(2) labelled 20, then Box(3) labelled 20"]
    );
}

#[test]
fn a_label_count_that_is_not_the_step_count_is_named() {
    // Given
    let (kept, fresh) = orders(vec![10, 20]);

    // When
    let parts = differing(&kept, &fresh);

    // Then
    assert_eq!(parts, vec!["labels: 2 for 3 steps"]);
}

#[test]
fn increasing_labels_differ_in_nothing() {
    let (kept, fresh) = orders(vec![10, 20, 30]);
    assert_eq!(differing(&kept, &fresh), Vec::<String>::new());
}
