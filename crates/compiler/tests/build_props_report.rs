//! How the props check reports (review-v67, 1 and 7): a script that
//! does not parse is refused once, with the other files' refusals still
//! reported, in file order; unchecked tags reach the build output as
//! cargo warnings.

use crate::common_build as build;
use crate::common_props as props;

use build::write;
use props::{USE_PLAIN, shown, with_plain};
use vapors_compiler::build::{build_script_report, compile_dir_into};

#[test]
fn a_script_that_does_not_parse_is_refused_once() {
    // Given
    let (src, out) = build::dirs("parse_once");
    write(&src, "bad.wisp", "<script>let x: = ;</script><p/>");

    // When
    let errors = shown(compile_dir_into(&src, &out));

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn every_files_refusals_are_reported_in_file_order() {
    // Given: props refusals in `a` and `b`, found after `c`'s script error
    let (src, out) = with_plain("file_order");
    write(&src, "c.wisp", "<script>let x: = ;</script><p/>");
    write(&src, "a.wisp", &format!("{USE_PLAIN}<Plain titel=\"a\" />"));
    write(&src, "b.wisp", &format!("{USE_PLAIN}<Plain titel=\"b\" />"));

    // When
    let errors = shown(compile_dir_into(&src, &out));

    // Then
    let files: Vec<&str> = errors
        .iter()
        .map(|e| e.split(':').next().unwrap_or(""))
        .collect();
    assert_eq!(files.len(), 3, "{errors:?}");
    assert!(
        files[0].ends_with("a.wisp")
            && files[1].ends_with("b.wisp")
            && files[2].ends_with("c.wisp"),
        "{errors:?}"
    );
}

#[test]
fn an_unchecked_tag_is_a_cargo_warning_in_the_build_output() {
    // Given
    let (src, out) = with_plain("warned");
    write(&src, "page.wisp", "<other_crate::Plain big=\"x\" />");

    // When
    let report = build_script_report(&src, Some(out.into_os_string()), Some("1".into()));

    // Then
    assert!(!report.failed, "{:?}", report.stderr);
    let warning = report
        .stdout
        .iter()
        .find(|line| line.starts_with("cargo::warning=") && line.contains("page.wisp"));
    assert!(
        warning.is_some_and(|line| line.contains("page.wisp:1:1") && line.contains("not checked")),
        "{:?}",
        report.stdout
    );
}

fn unchecked_dir(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let (src, out) = with_plain(name);
    write(&src, "a.wisp", "<Scene><Group /><Group /></Scene>");
    write(&src, "b.wisp", "<Group /><Mesh />");
    (src, out)
}

fn warnings(report: &vapors_compiler::build::BuildScriptReport) -> Vec<&String> {
    let lines = report.stdout.iter();
    lines.filter(|l| l.starts_with("cargo::warning=")).collect()
}

#[test]
fn unchecked_tags_across_files_are_one_warning_with_counts() {
    // Given
    let (src, out) = unchecked_dir("one_line");

    // When
    let report = build_script_report(&src, Some(out.into_os_string()), None);

    // Then
    let lines = warnings(&report);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("5 component tags"), "{}", lines[0]);
    assert!(
        lines[0].contains("Group (3), Mesh (1), Scene (1)"),
        "{}",
        lines[0]
    );
}

#[test]
fn wisp_notes_one_adds_a_positioned_line_per_tag_after_the_summary() {
    // Given
    let (src, out) = unchecked_dir("notes_on");

    // When
    let report = build_script_report(&src, Some(out.into_os_string()), Some("1".into()));

    // Then
    let lines = warnings(&report);
    assert_eq!(lines.len(), 6, "{lines:?}");
    assert!(lines[0].contains("5 component tags"), "{}", lines[0]);
    assert!(
        lines[1..].iter().all(|l| l.contains(".wisp:1:")),
        "{lines:?}"
    );
}

#[test]
fn wisp_notes_zero_and_no_unchecked_tags_are_quiet_or_summary_only() {
    // Given
    let (src, out) = with_plain("notes_zero");
    write(&src, "a.wisp", "<p/>");

    // When
    let quiet = build_script_report(&src, Some(out.clone().into_os_string()), Some("0".into()));
    write(&src, "b.wisp", "<Mesh />");
    let one = build_script_report(&src, Some(out.into_os_string()), Some("0".into()));

    // Then
    assert!(warnings(&quiet).is_empty(), "{:?}", quiet.stdout);
    assert_eq!(warnings(&one).len(), 1, "{:?}", one.stdout);
}

#[test]
fn any_other_wisp_notes_value_is_refused_naming_it() {
    // Given
    let (src, out) = unchecked_dir("notes_bad");

    // When
    let report = build_script_report(&src, Some(out.into_os_string()), Some("yes".into()));

    // Then
    assert!(report.failed);
    let refusal = report.stderr.join("\n");
    assert!(
        refusal.contains("WISP_NOTES") && refusal.contains("\"yes\""),
        "{refusal}"
    );
    assert!(refusal.contains('0') && refusal.contains('1'), "{refusal}");
}
