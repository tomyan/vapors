//! `FlexInsert` refuses a frame whose list was not laid out by its lane.

use vapors_elements::UpdateCounts;

use super::laned;

#[test]
fn a_frame_laid_out_by_lanes_alone_is_laned_and_one_with_a_decline_is_not() {
    let mut laid = UpdateCounts::default();
    laid.lanes = 2;
    let mut declined = laid;
    declined.lanes_declined = 1;
    let none = UpdateCounts::default();
    assert_eq!(laned(&laid), Ok(()));
    assert!(laned(&declined).is_err());
    assert!(laned(&none).is_err());
}
