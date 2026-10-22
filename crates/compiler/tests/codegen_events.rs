//! Translated from sumi's `codegen_events_test.go`: `on<type>={expr}`
//! attributes become listeners, a zero-argument handler is wrapped,
//! an event-aware one is passed as is, and expression attributes are
//! patched in effects.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated_with, refusal};

#[test]
fn zero_arg_handler_is_wrapped() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);\nlet increment = move || count.update(|n| *n += 1);";

    // When
    let code = generated_with(
        script,
        "<div onclick={increment}><span>{count}</span></div>",
    );

    // Then
    assert_has(
        &code,
        "listen(div0, \"click\", Box::new(move |_event: &mut Event|",
    );
    assert_has(&code, "increment()");
}

#[test]
fn event_aware_handler_is_passed_directly() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);\nlet handle_click = move |event: &mut Event| { count.update(|n| *n += 1); event.stop_propagation(); };";

    // When
    let code = generated_with(
        script,
        "<div onclick={handle_click}><span>{count}</span></div>",
    );

    // Then
    assert_has(&code, "listen(div0, \"click\", Box::new(handle_click))");
    assert_lacks(&code, "handle_click()");
}

#[test]
fn inline_closure_handler_is_passed_directly() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);";

    // When
    let code = generated_with(
        script,
        "<button onclick={move |_event| count.set(0)}>reset</button>",
    );

    // Then
    assert_has(
        &code,
        "listen(button0, \"click\", Box::new(move |_event| count.set(0)))",
    );
}

#[test]
fn dynamic_attributes_are_patched_in_effects() {
    // Given
    let script = "let locked: Signal<bool> = Signal::new(false);\nlet theme: Signal<String> = Signal::new(\"dark\".into());\nlet cls = move || format!(\"panel {}\", theme.get());";

    // When
    let code = generated_with(
        script,
        "<div class={cls()} disabled={locked.get()}><span>body</span></div>",
    );

    // Then
    assert_has(&code, "set_attribute(div0, \"class\", &value)");
    assert_has(&code, "let value = (cls()).to_string();");
    assert_has(&code, "set_attribute(div0, \"disabled\", &value)");
    assert_has(&code, "let value = (locked.get()).to_string();");
    assert_eq!(code.matches("Effect::new(").count(), 2, "{code}");
}

#[test]
fn static_attributes_are_set_once() {
    // When
    let code = generated_with("", "<div class=\"row\" id=\"main\"></div>");

    // Then
    assert_has(&code, "set_attribute(div0, \"class\", \"row\");");
    assert_has(&code, "set_attribute(div0, \"id\", \"main\");");
    assert_lacks(&code, "Effect::new(");
}

#[test]
fn an_event_attribute_with_a_string_value_is_refused() {
    // When
    let err = refusal("<button onclick=\"save\">x</button>");

    // Then
    assert_eq!((err.line, err.col), (1, 9), "{err}");
    assert!(err.message.contains("onclick={"), "{err}");
}

#[test]
fn the_event_name_is_the_attribute_name_after_on() {
    // Given
    let script = "let typed = move |event: &mut Event| { let _ = event.value(); };";

    // When
    let code = generated_with(script, "<input oninput={typed} />");

    // Then
    assert_has(&code, "listen(input0, \"input\", Box::new(typed))");
}
