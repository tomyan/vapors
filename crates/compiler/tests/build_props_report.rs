//! How the props check reports (review-v67, 1 and 7): a script that
//! does not parse is refused once, with the other files' refusals still
//! reported, in file order; unchecked tags reach the build output as
//! cargo warnings.

#[path = "common/build.rs"]
mod build;
#[path = "common/props.rs"]
mod props;

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
    let report = build_script_report(&src, Some(out.into_os_string()));

    // Then
    assert!(!report.failed, "{:?}", report.stderr);
    let warning = report
        .stdout
        .iter()
        .find(|line| line.starts_with("cargo::warning="));
    assert!(
        warning.is_some_and(|line| line.contains("page.wisp:1:1") && line.contains("not checked")),
        "{:?}",
        report.stdout
    );
}
