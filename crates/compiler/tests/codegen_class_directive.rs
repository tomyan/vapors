//! `class:name={bool}`: the element has class `name` while the
//! expression is true, beside its literal `class` names: the names are
//! the key's toggles and one effect toggles them.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated_with, refusal};

const SCRIPT: &str = "let on: Signal<bool> = Signal::new(false);";

#[test]
fn class_directives_are_toggled_by_one_effect() {
    // When
    let code = generated_with(
        SCRIPT,
        "<div class=\"card\" class:lit={on.get()} class:dim={!on.get()}></div>",
    );

    // Then
    assert_has(
        &code,
        "toggle_classes(div0, &[(\"lit\", on.get()), (\"dim\", !on.get())]);",
    );
    assert_lacks(&code, "set_attribute(div0, \"class\"");
    assert_eq!(code.matches("Effect::new(").count(), 1, "{code}");
}

#[test]
fn a_class_directive_alone_is_a_toggle_of_a_key_without_classes() {
    // When
    let code = generated_with(SCRIPT, "<div class:lit={on.get()}></div>");

    // Then
    assert_has(&code, "classes: &[], toggles: &[\"lit\"]");
    assert_has(&code, "toggle_classes(div0, &[(\"lit\", on.get())]);");
}

#[test]
fn a_class_directive_with_a_quoted_value_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<div class:lit=\"yes\"></div>"
    ));

    // Then
    assert_eq!(
        err.to_string(),
        "counter.wisp:4:6: `class:lit` takes a bool expression: write class:lit={condition}"
    );
}

#[test]
fn a_class_directive_whose_name_is_not_a_class_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<div class:a.b={{on.get()}}></div>"
    ));

    // Then
    assert_eq!(
        err.to_string(),
        "counter.wisp:4:6: `class:a.b` names class `a.b`: a class directive's name is letters, digits, `-` and `_`"
    );
}

#[test]
fn a_dynamic_class_beside_a_directive_is_refused() {
    // When
    let err = refusal_with_script("<div class={\"a\".to_owned()} class:lit={on.get()}></div>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:6: `class={..}` is refused: the compiler must see every class an element can have; write class=\"name\" for fixed classes and class:name={condition} for each class that toggles"
    );
}

#[test]
fn a_directive_naming_a_class_the_literal_already_sets_is_refused() {
    // When
    let err = refusal_with_script("<div class=\"card lit\" class:lit={on.get()}></div>");

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:23: `class:lit` toggles a class the literal `class` already sets: remove `lit` from `class=\"...\"`"
    );
}

#[test]
fn a_literal_class_in_other_case_is_the_directives_literal_class() {
    // When
    let code = generated_with(SCRIPT, "<div Class=\"card\" class:lit={on.get()}></div>");

    // Then
    assert_has(&code, "classes: &[\"card\"], toggles: &[\"lit\"]");
    assert_lacks(&code, "\"Class\"");
}

/// The refusal of `template` after `SCRIPT`; the template starts on
/// line 4.
fn refusal_with_script(template: &str) -> String {
    refusal(&format!("<script>\n{SCRIPT}\n</script>\n{template}")).to_string()
}
