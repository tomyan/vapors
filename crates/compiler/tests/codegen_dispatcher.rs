//! `#[dispatcher(names…)] let events;` in a script: a `Dispatcher` for
//! the component's own events, declared to the tree, cloned into the
//! handlers that use it; written any other way it is refused.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, generated_with, refusal};

#[test]
fn a_dispatcher_declares_its_events_and_reaches_handlers() {
    // Given
    let script = "#[dispatcher(optionchoose, comboboxdismiss)]\nlet events;\nlet choose = move |event: &mut Event| -> Result<(), vapors_dom::DispatchRefused> { if let Some(at) = event.target() { events.dispatch(at, Event::new(\"optionchoose\"))?; } Ok(()) };";

    // When
    let code = generated_with(script, "<button onclick={choose}>Go</button>");

    // Then
    assert_has(
        &code,
        "let events = Dispatcher::new(dom.dyn_dom(), &[\"optionchoose\", \"comboboxdismiss\"]);",
    );
    // the type lists them, and mount declares them first
    assert_has(
        &code,
        "events.extend([\"optionchoose\", \"comboboxdismiss\"]);",
    );
    assert_has(&code, "if seen.insert(std::any::type_name::<Self>()) {");
    assert_has(&code, "dom.borrow_mut().declare_events(&Self::events());");
}

#[test]
fn a_dispatcher_written_otherwise_is_refused_naming_the_form() {
    for (script, needle) in [
        ("#[dispatcher]\nlet events;", "names the events"),
        ("#[dispatcher(go)]\nlet events = 1;", "no initialiser"),
        ("#[dispatcher(go)]\nlet (a, b);", "a single name"),
        ("#[dispatcher(\"go\")]\nlet events;", "names the events"),
        ("#[prop]\n#[dispatcher(go)]\nlet events;", "stands alone"),
        (
            "#[dispatcher(go)]\nlet events;\n#[dispatcher(stop)]\nlet more;",
            "a second #[dispatcher]",
        ),
    ] {
        let why = refusal(&format!("<script>\n{script}\n</script>\n<p>x</p>")).to_string();
        assert!(why.contains(needle), "{script}: {why}");
    }
}
