//! Every job ends with the step that reports a timeout.

use std::time::Duration;

use super::*;
use crate::gate_yaml;

const NINE_MINUTES: Duration = Duration::from_mins(9);

fn steps(yaml: &str) -> Vec<Value> {
    let value = gate_yaml::parse(yaml, "ci.yml").expect("yaml");
    value.as_array().expect("a list").clone()
}

fn report(minutes: &str) -> String {
    format!(
        "- run: cargo xtask test\n\
         - name: report a timeout\n  if: cancelled()\n  timeout-minutes: 1\n  env:\n    GH_TOKEN: ${{{{ github.token }}}}\n    TIMEOUT_MINUTES: {minutes}\n  run: .github/scripts/report-timeout.sh\n"
    )
}

#[test]
fn a_job_ending_with_the_report_at_its_own_timeout_is_accepted() {
    // When
    let checked = check_timeout_report("gpu", NINE_MINUTES, &steps(&report("9")));

    // Then
    assert_eq!(checked, Ok(()));
}

#[test]
fn a_job_without_the_report_last_is_refused_naming_the_job_and_the_fix() {
    // Given: the report is there, but not last
    let yaml = format!("{}- run: echo after\n", report("9"));

    // When
    let refusal = check_timeout_report("gpu", NINE_MINUTES, &steps(&yaml)).expect_err("refused");

    // Then
    assert!(
        refusal.starts_with("gpu: the last step is not the timeout report"),
        "{refusal}"
    );
    assert!(refusal.contains("if: cancelled()"), "{refusal}");
    assert!(refusal.contains("TIMEOUT_MINUTES: 9"), "{refusal}");
}

#[test]
fn a_report_with_another_timeout_is_refused_naming_both() {
    // When
    let refusal =
        check_timeout_report("gpu", NINE_MINUTES, &steps(&report("10"))).expect_err("refused");

    // Then
    assert!(refusal.contains("TIMEOUT_MINUTES is 10"), "{refusal}");
    assert!(refusal.contains("timeout-minutes is 9"), "{refusal}");
}

#[test]
fn the_report_never_runs_in_the_gate_which_never_cancels() {
    // Given
    let report = &steps(&report("9"))[1];

    // When
    let runs = crate::gate_step::runs_on_success("gpu: report a timeout", report);

    // Then
    assert_eq!(runs, Ok(false));
}

#[test]
fn a_report_under_another_name_is_refused_since_the_script_skips_it_by_name() {
    // Given
    let yaml = report("9").replace("report a timeout", "say it timed out");

    // When
    let refusal = check_timeout_report("gpu", NINE_MINUTES, &steps(&yaml)).expect_err("refused");

    // Then
    assert!(refusal.contains("name: report a timeout"), "{refusal}");
}

#[test]
fn a_report_that_differs_in_how_it_runs_is_refused_naming_what() {
    let cases = [
        (
            "  run: .github/scripts/report-timeout.sh",
            "  run: echo timed out",
            "run `.github/scripts/report-timeout.sh`",
        ),
        (
            "    GH_TOKEN: ${{ github.token }}",
            "    GH_TOKEN: none",
            "env `GH_TOKEN: ${{ github.token }}`",
        ),
        (
            "  timeout-minutes: 1",
            "  timeout-minutes: 3",
            "`timeout-minutes: 1`",
        ),
    ];
    for (from, to, needle) in cases {
        // Given
        let yaml = report("9").replace(from, to);
        assert_ne!(yaml, report("9"), "{from} is in the report");

        // When
        let refusal = check_timeout_report("gpu", NINE_MINUTES, &steps(&yaml)).expect_err(needle);

        // Then
        assert!(refusal.starts_with("gpu: the timeout report"), "{refusal}");
        assert!(refusal.contains(needle), "{refusal}");
    }
}
