//! Refusals at code generation, each naming `file:line:col` in the
//! `.wisp`: script and expression syntax errors mapped back through
//! the section offsets, signals without a type (a struct field needs
//! one), and the deferred features.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::refusal;

#[test]
fn signal_without_a_type_annotation_is_refused() {
    // When
    let err = refusal(
        "<script>\nlet x = 1;\nlet count = Signal::new(0);\n</script>\n<span>{count}</span>",
    );

    // Then
    assert_eq!((err.line, err.col), (3, 1), "{err}");
    assert!(
        err.message.contains("count") && err.message.contains("Signal<"),
        "{err}"
    );
}

#[test]
fn memo_without_a_type_annotation_is_refused() {
    // When
    let err =
        refusal("<script>\nlet doubled = Memo::new(|| 2);\n</script>\n<span>{doubled}</span>");

    // Then
    assert_eq!((err.line, err.col), (2, 1), "{err}");
    assert!(err.message.contains("doubled"), "{err}");
}

#[test]
fn prop_without_a_type_annotation_is_refused() {
    // When
    let err = refusal("<script>\n#[prop]\nlet label = \"x\";\n</script>\n<span>{label}</span>");

    // Then
    assert_eq!((err.line, err.col), (2, 1), "{err}");
    assert!(err.message.contains("label"), "{err}");
}

#[test]
fn prop_without_a_default_is_refused() {
    // When
    let err = refusal("<script>\n#[prop]\nlet label: String;\n</script>\n<span>{label}</span>");

    // Then
    assert_eq!(err.line, 2, "{err}");
    assert!(err.message.contains("default"), "{err}");
}

#[test]
fn a_script_syntax_error_is_positioned_in_the_file() {
    // When
    let err = refusal("<script>\nlet a = 1;\nlet b = ;\n</script>\n<span>x</span>");

    // Then
    assert_eq!((err.line, err.col), (3, 9), "{err}");
}

#[test]
fn an_expression_syntax_error_is_positioned_where_the_hole_ends() {
    // When
    let err = refusal(
        "<script>\nlet count: Signal<i32> = Signal::new(0);\n</script>\n<p>\n  total: {count +}\n</p>",
    );

    // Then
    assert_eq!((err.line, err.col), (5, 18), "{err}");
}

#[test]
fn a_condition_syntax_error_is_positioned_where_the_condition_ends() {
    // When
    let err = refusal(
        "<script>\nlet count: Signal<i32> = Signal::new(0);\n</script>\n{if count.get() >}<p>x</p>{/if}",
    );

    // Then
    assert_eq!((err.line, err.col), (4, 18), "{err}");
}

#[test]
fn snippets_and_render_are_refused_as_deferred() {
    // When
    let err = refusal("<div>{render footer()}</div>");

    // Then
    assert_eq!((err.line, err.col), (1, 6), "{err}");
    assert!(err.message.contains("deferred"), "{err}");
    let err = refusal("<div>{snippet footer()}<b>x</b>{/snippet}</div>");
    assert!(err.message.contains("deferred"), "{err}");
}

#[test]
fn component_children_are_refused_as_deferred() {
    // When
    let err = refusal("<Card><p>body</p></Card>");

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(err.message.contains("deferred"), "{err}");
}

#[test]
fn a_hole_reading_nothing_still_compiles_to_a_static_text() {
    // Given: no signals, a hole with a constant
    let code = codegen::generated("<span>{1 + 1}</span>");

    // Then: it is still an effect; the compiler does not try to prove constancy
    codegen::assert_has(&code, "let value = (1 + 1).to_string();");
}

#[test]
fn prop_with_a_destructuring_pattern_is_refused() {
    // When
    let err =
        refusal("<script>\n#[prop]\nlet (a, b): (i32, i32) = (0, 0);\n</script>\n<span>{a}</span>");

    // Then
    assert!(err.message.contains("#[prop] needs a single name"), "{err}");
    assert_eq!(err.line, 2, "{err}");
}
