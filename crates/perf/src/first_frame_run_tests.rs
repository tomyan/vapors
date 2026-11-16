use super::profile;

#[test]
fn a_first_frame_run_takes_release_or_judged_and_refuses_another_profile() {
    assert_eq!(
        profile("release"),
        Ok("release (the shipped profile)".to_owned())
    );
    assert!(
        profile("judged")
            .expect("judged")
            .starts_with("judged (release, codegen-units 1")
    );
    let refused = profile("debug").expect_err("refused");
    assert!(refused.contains("debug profile"), "{refused}");
    assert!(
        refused.contains("cargo build --profile judged -p vapors-perf"),
        "{refused}"
    );
}
