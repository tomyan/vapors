use std::time::Duration;

use super::{PUSH_CALL_DEADLINE, session_name, time_left_to_push};

#[test]
fn a_session_name_a_ci_builder_could_take_is_refused() {
    assert_eq!(session_name("f2-slice"), Ok(()));
    let refused = session_name("f2-ci-1").expect_err("refused");
    assert!(
        refused.contains("`-ci-` names request_push's ci builders"),
        "{refused}"
    );
}

#[test]
fn the_push_starts_only_if_it_can_end_within_the_calls_deadline() {
    assert_eq!(time_left_to_push(Duration::from_mins(10)), Ok(()));
    let late = PUSH_CALL_DEADLINE.saturating_sub(Duration::from_secs(60));
    let refused = time_left_to_push(late).expect_err("refused");
    assert!(refused.contains("nothing was pushed"), "{refused}");
}
