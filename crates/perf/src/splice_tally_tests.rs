use super::SpliceTally;
use vapors_elements::{SpliceDecline, UpdateCounts};

#[test]
fn each_frames_splices_and_relabels_are_summed_into_the_line() {
    // Given: two frames, one spliced and patched with 3 labels moved
    let mut spliced = UpdateCounts::default();
    (spliced.order_splices, spliced.order_splices_patched) = (1, 1);
    spliced.paint_relabelled = 3;

    // When
    let tally = SpliceTally::default()
        .and(&spliced)
        .and(&UpdateCounts::default());

    // Then
    assert_eq!(
        tally.line(1000),
        "scaling-splices: 1000 rows: 2 frames, 1 splices, 1 patched, paint_relabelled 3, \
         paint_relabel_all 0, declined freed_stacked 0 inserted_stacked 0 other 0, order_walks 0"
    );
}

#[test]
fn declines_are_counted_by_reason_and_walks_summed() {
    // Given: a frame declined for a freed box's stacking, one for an
    // inserted box's, one for another reason; each walked paint order
    let declined = |reason| {
        let mut counts = UpdateCounts::default();
        (
            counts.splices_declined,
            counts.splice_decline,
            counts.order_walks,
        ) = (1, Some(reason), 1);
        counts
    };

    // When
    let tally = SpliceTally::default()
        .and(&declined(SpliceDecline::FreedStacked))
        .and(&declined(SpliceDecline::InsertedStacked))
        .and(&declined(SpliceDecline::Reordered));

    // Then
    let counted = (
        tally.declined_freed_stacked,
        tally.declined_inserted_stacked,
        tally.declined_other,
        tally.order_walks,
    );
    assert_eq!(counted, (1, 1, 1, 3));
}
