//! Attributes that would compile to something other than what they
//! say are refused, naming `file:line:col` and the form to write: a
//! boolean attribute from an expression (always on), a colon-named
//! attribute that is not a directive, and `{` in a quoted value.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::refusal;

/// The refusal of `template` under a script with a `bool` signal
/// `flag`; the template starts on line 4.
fn refusal_with_flag(template: &str) -> String {
    refusal(&format!(
        "<script>\nlet flag: Signal<bool> = Signal::new(false);\n</script>\n{template}"
    ))
    .to_string()
}

#[test]
fn a_boolean_attribute_from_an_expression_is_refused() {
    // When
    let err = refusal_with_flag("<details open={flag.get()}></details>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:10: `open={..}` would always be on: an HTML boolean attribute is on by being present, so `open=\"false\"` is on too, and generated code cannot remove an attribute; write `open=\"\"` on an element inside `{if ..}`"
    );
}

#[test]
fn checked_from_an_expression_is_refused_naming_the_binding() {
    // When
    let err = refusal_with_flag("<input type=\"checkbox\" checked={flag.get()} />");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:24: `checked={..}` would always be on: an HTML boolean attribute is on by being present, so `checked=\"false\"` is on too, and generated code cannot remove an attribute; write `bind:checked={signal}`, or `checked=\"\"` on an element inside `{if ..}`"
    );
}

#[test]
fn a_boolean_attribute_name_is_matched_ignoring_case() {
    // When
    let err = refusal_with_flag("<button Disabled={flag.get()}>x</button>");

    // Then
    assert!(
        err.starts_with("counter.wisp:4:9: `Disabled={..}` would always be on"),
        "{err}"
    );
}

#[test]
fn a_colon_named_attribute_that_is_not_a_directive_is_refused() {
    // When
    let err = refusal_with_flag("<div style:color={flag.get()}></div>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:6: `style:color` is not a directive: a name with `:` is a directive, and an element takes `bind:value`, `bind:checked`, `bind:this` and `class:name={bool}`; a listener is `onclick={handler}`, with no colon"
    );
}

#[test]
fn a_svelte_event_directive_is_refused_naming_the_listener_form() {
    // When
    let err = refusal_with_flag("<button on:click={flag.get()}>x</button>");

    // Then
    assert!(
        err.starts_with("counter.wisp:4:9: `on:click` is not a directive:"),
        "{err}"
    );
}

#[test]
fn a_brace_in_a_quoted_attribute_value_is_refused() {
    // When
    let err = refusal_with_flag("<div\n  title=\"card {flag.get()}\"></div>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:5:15: the value of attribute `title` has a `{` in quotes, which is text, not an expression: write title={format!(\"..\")} to build it, or `&#123;` for a literal brace"
    );
}
