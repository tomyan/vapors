//! The player's focus fixup: the text fixture with an Escape handler
//! that hides the focused field. The step's update leaves the field
//! without a box, and the settle the player runs after it moves focus
//! to the body with the field's `change`, `blur` and `focusout`, as an
//! app's frame does.

use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;
use vapors_elements_corpus::Fixture;
use vapors_elements_corpus::events_script::parse;
use vapors_elements_corpus::events_vapors::play;

#[path = "corpus_support/events.rs"]
#[expect(dead_code, reason = "this test reads no fixture file (`read`)")]
mod events;

use events::{VIEWPORT, fixture};

/// The text fixture, with Escape on `input#name` hiding it.
fn hides_on_escape() -> Fixture {
    Fixture {
        mount: |doc, parent| {
            (fixture("text").mount)(doc, parent);
            let field = named(&doc.borrow(), "name");
            let weak = Rc::downgrade(doc);
            let hide = move |e: &mut Event| {
                if let (Some(doc), "Escape") = (weak.upgrade(), e.key()) {
                    // `[hidden] { display: none }` (ua.css): the field is a
                    // template element, whose `style` its key fixes
                    doc.borrow_mut().set_attribute(field, "hidden", "");
                }
            };
            doc.borrow_mut().listen(field, "keydown", Box::new(hide));
        },
        ..fixture("text")
    }
}

/// The element with `id`.
fn named(doc: &Document, id: &str) -> NodeId {
    let found = doc
        .elements()
        .into_iter()
        .find(|n| doc.attribute(*n, "id") == Some(id));
    found.expect("the text fixture has the element")
}

#[test]
fn the_player_settles_focus_after_a_step_hides_the_focused_field() {
    // Given: the field clicked into and typed in
    let script = [
        "listen change focusout",
        "click 120 12",
        "expect value input#name Warm",
        "type \"th\"",
        "expect value input#name Warmth",
        "key Escape",
        "expect event change input#name",
    ]
    .join("\n");
    let script = parse("text", &script).expect("parses");

    // When
    let ours = play(&hides_on_escape(), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then: the Escape step commits and blurs the field, and the body has focus
    let escape = ours.last().expect("a step");
    let heard: Vec<_> = escape
        .events
        .iter()
        .map(|(kind, _)| kind.as_str())
        .collect();
    assert_eq!(heard, ["change", "focusout"]);
    assert_eq!(escape.focus, "body");
}
