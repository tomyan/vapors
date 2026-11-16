//! The worked counter on a real Document: a tap on its button runs the
//! compiled handler, whose signal's effect sets the text node, while
//! dispatch holds no borrow of the document.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, viewport};
use vapors_compiler_examples::counter::{Counter, CounterProps, SCOPE};
use vapors_compiler_examples::flex_body::{FlexBody, FlexBodyProps};
use vapors_elements::Document;

#[test]
fn tapping_the_button_counts_and_the_next_update_shows_it() {
    // Given
    let doc = Rc::new(RefCell::new(document()));
    // body unmargined and flex, from a compiled sheet: one added from
    // Rust beside the analysed counter is refused (R1b)
    let root = doc.borrow().root();
    let _sheet = FlexBody::mount(&doc, root, FlexBodyProps::default());
    let body = doc.borrow().body();
    let counter = Counter::mount(&doc, body, CounterProps::default());
    doc.borrow_mut()
        .update(viewport(200.0, 100.0))
        .expect("updates");
    doc.borrow_mut().take_changed();

    // When
    Document::tapped(&doc, [3.0, 3.0]);
    Document::tapped(&doc, [3.0, 3.0]);

    // Then
    assert_eq!(counter.count.get(), 2);
    assert_eq!(
        doc.borrow().inner_html(body),
        format!("<button data-wisp=\"{SCOPE}\">Clicked 2 times</button>")
    );
    assert!(
        doc.borrow_mut().take_changed(),
        "the text change asks for a frame"
    );
    assert!(doc.borrow_mut().update(viewport(200.0, 100.0)).is_ok());
}
