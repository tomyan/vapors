use super::agreed;

#[test]
fn the_proxys_count_and_the_clis_report_agree_within_a_hundredth_of_a_cent() {
    assert_eq!(agreed(0.004_088, Some(0.004_088), true), Ok(()));
}

#[test]
fn a_count_the_cli_disagrees_with_fails_naming_both() {
    let refused = agreed(0.0007, Some(0.0005), true).expect_err("refused");
    assert!(
        refused.starts_with("the proxy counted $0.0007 and the CLI reported $0.0005"),
        "{refused}"
    );
}

#[test]
fn a_session_that_ended_with_0_and_reported_no_cost_fails() {
    let refused = agreed(0.0005, None, true).expect_err("refused");
    assert!(refused.contains("holds no total_cost_usd"), "{refused}");
}

#[test]
fn a_session_the_harness_stopped_may_report_no_cost() {
    assert_eq!(agreed(0.0005, None, false), Ok(()));
}
