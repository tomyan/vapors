//! Clicks given through `DocumentInput` as the platform counts them: an
//! Alt-click reaches the document's `click` with Alt held, and the
//! second click in a row fires `dblclick` (a card's label resets its
//! control on either).

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Modifiers, Platform};
use vapors_text::FontSystem;

use crate::DocumentInput;

/// A 100x20 label at the top left, logging its clicks.
fn label() -> (Rc<RefCell<Document>>, Rc<RefCell<Vec<String>>>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } div { width: 100px; height: 20px; }",
    )
    .expect("parses");
    let body = doc.body();
    let label = doc.create_element("div");
    doc.append(body, label);
    let log = Rc::new(RefCell::new(Vec::new()));
    for name in ["pointerdown", "click", "dblclick"] {
        let log = Rc::clone(&log);
        doc.listen(
            label,
            name,
            Box::new(move |e: &mut DomEvent| {
                let alt = if e.modifiers().alt { " alt" } else { "" };
                log.borrow_mut().push(format!("{}{alt}", e.name()));
            }),
        );
    }
    doc.update(Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    })
    .expect("updates");
    (Rc::new(RefCell::new(doc)), log)
}

fn click(input: &mut DocumentInput, doc: &Rc<RefCell<Document>>, n: u32, modifiers: Modifiers) {
    let clicks = NonZeroU32::new(n).expect("a count");
    for pressed in [true, false] {
        let event = Event::PrimaryButton {
            pressed,
            clicks,
            modifiers,
        };
        input
            .give(doc, &event, &Platform::detached())
            .expect("placed");
    }
}

#[test]
fn an_alt_click_and_a_double_click_reach_the_document_as_the_platform_counts_them() {
    // Given
    let (doc, log) = label();
    let mut input = DocumentInput::new();
    input
        .give(
            &doc,
            &Event::PointerMoved { x: 10.0, y: 10.0 },
            &Platform::detached(),
        )
        .expect("placed");

    // When: an Alt-click, then a plain click and its second in a row
    click(
        &mut input,
        &doc,
        1,
        Modifiers {
            alt: true,
            ..Modifiers::default()
        },
    );
    click(&mut input, &doc, 1, Modifiers::default());
    click(&mut input, &doc, 2, Modifiers::default());

    // Then
    assert_eq!(
        *log.borrow(),
        [
            "pointerdown alt",
            "click alt",
            "pointerdown",
            "click",
            "pointerdown",
            "click",
            "dblclick"
        ]
    );
}
