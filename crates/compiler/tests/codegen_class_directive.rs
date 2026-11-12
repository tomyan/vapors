//! `class:name={bool}`: the element has class `name` while the
//! expression is true, beside its literal `class` names, all set by
//! one effect.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated_with, refusal};

const SCRIPT: &str = "let on: Signal<bool> = Signal::new(false);";

#[test]
fn class_directives_and_literal_classes_are_set_by_one_effect() {
    // When
    let code = generated_with(
        SCRIPT,
        "<div class=\"card\" class:lit={on.get()} class:dim={!on.get()}></div>",
    );

    // Then
    assert_has(
        &code,
        "let value = vapors_dom::class_names(\"card\", &[(\"lit\", on.get()), (\"dim\", !on.get())]);",
    );
    assert_has(&code, "set_attribute_if_changed(div0, \"class\", &value);");
    assert_lacks(&code, "set_attribute(div0, \"class\", \"card\")");
    assert_eq!(code.matches("Effect::new(").count(), 1, "{code}");
}

#[test]
fn a_class_directive_alone_starts_from_no_classes() {
    // When
    let code = generated_with(SCRIPT, "<div class:lit={on.get()}></div>");

    // Then
    assert_has(
        &code,
        "let value = vapors_dom::class_names(\"\", &[(\"lit\", on.get())]);",
    );
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
