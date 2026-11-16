use super::SpliceTally;
use vapors_elements::UpdateCounts;

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
         paint_relabel_all 0"
    );
}
