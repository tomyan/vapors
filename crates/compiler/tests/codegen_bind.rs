//! Translated from sumi's `codegen_bind_test.go`: `bind:value` and
//! `bind:checked` wire the control's event to the signal and patch
//! the property from the signal; a handler for the same event beside
//! a binding is refused naming both.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated_with, refusal};

const SCRIPT: &str = "let name: Signal<String> = Signal::new(String::new());\nlet on: Signal<bool> = Signal::new(false);";

#[test]
fn bind_value_on_text_input() {
    // When
    let code = generated_with(SCRIPT, "<input type=\"text\" bind:value={name} />");

    // Then: the update half, from the input event
    assert_has(
        &code,
        "listen(input0, \"input\", Box::new(move |event: &mut Event|",
    );
    assert_has(&code, "name.set(event.value().to_string());");
    // And the display half, from the signal
    assert_has(
        &code,
        "set_property(input0, \"value\", PropertyValue::Text(name.get()))",
    );
    assert_lacks(&code, "bind:value");
}

#[test]
fn bind_checked_on_checkbox() {
    // When
    let code = generated_with(SCRIPT, "<input type=\"checkbox\" bind:checked={on} />");

    // Then
    assert_has(
        &code,
        "listen(input0, \"change\", Box::new(move |event: &mut Event|",
    );
    assert_has(&code, "on.set(event.checked());");
    assert_has(
        &code,
        "set_property(input0, \"checked\", PropertyValue::Bool(on.get()))",
    );
}

#[test]
fn bind_value_on_select_uses_change() {
    // When
    let code = generated_with(
        SCRIPT,
        "<select bind:value={name}><option value=\"a\">A</option></select>",
    );

    // Then
    assert_has(
        &code,
        "listen(select0, \"change\", Box::new(move |event: &mut Event|",
    );
    assert_has(&code, "name.set(event.value().to_string());");
    assert_has(
        &code,
        "set_property(select0, \"value\", PropertyValue::Text(name.get()))",
    );
}

#[test]
fn bind_value_on_textarea_uses_input() {
    // When
    let code = generated_with(SCRIPT, "<textarea bind:value={name}></textarea>");

    // Then
    assert_has(&code, "listen(textarea0, \"input\"");
}

#[test]
fn bind_value_with_a_handler_for_the_same_event_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\nlet typed = move |_event: &mut Event| {{}};\n</script>\n<input bind:value={{name}} oninput={{typed}} />"
    ));

    // Then
    assert!(
        err.message.contains("bind:value") && err.message.contains("oninput"),
        "{err}"
    );
    assert_eq!((err.line, err.col), (6, 8), "{err}");
}

#[test]
fn bind_on_an_unknown_property_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<input bind:foo={{name}} />"
    ));

    // Then
    assert!(
        err.message.contains("bind:value") && err.message.contains("bind:checked"),
        "{err}"
    );
    assert_eq!((err.line, err.col), (5, 8), "{err}");
}

#[test]
fn bind_on_an_element_that_is_not_a_control_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<div bind:value={{name}}></div>"
    ));

    // Then
    assert!(err.message.contains("input"), "{err}");
}
