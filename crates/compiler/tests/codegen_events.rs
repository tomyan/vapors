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
    assert_has(&code, "vapors_dom::handle_without_event(&increment)");
}

#[test]
fn event_aware_handler_is_called_with_the_event_and_its_result_reported() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);\nlet handle_click = move |event: &mut Event| { count.update(|n| *n += 1); event.stop_propagation(); };";

    // When
    let code = generated_with(
        script,
        "<div onclick={handle_click}><span>{count}</span></div>",
    );

    // Then
    assert_has(
        &code,
        "listen(div0, \"click\", Box::new(move |event: &mut Event| vapors_dom::handle(event, &handle_click)))",
    );
    assert_lacks(&code, "handle_click()");
}

#[test]
fn inline_closure_handler_is_called_with_the_event() {
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
        "Box::new(move |event: &mut Event| vapors_dom::handle(event, &move |_event| count.set(0)))",
    );
}

#[test]
fn dynamic_attributes_are_patched_in_effects() {
    // Given
    let script = "let locked: Signal<bool> = Signal::new(false);\nlet theme: Signal<String> = Signal::new(\"dark\".into());\nlet cls = move || format!(\"panel {}\", theme.get());";

    // When
    let code = generated_with(
        script,
        "<div title={cls()} aria-disabled={locked.get()}><span>body</span></div>",
    );

    // Then
    assert_has(&code, "set_attribute_if_changed(div0, \"title\", &value)");
    assert_has(&code, "let shown = &(cls());");
    assert_has(
        &code,
        "set_attribute_if_changed(div0, \"aria-disabled\", &value)",
    );
    assert_has(&code, "let shown = &(locked.get());");
    assert_eq!(code.matches("Effect::new(").count(), 2, "{code}");
}

#[test]
fn static_attributes_are_fixed_by_the_key_and_set_once_from_it() {
    // When
    let code = generated_with("", "<div title=\"row\" id=\"main\"></div>");

    // Then
    assert_has(
        &code,
        "attributes: &[(\"title\", \"row\"), (\"id\", \"main\")]",
    );
    assert_lacks(&code, "set_attribute(");
    assert_lacks(&code, "_if_changed");
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
    assert_has(
        &code,
        "listen(input0, \"input\", Box::new(move |event: &mut Event| vapors_dom::handle(event, &typed)))",
    );
}

#[test]
fn a_handler_that_assigns_to_what_it_captures_is_refused_naming_it() {
    // Given: a script closure, and one written inline, each changing `n`
    let cases = [
        "<script>\nlet mut n = 0;\nlet bump = move || n += 1;\n</script>\n<button onclick={bump}>+</button>",
        "<script>\nlet mut n = 0;\n</script>\n<button onclick={move |_event| { n = 1; }}>+</button>",
        "<script>\nlet mut n = (0, 0);\nlet bump = move |_event: &mut Event| { let r = &mut n.0; *r = 1; };\n</script>\n<button onclick={bump}>+</button>",
    ];
    for case in cases {
        // When
        let why = refusal(case).to_string();

        // Then
        assert!(why.contains("assigns to `n`, which it captures"), "{why}");
        assert!(why.contains("keep `n` in a signal"), "{why}");
    }
}

#[test]
fn a_handler_that_assigns_only_to_its_own_bindings_is_taken() {
    // When: `total` is the handler's own, and `count` changes through a signal
    let code = generated_with(
        "let count: Signal<i32> = Signal::new(0);\nlet bump = move || { let mut total = 0; total += 1; count.set(total); };",
        "<button onclick={bump}>+</button>",
    );

    // Then
    assert_has(&code, "vapors_dom::handle_without_event(&bump)");
}
