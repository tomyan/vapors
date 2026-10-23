//! Which calls ask for a frame: every change to what is shown, and not
//! a listener.

mod support;

use support::{document, element, text};
use vapors_dom::Dom;

/// One call on the document, by name.
type Change = Box<dyn Fn(&mut vapors_elements::Document)>;

#[test]
fn each_change_to_what_is_shown_asks_for_a_frame_and_listening_does_not() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    let words = text(&mut doc, div, "a");
    let changes: [(&str, Change); 6] = [
        ("set_text", Box::new(move |d| d.set_text(words, "b"))),
        (
            "set_attribute",
            Box::new(move |d| d.set_attribute(div, "class", "x")),
        ),
        (
            "set_property",
            Box::new(move |d| d.set_property(div, "value", vapors_dom::PropertyValue::Bool(true))),
        ),
        (
            "append",
            Box::new(move |d| {
                let n = d.create_element("div");
                d.append(div, n);
            }),
        ),
        (
            "insert_before",
            Box::new(move |d| {
                let n = d.create_text("c");
                d.insert_before(div, n, Some(words));
            }),
        ),
        ("remove", Box::new(move |d| d.remove(words))),
    ];

    for (name, change) in changes {
        doc.take_changed();

        // When
        change(&mut doc);

        // Then
        assert!(doc.take_changed(), "{name} asks for a frame");
    }
    doc.listen(div, "click", Box::new(|_: &mut vapors_dom::Event| {}));
    assert!(!doc.take_changed(), "a listener changes nothing shown");
}
