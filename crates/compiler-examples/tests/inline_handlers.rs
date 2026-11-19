//! An inline closure in `onclick={..}` with no parameter is called
//! without the event; one with a parameter is passed it. Each form is
//! mounted on the recording Dom and clicked.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::{hop_bare, hop_evented, hop_moved};
use vapors_dom::recording::RecordingDom;

macro_rules! clicked_once {
    ($module:ident, $component:ident, $props:ident) => {{
        let dom = Rc::new(RefCell::new(RecordingDom::new()));
        let root = dom.borrow().root();
        let mounted = $module::$component::mount(&dom, root, $module::$props::default());
        RecordingDom::click(&dom, mounted.roots()[0]);
        mounted.count.get()
    }};
}

#[test]
fn a_closure_with_no_parameters_is_called_without_the_event() {
    // When
    let count = clicked_once!(hop_bare, HopBare, HopBareProps);

    // Then
    assert_eq!(count, 1);
}

#[test]
fn a_move_closure_with_no_parameters_is_called_without_the_event() {
    // When
    let count = clicked_once!(hop_moved, HopMoved, HopMovedProps);

    // Then
    assert_eq!(count, 1);
}

#[test]
fn a_closure_with_one_parameter_still_takes_the_event() {
    // When
    let count = clicked_once!(hop_evented, HopEvented, HopEventedProps);

    // Then
    assert_eq!(count, 1);
}
