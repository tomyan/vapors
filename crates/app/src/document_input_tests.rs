use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Cursor as Shown, Event};
use vapors_text::FontSystem;

use super::{DocumentInput, Handled};

/// A 100x20 CSS-pixel button at the top left with a pointer cursor,
/// updated at 2x, and a count of its clicks.
fn button() -> (Rc<RefCell<Document>>, Rc<RefCell<u32>>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; display: flex; } button { width: 100px; height: 20px; cursor: pointer; }",
    )
    .expect("parses");
    let body = doc.body();
    let button = doc.create_element("button");
    doc.append(body, button);
    let clicks = Rc::new(RefCell::new(0));
    let counter = Rc::clone(&clicks);
    doc.listen(
        button,
        "click",
        Box::new(move |_: &mut DomEvent| *counter.borrow_mut() += 1),
    );
    doc.update(Viewport {
        width: 200.0,
        height: 100.0,
        scale: 2.0,
    })
    .expect("updates");
    doc.take_changed();
    (Rc::new(RefCell::new(doc)), clicks)
}

#[test]
fn an_event_lands_in_css_pixels_and_a_button_where_the_pointer_last_moved() {
    // Given
    let (document, _) = button();
    let mut input = DocumentInput::new(2.0);

    // When
    let moved = input.at(&Event::PointerMoved { x: 40.0, y: 20.0 });
    input.handle(&document, &Event::PointerMoved { x: 40.0, y: 20.0 });

    // Then
    assert_eq!(moved, Some([20.0, 10.0]));
    assert_eq!(
        input.at(&Event::PrimaryButton { pressed: true }),
        Some([20.0, 10.0])
    );
    assert_eq!(input.at(&Event::PointerLeft), None);
}

#[test]
fn a_press_and_release_on_the_button_clicks_it() {
    // Given
    let (document, clicks) = button();
    let mut input = DocumentInput::new(2.0);
    input.handle(&document, &Event::PointerMoved { x: 40.0, y: 20.0 });

    // When
    input.handle(&document, &Event::PrimaryButton { pressed: true });
    input.handle(&document, &Event::PrimaryButton { pressed: false });

    // Then
    assert_eq!(*clicks.borrow(), 1);
}

#[test]
fn moving_onto_the_button_shows_its_cursor_once_and_off_it_the_default() {
    // Given
    let (document, _) = button();
    let mut input = DocumentInput::new(2.0);

    // When
    let onto = input.handle(&document, &Event::PointerMoved { x: 40.0, y: 20.0 });
    let along = input.handle(&document, &Event::PointerMoved { x: 60.0, y: 20.0 });
    let off = input.handle(&document, &Event::PointerMoved { x: 300.0, y: 150.0 });

    // Then
    assert_eq!(onto.cursor, Some(Shown::Pointer));
    assert_eq!(
        along,
        Handled {
            changed: along.changed,
            cursor: None
        }
    );
    assert_eq!(off.cursor, Some(Shown::Default));
}
