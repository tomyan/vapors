//! Translated from sumi's `parser/template/error_test.go`,
//! `parser_title_test.go` and `slot_removed_test.go`, plus the
//! refusals vapors adds. Every refusal names `file:line:col`. sumi's
//! `Error` carries a byte offset that editor tooling maps later; here
//! the position is resolved when the error is made. `<title>` set the
//! terminal title in sumi; a window title is a platform concern here,
//! so the tag is refused with that pointer rather than parsed.

mod common;

use common::{parse_err, parse_file, parse_ok};

#[test]
fn error_displays_as_file_line_col_message() {
    // When
    let err = parse_err("  @oops");

    // Then
    assert_eq!((err.line, err.col), (1, 3), "{err}");
    assert_eq!(err.to_string(), format!("test.wisp:1:3: {}", err.message));
    assert_eq!(err.path, "test.wisp");
}

#[test]
fn positions_count_lines_and_columns_in_characters() {
    // When
    let err = parse_err("<div>\n  <p>héllo</p><p>x</span>\n</div>");

    // Then: the stray </span> on line 2 at character 19 (byte 20)
    assert_eq!((err.line, err.col), (2, 19), "{err}");
}

#[test]
fn mismatched_close_tag_names_both_tags_and_where_it_opened() {
    // When
    let err = parse_err("<div>\n  <span>x</div>\n</div>");

    // Then
    assert_eq!((err.line, err.col), (2, 10), "{err}");
    assert!(
        err.message.contains("</span>") && err.message.contains("</div>"),
        "{err}"
    );
    assert!(
        err.message.contains("2:3"),
        "names where <span> opened: {err}"
    );
}

#[test]
fn unclosed_tag_is_refused_where_it_opened() {
    // When
    let err = parse_err("<div>\n  <span>x\n");

    // Then
    assert_eq!((err.line, err.col), (2, 3), "{err}");
    assert!(err.message.contains("</span>"), "{err}");
}

#[test]
fn close_tag_without_open_is_refused() {
    // When
    let err = parse_err("<div></div></span>");

    // Then
    assert_eq!((err.line, err.col), (1, 12), "{err}");
    assert!(err.message.contains("</span>"), "{err}");
}

#[test]
fn unterminated_text_hole_is_refused() {
    // sumi keeps an unterminated `{` as literal text; here it is refused.
    // When
    let err = parse_err("<p>total: {count</p>");

    // Then
    assert_eq!((err.line, err.col), (1, 11), "{err}");
    assert!(err.message.contains('}'), "{err}");
}

#[test]
fn unbalanced_braces_in_a_text_hole_are_refused() {
    // When
    let err = parse_err("<p>{ Point { x: 1 }</p>");

    // Then
    assert_eq!((err.line, err.col), (1, 4), "{err}");
}

#[test]
fn stray_closing_brace_in_text_is_refused() {
    // When
    let err = parse_err("<p>a } b</p>");

    // Then
    assert_eq!((err.line, err.col), (1, 6), "{err}");
    assert!(err.message.contains('}'), "{err}");
}

#[test]
fn empty_text_hole_is_refused() {
    // When
    let err = parse_err("<p>{}</p>");

    // Then
    assert_eq!((err.line, err.col), (1, 4), "{err}");
}

#[test]
fn html_comment_is_refused_with_a_pointer() {
    // When
    let err = parse_err("<div><!-- note --></div>");

    // Then
    assert_eq!((err.line, err.col), (1, 6), "{err}");
    assert!(err.message.contains("<script>"), "{err}");
}

#[test]
fn unexpected_character_at_top_level_is_refused() {
    // When
    let err = parse_err("<div></div> trailing text");

    // Then
    assert_eq!((err.line, err.col), (1, 13), "{err}");
}

#[test]
fn void_element_close_tag_is_refused() {
    // When
    let err = parse_err("<div><br></br></div>");

    // Then
    assert_eq!((err.line, err.col), (1, 10), "{err}");
    assert!(err.message.contains("br"), "{err}");
}

// parser_title_test.go

#[test]
fn title_element_is_refused_as_a_platform_concern() {
    // When
    let err = parse_err("<title>My App</title>\n<div></div>");

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(err.message.contains("title"), "{err}");
}

// slot_removed_test.go

#[test]
fn slot_placeholder_is_refused_with_the_snippet_pointer() {
    // When
    let err = parse_err("<div><slot:header /></div>");

    // Then
    assert!(
        err.message.contains("snippet") && err.message.contains("render"),
        "{err}"
    );
}

#[test]
fn slot_block_is_refused_with_the_snippet_pointer() {
    // When
    let err = parse_err("<div>{slot header}<span>Title</span>{/slot}</div>");

    // Then
    assert!(err.message.contains("snippet"), "{err}");
}

#[test]
fn plain_slot_element_is_refused_too() {
    // When
    let err = parse_err("<div><slot /></div>");

    // Then
    assert!(err.message.contains("snippet"), "{err}");
}

#[test]
fn a_file_without_a_template_is_refused() {
    // When
    let err = parse_file("<script>\nlet x = 1;\n</script>\n").unwrap_err();

    // Then
    assert!(err.message.contains("template"), "{err}");
    assert_eq!(err.line, 3, "{err}");
}

#[test]
fn parse_ok_is_the_happy_path() {
    // When / Then
    assert_eq!(parse_ok("<div></div>").children.len(), 1);
}
