//! Translated from sumi's `codegen_rename_test.go`: a bare signal
//! name in a text hole becomes a read, and only when it stands alone
//! as a path (never inside another identifier, a field name, or a
//! string). sumi rewrites by scanning characters; here the expression
//! is parsed with syn and paths are rewritten, so the partial-match
//! cases hold by construction and are kept as a regression guard.
//! Difference: a signal method called explicitly (`{count.get()}`,
//! `{items.with(..)}`) is left alone rather than doubled.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated_with};

const SCRIPT: &str = "let count: Signal<i32> = Signal::new(0);\nlet items: Signal<Vec<i32>> = Signal::new(Vec::new());\nlet label = \"plain\";";

#[test]
fn replace_identifier_basic() {
    // When
    let code = generated_with(SCRIPT, "<span>{count + 1}</span>");

    // Then
    assert_has(&code, "let shown = &(count.get() + 1);");
}

#[test]
fn replace_identifier_in_function_call() {
    // When
    let code = generated_with(SCRIPT, "<span>{i32::abs(count)}</span>");

    // Then
    assert_has(&code, "let shown = &(i32::abs(count.get()));");
}

#[test]
fn replace_identifier_no_partial_match() {
    // Given
    let script = format!("{SCRIPT}\nlet discount = 1;\nlet counting = 2;");

    // When
    let code = generated_with(&script, "<span>{discount + counting}</span>");

    // Then
    assert_has(&code, "let shown = &(discount + counting);");
    assert_lacks(&code, "get()");
}

#[test]
fn replace_identifier_not_a_field_name() {
    // Given
    let script =
        format!("{SCRIPT}\nstruct Point {{ count: i32 }}\nlet point = Point {{ count: 1 }};");

    // When
    let code = generated_with(&script, "<span>{point.count}</span>");

    // Then
    assert_has(&code, "let shown = &(point.count);");
}

#[test]
fn replace_identifier_multiple_occurrences() {
    // When
    let code = generated_with(SCRIPT, "<span>{count * count}</span>");

    // Then
    assert_has(&code, "let shown = &(count.get() * count.get());");
}

#[test]
fn method_call_on_a_signal_reads_it_first() {
    // When
    let code = generated_with(SCRIPT, "<span>{items.len()}</span>");

    // Then
    assert_has(&code, "let shown = &(items.get().len());");
}

#[test]
fn an_explicit_signal_method_is_left_alone() {
    // When
    let code = generated_with(
        SCRIPT,
        "<span>{count.get()} {items.with(|v| v.len())}</span>",
    );

    // Then
    assert_has(&code, "let shown = &(count.get());");
    assert_has(&code, "let shown = &(items.with(|v| v.len()));");
    assert_lacks(&code, "get().get()");
}

#[test]
fn a_plain_variable_is_left_alone() {
    // When
    let code = generated_with(SCRIPT, "<span>{label}</span>");

    // Then
    assert_has(&code, "let shown = &(label);");
}

#[test]
fn attribute_expressions_are_raw() {
    // When
    let code = generated_with(SCRIPT, "<span title={count.get().to_string()}></span>");

    // Then
    assert_has(&code, "let shown = &(count.get().to_string());");
}

#[test]
fn conditions_and_iterables_are_raw() {
    // When
    let code = generated_with(SCRIPT, "{if count.get() > 0}<span>x</span>{/if}");

    // Then
    assert_has(&code, "move || count.get() > 0");
}
