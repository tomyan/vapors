//! Counting lines of code as clippy's `too_many_lines` does.

use super::*;

#[test]
fn blank_and_comment_only_lines_are_not_code() {
    // Given
    let body = "{\n    let a = 1; // trailing\n\n    // a comment\n    /* one line */\n    a\n}";

    // When / Then
    assert_eq!(code_lines(body), 2);
}

#[test]
fn a_block_comment_across_lines_hides_them_until_it_ends() {
    // Given
    let body = "{\n    /* starts\n    still comment\n    ends */ let a = 1;\n    a\n}";

    // When / Then: the line the comment ends on has code after it
    assert_eq!(code_lines(body), 2);
}

#[test]
fn a_line_comment_inside_a_string_ends_the_lines_code_as_it_does_for_clippy() {
    // Given
    let body = "{\n    let url = \"http://x\";\n    url\n}";

    // When / Then: `http:` is code before the `//`, so the line counts
    assert_eq!(code_lines(body), 2);
}

#[test]
fn a_body_that_is_not_braced_is_counted_whole() {
    // Given / When / Then
    assert_eq!(code_lines("a\nb"), 2);
}

#[test]
fn the_index_slices_by_line_and_character_column() {
    // Given: a non-ASCII character before the span on its line
    let text = "fn a() {}\nlet é = {\n1\n};\n";
    let index = LineIndex::new(text);
    let start = LineColumn { line: 2, column: 8 };
    let end = LineColumn { line: 4, column: 1 };

    // When
    let slice = index.slice(start, end);

    // Then
    assert_eq!(slice, "{\n1\n}");
}

#[test]
fn a_column_past_the_lines_end_stops_at_the_lines_end() {
    // Given
    let text = "ab\ncd";
    let index = LineIndex::new(text);

    // When
    let slice = index.slice(
        LineColumn { line: 1, column: 0 },
        LineColumn { line: 1, column: 9 },
    );

    // Then
    assert_eq!(slice, "ab\n");
}
