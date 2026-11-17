//! One-scene S5a, part 1: the build parses, refuses, classifies and
//! deduplicates each crate's literal shapes into its `SHAPES` table.

use crate::common_build as build;

use build::{dirs, read, write};
use vapors_compiler::build::compile_dir_into;

const TRIANGLE: &str = "M0 0L4 0L2 3Z";

/// A component of one svg holding `shapes`; its template starts on line 1.
fn page(shapes: &str) -> String {
    format!("<svg viewBox=\"0 0 8 8\" width=\"8\" height=\"8\">\n{shapes}\n</svg>")
}

fn errors_of(name: &str, files: &[(&str, String)]) -> Vec<String> {
    let (src, out) = dirs(name);
    for (file, text) in files {
        write(&src, file, text);
    }
    let refused = compile_dir_into(&src, &out).expect_err("the build refuses");
    refused.iter().map(ToString::to_string).collect()
}

#[test]
fn a_bad_d_fails_the_build_naming_the_file_and_line() {
    // Given: the bad path on line 3 of its file
    let files = [(
        "icon.wisp",
        page("<path d=\"M0 0L4 0L2 3Z\"/>\n<path d=\"M0 0 X\"/>"),
    )];

    // When
    let errors = errors_of("shapes_bad_d", &files);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("icon.wisp:3:"), "{errors:?}");
    assert!(errors[0].contains("<path> d"), "{errors:?}");
}

#[test]
fn a_path_the_tessellator_refuses_fails_the_build_naming_the_file_and_line() {
    // Given: a fill with no area
    let files = [("flat.wisp", page("<path d=\"M0 0L4 0Z\"/>"))];

    // When
    let errors = errors_of("shapes_no_area", &files);

    // Then
    assert!(errors[0].contains("flat.wisp:2:1: "), "{errors:?}");
    assert!(errors[0].contains("has no area"), "{errors:?}");
}

#[test]
fn the_same_path_in_two_components_of_one_crate_is_one_entry() {
    // Given: a triangle in both, a rect path in the second, a bound path
    let (src, out) = dirs("shapes_dedup");
    write(&src, "a.wisp", &page(&format!("<path d=\"{TRIANGLE}\"/>")));
    let second = format!("<path d=\"{TRIANGLE}\"/><path d=\"M1 1H5V4H1Z\"/><path d={{d}}/>");
    write(&src, "b.wisp", &bound(&second));

    // When
    compile_dir_into(&src, &out).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then: wisp.rs includes the table, which holds each shape once
    assert!(read(&out.join("wisp.rs")).contains("include!(\"shapes.rs\")"));
    insta::assert_snapshot!(read(&out.join("shapes.rs")));
}

/// A component whose script has the string signal `d`.
fn bound(shapes: &str) -> String {
    format!(
        "<script>\nlet d: Signal<String> = Signal::new(String::new());\n</script>\n{}",
        page(shapes)
    )
}
