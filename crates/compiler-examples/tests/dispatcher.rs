//! A component's own event (`#[dispatcher]`, `src/picker.wisp`): a click
//! dispatches `pick` at the button; the component acts only when no
//! listener cancelled it, as a default action; a refused dispatch is the
//! click listener's `Err`, sent to the error report. Driven by the
//! recording Dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::picker::{Picker, PickerProps};
use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dom, Event, set_error_report};

fn mounted(on_pick: fn(&mut Event)) -> (Rc<RefCell<RecordingDom>>, Picker, Rc<RefCell<u32>>) {
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
            on_pick(e);
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
    let (dom, picker, heard) = mounted(|_| {});

    // When
    click(&dom);

    // Then: the root heard it, bubbled; the component acted
    assert_eq!(*heard.borrow(), 1);
    assert!(picker.picked.get());
}

#[test]
fn a_cancelled_pick_is_not_acted_on() {
    // Given
    let (dom, picker, heard) = mounted(Event::prevent_default);

    // When
    click(&dom);

    // Then
    assert_eq!(*heard.borrow(), 1);
    assert!(!picker.picked.get());
}

thread_local! {
    static REPORTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn reported(message: &str) {
    REPORTED.with(|seen| seen.borrow_mut().push(message.to_owned()));
}

#[test]
fn a_refused_pick_is_reported_and_not_acted_on() {
    // Given: a listener releases pointer capture, which `pick` cannot carry
    set_error_report(reported);
    let (dom, picker, heard) = mounted(Event::release_pointer_capture);

    // When
    click(&dom);

    // Then: the click listener's `?` sent the refusal to the report
    assert_eq!(*heard.borrow(), 1);
    assert!(!picker.picked.get());
    REPORTED.with(|seen| {
        let seen = seen.borrow();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            seen[0].contains("dispatched to 1 listeners, then refused"),
            "{seen:?}"
        );
    });
}
