//! The generated file is included into the user's crate, where clippy
//! pedantic runs on it. What the compiler writes itself is clean; the
//! lints that depend on types it cannot see are allowed on the one
//! function they apply to, with the reason in the attribute. The
//! compiled examples crate is the check that the whole file passes.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated, generated_with};

const PROP: &str = "#[prop]\nlet label: String = \"Count\".to_string();";

#[test]
fn an_empty_props_struct_derives_default() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "#[derive(Default)]\npub struct CounterProps {}");
    assert_lacks(&code, "impl Default for CounterProps");
}

#[test]
fn props_defaults_are_written_out_with_the_reason_they_are_not_derived() {
    // When
    let code = generated_with(PROP, "<span>{label}</span>");

    // Then
    assert_has(
        &code,
        "#[allow(clippy::derivable_impls, reason = \"the defaults are the #[prop] initialisers as written in the .wisp\")]\nimpl Default for CounterProps",
    );
}

#[test]
fn cloning_a_captured_binding_is_allowed_with_the_reason() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);\nlet increment = move || count.update(|n| *n += 1);";

    // When
    let code = generated_with(script, "<button onclick={increment}>+</button>");

    // Then
    assert_has(&code, "clippy::clone_on_copy");
    assert_has(
        &code,
        "reason = \"a closure gets a clone of each binding it captures; the compiler cannot see which are Copy\"",
    );
}

#[test]
fn a_text_hole_is_allowed_to_call_to_string_on_a_string() {
    // When
    let code = generated("<span>{1 + 1}</span>");

    // Then
    assert_has(&code, "clippy::implicit_clone");
    assert_has(
        &code,
        "reason = \"a text hole or expression attribute shows any Display value through to_string; the compiler cannot see when it is already a String\"",
    );
}

#[test]
fn a_child_component_is_allowed_a_props_update_that_may_be_complete() {
    // When
    let code = generated("<Greeting name=\"Sumi\" />");

    // Then
    assert_has(&code, "clippy::needless_update");
    assert_has(
        &code,
        "reason = \"a child's props end with ..Default::default(); the compiler cannot see when every prop is already given\"",
    );
}

#[test]
fn no_allowance_is_written_for_code_that_was_not_generated() {
    // When
    let code = generated_with(PROP, "<div>static</div>");

    // Then
    assert_lacks(&code, "clone_on_copy");
    assert_lacks(&code, "implicit_clone");
    assert_lacks(&code, "needless_update");
    assert_lacks(&code, "needless_pass_by_value");
}

#[test]
fn empty_props_are_taken_by_value_with_the_reason() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "clippy::needless_pass_by_value");
    assert_has(
        &code,
        "every component takes its props struct by value, empty or not, so parents mount them alike",
    );
}

#[test]
fn roots_is_must_use() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "#[must_use]\npub fn roots(&self) -> &[NodeId]");
}

#[test]
fn an_expression_attribute_is_allowed_to_call_to_string_on_a_string() {
    // Given
    let script = "let theme: Signal<String> = Signal::new(String::new());";

    // When
    let code = generated_with(script, "<div class={theme.get()}></div>");

    // Then
    assert_has(&code, "clippy::implicit_clone");
}
