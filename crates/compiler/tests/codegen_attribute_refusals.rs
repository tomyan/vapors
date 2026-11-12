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

#[test]
fn a_boolean_attribute_with_the_literal_false_is_refused() {
    // When
    let err = refusal_with_flag("<button disabled=\"False\">x</button>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:9: `disabled=\"False\"` is on: an HTML boolean attribute is on by being present, whatever its value; omit it, or write `disabled=\"\"` on an element inside `{if ..}`"
    );
}

#[test]
fn a_boolean_attribute_with_another_literal_value_is_accepted() {
    // When
    let code = codegen::generated("<button disabled=\"disabled\" hidden=\"\">x</button>");

    // Then
    assert!(code.contains("\"disabled\""), "{code}");
}

#[test]
fn a_less_common_boolean_attribute_is_refused_from_an_expression() {
    // When
    let err = refusal_with_flag("<div nowrap={flag.get()}></div>");

    // Then
    assert!(err.contains("`nowrap={..}` would always be on"), "{err}");
}

#[test]
fn an_attribute_given_twice_in_other_case_is_refused() {
    // When
    let err = refusal_with_flag("<div title=\"a\" Title=\"b\"></div>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:16: attribute `Title` is given twice: keep one"
    );
}

#[test]
fn a_colon_name_on_a_component_is_refused_naming_the_fix() {
    // When
    let err = refusal_with_flag("<Card class:lit={flag.get()} />");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:7: `class:lit` is not a prop: a component tag takes plain props and `bind:name`; `class:` directives and `on:` listeners are for elements"
    );
}

#[test]
fn a_namespaced_attribute_is_refused_saying_namespaces_are_unsupported() {
    // When
    let err = refusal_with_flag("<div xlink:href=\"#a\"></div>");

    // Then
    assert!(
        err.contains(
            "namespaced attributes such as `xlink:href`, `xml:lang` and `xmlns:*` are not supported"
        ),
        "{err}"
    );
}

#[test]
fn a_class_attribute_from_an_expression_is_refused() {
    // When
    let err = refusal_with_flag("<div class={flag.get().to_string()}></div>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:6: `class={..}` is refused: the compiler must see every class an element can have; write class=\"name\" for fixed classes and class:name={condition} for each class that toggles"
    );
}

#[test]
fn a_style_attribute_from_an_expression_is_refused() {
    // When
    let err = refusal_with_flag("<div style={flag.get().to_string()}></div>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:6: `style={..}` is refused: the compiler must see every declaration an element can have; write the declarations under a class in the <style> section and toggle it with class:name={condition}"
    );
}
