use super::super::KeptHits;

#[test]
#[should_panic(expected = "the kept hits note label 5 in frame 0, whose list holds no hit")]
fn a_relabel_of_a_label_noted_with_no_hits_is_refused() {
    // Given: label 5 noted in frame 0, whose list is empty
    let mut hits = KeptHits::default();
    hits.frames.insert(5, 0);
    hits.by_frame.insert(0, Vec::new());

    // When
    hits.relabel(&[(5, 6)]);
}

/// Kept hits holding one hit, in frame 0, under `label`.
fn one_hit_under(label: u64) -> KeptHits {
    let rect = crate::layout::BoxRect {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };
    let mut hits = KeptHits::default();
    hits.put(
        label,
        std::iter::once((0, (vapors_dom::NodeId(1), rect))),
        None,
    );
    hits
}

#[test]
fn hits_in_the_same_order_under_another_label_are_named_by_the_check() {
    // Given: the same hit, kept under label 5 and gathered under 6, so
    // the hits in order are equal and only the labels show the stale one
    let (kept, gathered) = (one_hit_under(5), one_hit_under(6));
    assert_eq!(kept.hits(), gathered.hits());

    // When
    let differing = kept.first_label_differing(&gathered);

    // Then
    assert_eq!(
        differing.as_deref(),
        Some("kept hits' label 5: in frame Some(0), gathered afresh None")
    );
    assert_eq!(kept.first_label_differing(&one_hit_under(5)), None);
}
