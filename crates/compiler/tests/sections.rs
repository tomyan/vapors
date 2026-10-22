//! Translated from sumi's `parser/section/section_test.go` and
//! `section_offset_test.go`. Differences: there is no imports section
//! (Rust `use` lines go in `<script>`); sumi silently drops a section
//! whose close tag is missing and allows two `<script>` blocks (the
//! second is template text), both refused here; a file without a
//! template is refused (sumi parses it as empty). Offsets are byte
//! offsets into the file, `None` for an absent section, and the
//! template keeps its position too.

mod common;

use common::parse_file;

#[test]
fn parse_template_only() {
    // When
    let component = parse_file("<span>Hello</span>").unwrap();

    // Then
    assert!(component.script.is_none());
    assert!(component.style.is_none());
    assert_eq!(component.template.children.len(), 1);
}

#[test]
fn parse_all_three_sections() {
    // Given
    let input = "<script>\nlet count: Signal<i32> = Signal::new(0);\n</script>\n\n<style>\n.title { color: green; }\n</style>\n\n<span>Hello</span>";

    // When
    let component = parse_file(input).unwrap();

    // Then
    assert_eq!(
        component.script.as_ref().unwrap().text,
        "\nlet count: Signal<i32> = Signal::new(0);\n"
    );
    assert_eq!(
        component.style.as_ref().unwrap().text,
        "\n.title { color: green; }\n"
    );
    assert_eq!(component.template.children.len(), 1);
}

#[test]
fn parse_script_and_template() {
    // When
    let component =
        parse_file("<script>\nlet name = 1;\n</script>\n\n<span>Hello, {name}</span>").unwrap();

    // Then
    assert_eq!(component.script.as_ref().unwrap().text, "\nlet name = 1;\n");
    assert!(component.style.is_none());
}

#[test]
fn parse_empty_input_is_refused() {
    // When
    let err = parse_file("").unwrap_err();

    // Then
    assert!(err.message.contains("template"), "{err}");
}

#[test]
fn parse_whitespace_between_sections() {
    // When
    let component =
        parse_file("<script>\nlet x = 1;\n</script>\n\n\n<span>Hello</span>\n  ").unwrap();

    // Then
    assert_eq!(component.script.as_ref().unwrap().text, "\nlet x = 1;\n");
    assert_eq!(component.template.children.len(), 1);
}

#[test]
fn style_may_come_before_script() {
    // When
    let component =
        parse_file("<style>\n.a{}\n</style>\n<script>\nlet x = 1;\n</script>\n<span>Hi</span>")
            .unwrap();

    // Then
    assert_eq!(component.style.as_ref().unwrap().text, "\n.a{}\n");
    assert_eq!(component.script.as_ref().unwrap().text, "\nlet x = 1;\n");
}

#[test]
fn style_text_passes_through_untouched() {
    // Given: braces, a Rust-looking hole and odd spacing in CSS
    let css = "\n  .x { content: \"{not a hole}\"; }\n\n";

    // When
    let component = parse_file(&format!("<style>{css}</style>\n<div></div>")).unwrap();

    // Then
    assert_eq!(component.style.as_ref().unwrap().text, css);
}

// section_offset_test.go

#[test]
fn script_offset_points_at_its_content() {
    // Given
    let input = "<script>\nlet x = 1;\n</script>\n<span>Hi</span>";

    // When
    let component = parse_file(input).unwrap();

    // Then
    let script = component.script.unwrap();
    assert_eq!(script.offset, 8);
    assert_eq!(
        &input[script.offset..script.offset + script.text.len()],
        script.text
    );
}

#[test]
fn style_offset_accounts_for_the_script_before_it() {
    // Given
    let input = "<script>\nlet x = 1;\n</script>\n<style>\n.a{}\n</style>\n<span>Hi</span>";

    // When
    let component = parse_file(input).unwrap();

    // Then
    let style = component.style.unwrap();
    assert_eq!(
        &input[style.offset..style.offset + style.text.len()],
        style.text
    );
}

#[test]
fn template_offset_points_at_the_trimmed_template() {
    // Given
    let input = "<script>\nlet x = 1;\n</script>\n\n<span>Hi</span>\n";

    // When
    let component = parse_file(input).unwrap();

    // Then
    assert_eq!(component.template_offset, input.find("<span>").unwrap());
}

#[test]
fn template_only_file_starts_at_zero() {
    // When
    let component = parse_file("<span>Only a template</span>").unwrap();

    // Then
    assert_eq!(component.template_offset, 0);
}

#[test]
fn errors_in_the_template_are_positioned_in_the_file() {
    // Given: the template starts on line 4
    let input = "<script>\nlet x = 1;\n</script>\n<div>\n  <p>x</span>\n</div>";

    // When
    let err = parse_file(input).unwrap_err();

    // Then
    assert_eq!((err.line, err.col), (5, 7), "{err}");
}

// vapors refusals

#[test]
fn second_script_section_is_refused() {
    // When
    let err =
        parse_file("<script>\nlet a = 1;\n</script>\n<script>\nlet b = 2;\n</script>\n<div></div>")
            .unwrap_err();

    // Then
    assert_eq!((err.line, err.col), (4, 1), "{err}");
    assert!(err.message.contains("<script>"), "{err}");
}

#[test]
fn unclosed_script_section_is_refused() {
    // When
    let err = parse_file("<script>\nlet a = 1;\n<div></div>").unwrap_err();

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(err.message.contains("</script>"), "{err}");
}

#[test]
fn script_tag_with_attributes_is_refused() {
    // When
    let err = parse_file("<script lang=\"rust\">\n</script>\n<div></div>").unwrap_err();

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(err.message.contains("<script>"), "{err}");
}

#[test]
fn section_after_the_template_is_refused() {
    // When
    let err = parse_file("<div></div>\n<style>.a{}</style>").unwrap_err();

    // Then
    assert_eq!((err.line, err.col), (2, 1), "{err}");
    assert!(err.message.contains("before"), "{err}");
}
