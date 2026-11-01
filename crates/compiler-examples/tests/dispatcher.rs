//! A component's own event (`#[dispatcher]`, `src/picker.wisp`): a click
//! dispatches `pick` at the button; the component acts only when no
//! listener cancelled it, as a default action. Driven by the recording
//! Dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::picker::{Picker, PickerProps};
use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dom, Event};

fn mounted(cancel: bool) -> (Rc<RefCell<RecordingDom>>, Picker, Rc<RefCell<u32>>) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let picker = Picker::mount(&dom, root, PickerProps::default());
    let heard = Rc::new(RefCell::new(0));
    let into = Rc::clone(&heard);
    dom.borrow_mut().listen(
        root,
        "pick",
        Box::new(move |e: &mut Event| {
            *into.borrow_mut() += 1;
            if cancel {
                e.prevent_default();
            }
        }),
    );
    (dom, picker, heard)
}

fn click(dom: &Rc<RefCell<RecordingDom>>) {
    let root = dom.borrow().root();
    let button = dom.borrow().children(root)[0];
    RecordingDom::click(dom, button);
}

#[test]
fn a_pick_no_listener_cancels_is_acted_on() {
    // Given
    let (dom, picker, heard) = mounted(false);

    // When
    click(&dom);

    // Then: the root heard it, bubbled; the component acted
    assert_eq!(*heard.borrow(), 1);
    assert!(picker.picked.get());
    assert_eq!(picker.refused.get(), None);
}

#[test]
fn a_cancelled_pick_is_not_acted_on() {
    // Given
    let (dom, picker, heard) = mounted(true);

    // When
    click(&dom);

    // Then
    assert_eq!(*heard.borrow(), 1);
    assert!(!picker.picked.get());
}
