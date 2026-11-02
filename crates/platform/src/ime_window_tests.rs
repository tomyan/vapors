//! The IME told what the field needs: allowed and placed for a field,
//! also one set before the window opened; dropped and allowed again when
//! a composition ends with a click outside it.

use std::cell::RefCell;

use super::*;
use winit::event::Ime;

/// A window that records what the IME was told.
#[derive(Default)]
pub(crate) struct Told(pub(crate) RefCell<Vec<String>>);

impl ImeTarget for Told {
    fn set_ime_allowed(&self, allowed: bool) {
        self.0.borrow_mut().push(format!("allowed {allowed}"));
    }

    fn set_ime_cursor_area(&self, at: [f64; 2], size: [f64; 2]) {
        self.0.borrow_mut().push(format!("area {at:?} {size:?}"));
    }
}

fn field(text: &str, selection: (u32, u32)) -> TextField {
    TextField {
        id: 1,
        text: text.into(),
        selection,
        area: [10.0, 20.0, 1.0, 16.0],
    }
}

#[test]
fn a_field_set_before_the_window_opens_gets_the_ime_when_it_opens() {
    // Given: the field set with no window yet
    let mut composition = Composition::default();
    mirror(
        None::<&Told>,
        &mut composition,
        Some(&field("Warm", (4, 4))),
    );

    // When
    let window = Told::default();
    opened(&window, &composition);

    // Then
    assert_eq!(
        *window.0.borrow(),
        ["allowed true", "area [10.0, 20.0] [1.0, 16.0]"]
    );
}

#[test]
fn a_composition_ended_by_a_click_outside_it_is_dropped_by_the_ime() {
    // Given: composing "a" after "Warm"
    let window = Told::default();
    let mut composition = Composition::default();
    mirror(
        Some(&window),
        &mut composition,
        Some(&field("Warm", (4, 4))),
    );
    drop(composition.edit(&Ime::Preedit("a".into(), Some((1, 1)))));
    mirror(
        Some(&window),
        &mut composition,
        Some(&field("Warma", (5, 5))),
    );
    window.0.borrow_mut().clear();

    // When: a click at 2
    mirror(
        Some(&window),
        &mut composition,
        Some(&field("Warma", (2, 2))),
    );

    // Then: the IME loses its focus, then has it again
    assert_eq!(window.0.borrow()[..2], ["allowed false", "allowed true"]);
}
