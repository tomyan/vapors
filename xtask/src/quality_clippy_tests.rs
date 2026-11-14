//! Reading clippy's JSON and joining it to xtask's functions.

use super::*;

fn diagnostic(code: &str, path: &str, line: usize, text: &str) -> String {
    format!(
        r#"{{"reason":"compiler-message","message":{{"code":{{"code":"{code}"}},"message":"{text}","spans":[{{"is_primary":false,"file_name":"x","line_start":1}},{{"is_primary":true,"file_name":"{path}","line_start":{line}}}]}}}}"#
    )
}

#[test]
fn counts_are_read_from_the_primary_span_and_the_n_over_1() {
    // Given
    let stream = [
        r#"{"reason":"compiler-artifact","target":{}}"#.to_owned(),
        diagnostic(
            "clippy::too_many_lines",
            "a.rs",
            3,
            "this function has too many lines (12/1)",
        ),
        diagnostic(
            "clippy::cognitive_complexity",
            "a.rs",
            4,
            "the function has a cognitive complexity of (3/1)",
        ),
        diagnostic("clippy::unused_self", "a.rs", 9, "unused `self` argument"),
        "not json".to_owned(),
    ]
    .join("\n");

    // When
    let counts = parse_counts(&stream).expect("read");

    // Then
    assert_eq!(counts.lines, BTreeMap::from([(("a.rs".to_owned(), 3), 12)]));
    assert_eq!(
        counts.complexity,
        BTreeMap::from([(("a.rs".to_owned(), 4), 3)])
    );
}

#[test]
fn a_diagnostic_of_another_shape_is_refused_quoting_it() {
    let stream = diagnostic("clippy::too_many_lines", "a.rs", 3, "no number here");
    let error = parse_counts(&stream).expect_err("refused");
    assert!(
        error.starts_with("clippy message without `(N/1)`: no number here"),
        "{error}"
    );
}
