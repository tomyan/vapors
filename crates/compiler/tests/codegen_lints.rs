//! The generated file is included into the user's crate, where clippy
//! pedantic runs on it. What the compiler writes itself is clean. The
//! lints that depend on types it cannot see are allowed on the one
//! generated `let` that trips them, with the reason in the attribute,
//! and never on a statement or expression the user wrote. The compiled
//! examples crate is the check that the whole file passes.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated, generated_with};

const PROP: &str = "#[prop]\nlet label: String = \"Count\".to_string();";
const CLONES: &str =
    "a closure gets a clone of each binding it captures; the compiler cannot see which are Copy";
const TO_STRING: &str = "a text hole or expression attribute shows any Display value through to_string; the compiler cannot see when it is already a String";
const UPDATE: &str = "a child's props end with ..Default::default(); the compiler cannot see when every prop is already given";

fn allow(lint: &str, reason: &str) -> String {
    format!("#[allow(clippy::{lint}, reason = {reason:?})]")
}

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
fn a_generated_clone_carries_its_own_allowance() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);\nlet increment = move || count.update(|n| *n += 1);";

    // When
    let code = generated_with(script, "<button onclick={increment}>+</button>");

    // Then
    assert_has(
        &code,
        &format!(
            "{}\nlet increment = increment.clone();",
            allow("clone_on_copy", CLONES)
        ),
    );
}

#[test]
fn a_text_hole_allows_only_the_generated_to_string() {
    // When
    let code = generated("<span>{1 + 1}</span>");

    // Then: the hole's expression is its own statement, outside the allowance
    assert_has(&code, "let shown = &(1 + 1);");
    assert_has(
        &code,
        &format!(
            "{}\nlet value = shown.to_string();",
            allow("implicit_clone", TO_STRING)
        ),
    );
}

#[test]
fn an_expression_attribute_allows_only_the_generated_to_string() {
    // Given
    let script = "let theme: Signal<String> = Signal::new(String::new());";

    // When
    let code = generated_with(script, "<div class={theme.get()}></div>");

    // Then
    assert_has(&code, "let shown = &(theme.get());");
    assert_has(
        &code,
        &format!(
            "{}\nlet value = shown.to_string();",
            allow("implicit_clone", TO_STRING)
        ),
    );
}

#[test]
fn a_child_component_allows_only_the_generated_props_literal() {
    // Given
    let script = "let who: String = String::new();";

    // When
    let code = generated_with(script, "<Greeting name={who.clone()} greeting=\"Hi\" />");

    // Then: the user's expression is evaluated before the allowance
    assert_has(&code, "let greeting0_name = who.clone();");
    assert_has(
        &code,
        &format!(
            "{}\nlet greeting0_props = GreetingProps {{",
            allow("needless_update", UPDATE)
        ),
    );
    assert_has(&code, "name: greeting0_name,");
    assert_has(&code, "greeting: \"Hi\".into(),");
}

#[test]
fn no_allowance_covers_the_users_script() {
    // Given: a Copy clone the user wrote, in a component with a hole,
    // a handler and a child, so every allowance is generated
    let script = "let n: i32 = 1;\nlet m = n.clone();\nlet count: Signal<i32> = Signal::new(0);\nlet bump = move || count.set(m);";

    // When
    let code = generated_with(
        script,
        "<button onclick={bump}>{count}</button><Greeting />",
    );

    // Then
    assert_has(&code, "let m = n.clone();");
    for (lint, reason) in [
        ("clone_on_copy", CLONES),
        ("implicit_clone", TO_STRING),
        ("needless_update", UPDATE),
    ] {
        assert_lacks(
            &code,
            &format!("{}\npub fn mount_before", allow(lint, reason)),
        );
        assert_lacks(
            &code,
            &format!("{}\nlet m = n.clone();", allow(lint, reason)),
        );
    }
}

#[test]
fn no_allowance_is_written_for_code_that_was_not_generated() {
    // When
    let code = generated_with(PROP, "<div>static</div>");

    // Then
    assert_lacks(&code, "#[allow(clippy::clone_on_copy");
    assert_lacks(&code, "#[allow(clippy::implicit_clone");
    assert_lacks(&code, "#[allow(clippy::needless_update");
    assert_lacks(&code, "#[allow(clippy::needless_pass_by_value");
}

#[test]
fn empty_props_are_taken_by_value_with_the_reason() {
    // When
    let code = generated("<div></div>");

    // Then: a parameter lint, so it cannot cover anything in the script
    assert_has(
        &code,
        "#[allow(clippy::needless_pass_by_value, reason = \"every component takes its props struct by value, empty or not, so parents mount them alike\")]\npub fn mount_before",
    );
}

#[test]
fn mount_before_allows_its_own_length_with_the_reason() {
    // When
    let code = generated("<div></div>");

    // Then: a lint about the function's size, so it cannot hide another
    assert_has(
        &code,
        "#[allow(clippy::too_many_lines, reason = \"mount_before builds the whole template, so its length follows the template's size\")]",
    );
}

#[test]
fn roots_is_must_use() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "#[must_use]\npub fn roots(&self) -> Vec<NodeId>");
}

#[test]
fn props_that_are_all_signals_are_taken_by_value_with_the_reason() {
    // When: the only prop is a signal, which is Copy, so mount does not
    // consume its props
    let code = generated_with(
        "#[prop]\nlet outside: Signal<i32> = Signal::new(0);",
        "<div>{outside}</div>",
    );

    // Then
    assert_has(&code, "#[allow(clippy::needless_pass_by_value");
}
