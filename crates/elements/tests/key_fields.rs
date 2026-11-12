//! `keydown` and `keyup` carry the modifiers held, the key's `code` and
//! whether it is an auto-repeat (review-fable-input M3), as the DOM's
//! `KeyboardEvent` does: a listener tells Shift+Enter from Enter, and
//! ignores a held key's repeats. The plain `key_pressed` sends no code
//! and no repeat.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::focus_page::{Page, page};
use vapors_dom::{Dom, Event};
use vapors_elements::{Document, Keystroke, Modifiers};

/// What a listener saw: the modifiers, code and repeat of each event.
type Seen = Rc<RefCell<Vec<(String, Modifiers, String, bool)>>>;

fn listen(page: &Page, event: &str, seen: &Seen) {
    let into = Rc::clone(seen);
    page.doc.borrow_mut().listen(
        page.save,
        event,
        Box::new(move |e: &mut Event| {
            into.borrow_mut().push((
                e.name().to_owned(),
                e.modifiers(),
                e.code().to_owned(),
                e.repeat(),
            ));
        }),
    );
}

fn shift() -> Modifiers {
    Modifiers {
        shift: true,
        ..Modifiers::default()
    }
}

#[test]
fn keydown_carries_the_modifiers_held() {
    // Given
    let page = page();
    Document::focus(&page.doc, Some(page.save));
    let seen = Seen::default();
    listen(&page, "keydown", &seen);

    // When
    Document::key_pressed(&page.doc, "Enter", shift());

    // Then
    assert_eq!(seen.borrow()[0].1, shift());
}

#[test]
fn keydown_and_keyup_carry_the_code_and_repeat() {
    // Given
    let page = page();
    Document::focus(&page.doc, Some(page.save));
    let seen = Seen::default();
    listen(&page, "keydown", &seen);
    listen(&page, "keyup", &seen);
    let held = Keystroke {
        key: "s",
        code: "KeyS",
        repeat: true,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::default()
        },
    };

    // When: a held Control+S repeats, then comes up
    Document::key_pressed_with(&page.doc, held);
    Document::key_released_with(
        &page.doc,
        Keystroke {
            repeat: false,
            ..held
        },
    );

    // Then
    assert_eq!(
        *seen.borrow(),
        [
            (
                "keydown".to_owned(),
                held.modifiers,
                "KeyS".to_owned(),
                true
            ),
            ("keyup".to_owned(), held.modifiers, "KeyS".to_owned(), false),
        ]
    );
}

#[test]
fn the_plain_key_press_carries_no_code_and_no_repeat() {
    // Given
    let page = page();
    Document::focus(&page.doc, Some(page.save));
    let seen = Seen::default();
    listen(&page, "keydown", &seen);

    // When
    Document::key_pressed(&page.doc, "a", Modifiers::default());

    // Then
    assert_eq!(
        *seen.borrow(),
        [(
            "keydown".to_owned(),
            Modifiers::default(),
            String::new(),
            false
        )]
    );
}
