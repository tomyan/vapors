//! `<vapors:window onkeydown={…} />` (`src/codegen/window.rs`): each
//! listener is added to the page with `listen_window`, takes the same
//! handler wrapping an element's does, and is removed when the
//! component's owner is disposed (unmount).

use crate::common_codegen as codegen;

use codegen::{assert_has, generated_with};

const SCRIPT: &str =
    "let down = move |event: &mut Event| { let _ = event.key(); };\nlet up = move || {};";

#[test]
fn each_listener_is_added_to_the_window_with_the_elements_handler_wrapping() {
    // When
    let code = generated_with(SCRIPT, "<vapors:window onkeydown={down} onkeyup={up} />");

    // Then
    assert_has(
        &code,
        "listen_window(\"keydown\", Box::new(move |event: &mut Event| vapors_elements::__wisp::dom::handle(event, &down)))",
    );
    assert_has(
        &code,
        "listen_window(\"keyup\", Box::new(move |_event: &mut Event| vapors_elements::__wisp::dom::handle_without_event(&up)))",
    );
}

#[test]
fn each_listener_is_removed_when_the_owner_is_disposed() {
    // When
    let code = generated_with(SCRIPT, "<vapors:window onkeydown={down} />");

    // Then
    assert_has(
        &code,
        "vapors_signals::on_cleanup(move || { dom.borrow_mut().unlisten_window(listener); })",
    );
}
