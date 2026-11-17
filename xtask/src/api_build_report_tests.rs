use super::agreed;

#[test]
fn the_proxys_count_and_the_clis_report_agree_within_a_hundredth_of_a_cent() {
    assert_eq!(agreed(0.004_088, Some(0.004_088)), Ok(()));
    assert_eq!(agreed(0.0005, None), Ok(()));
}

#[test]
fn a_count_the_cli_disagrees_with_fails_naming_both() {
    let refused = agreed(0.0007, Some(0.0005)).expect_err("refused");
    assert!(
        refused.starts_with("the proxy counted $0.0007 and the CLI reported $0.0005"),
        "{refused}"
    );
}
