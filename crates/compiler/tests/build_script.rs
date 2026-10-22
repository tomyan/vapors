//! `build::build_script_report`: what `compile_dir` prints for cargo
//! and whether the build script must fail.

#[path = "common/build.rs"]
mod build;

use build::{COUNTER, dirs, write};
use vapors_compiler::build::{build_script_report, resolve_src};

#[test]
fn success_prints_a_rerun_line_per_input() {
    // Given
    let (src, out) = dirs("script_success");
    write(&src, "counter.wisp", COUNTER);

    // When
    let report = build_script_report(&src, Some(out.into_os_string()));

    // Then
    assert!(!report.failed);
    assert_eq!(
        report.stdout,
        vec![
            format!("cargo::rerun-if-changed={}", src.display()),
            format!(
                "cargo::rerun-if-changed={}",
                src.join("counter.wisp").display()
            ),
        ]
    );
    assert!(report.stderr.is_empty(), "{:?}", report.stderr);
}

#[test]
fn a_refusal_is_printed_rustc_style_and_fails() {
    // Given
    let (src, out) = dirs("script_refusal");
    write(&src, "a.wisp", "<blink></blink>");

    // When
    let report = build_script_report(&src, Some(out.into_os_string()));

    // Then
    assert!(report.failed);
    assert_eq!(report.stderr.len(), 1, "{:?}", report.stderr);
    assert!(
        report.stderr[0].starts_with(&format!("{}:1:1: ", src.join("a.wisp").display())),
        "{:?}",
        report.stderr
    );
    assert!(
        report
            .stdout
            .contains(&format!("cargo::rerun-if-changed={}", src.display())),
        "a fix must rerun the build: {:?}",
        report.stdout
    );
}

#[test]
fn a_missing_out_dir_fails_and_names_it() {
    // Given
    let (src, _) = dirs("script_no_out_dir");
    write(&src, "counter.wisp", COUNTER);

    // When
    let report = build_script_report(&src, None);

    // Then
    assert!(report.failed);
    assert!(report.stderr[0].contains("OUT_DIR"), "{:?}", report.stderr);
    assert!(report.stderr[0].contains("build.rs"), "{:?}", report.stderr);
}

#[test]
fn a_relative_source_directory_is_resolved_against_the_manifest() {
    // When
    let resolved = resolve_src(std::path::Path::new("src"), Some("/work/crate".into()));

    // Then: refusals then name a path that opens from any directory
    assert_eq!(resolved, std::path::Path::new("/work/crate/src"));
}

#[test]
fn an_absolute_source_directory_is_kept() {
    // When
    let resolved = resolve_src(
        std::path::Path::new("/elsewhere/src"),
        Some("/work/crate".into()),
    );

    // Then
    assert_eq!(resolved, std::path::Path::new("/elsewhere/src"));
}
