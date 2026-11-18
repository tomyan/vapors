//! `<vapors:window onkeydown>` (`src/key_pad.wisp`) on the recording Dom:
//! it hears a key while nothing is focused, and while the button is,
//! after the button's own listener; after unmount, nothing. A component
//! with nothing but the tag (`src/key_only.wisp`) mounts and unmounts.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::key_only::{KeyOnly, KeyOnlyProps};
use vapors_compiler_examples::key_pad::{KeyPad, KeyPadProps};
use vapors_dom::recording::RecordingDom;
use vapors_dom::{Event, NodeId};
use vapors_signals::Signal;

type Heard = Signal<Vec<String>>;

fn mounted() -> (Rc<RefCell<RecordingDom>>, NodeId, KeyPad, Heard) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let heard = Signal::new(Vec::new());
    let pad = KeyPad::mount(&dom, root, KeyPadProps { heard });
    (dom, root, pad, heard)
}

fn key(dom: &Rc<RefCell<RecordingDom>>, at: NodeId, key: &str) {
    RecordingDom::dispatch(dom, at, Event::new("keydown").with_key(key));
}

#[test]
fn a_key_with_nothing_focused_reaches_the_window_listener() {
    // Given: the page's key event lands on the root, as with no focus
    let (dom, root, pad, heard) = mounted();

    // When
    key(&dom, root, "ArrowRight");
    key(&dom, root, "ArrowRight");
    key(&dom, root, "ArrowLeft");

    // Then
    assert_eq!(pad.x.get(), 1);
    assert_eq!(heard.get().len(), 3);
}

#[test]
fn a_focused_buttons_listener_runs_before_the_window_listener() {
    // Given
    let (dom, _, pad, heard) = mounted();
    let button = pad.roots()[0];

    // When
    key(&dom, button, "ArrowRight");

    // Then
    assert_eq!(heard.get(), ["button ArrowRight", "window ArrowRight"]);
    assert_eq!(pad.x.get(), 1);
}

#[test]
fn after_unmount_a_key_reaches_nothing() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let heard = Signal::new(Vec::new());
    let pad = KeyPad::mount(&dom, root, KeyPadProps { heard });
    pad.unmount(&dom);

    // When
    key(&dom, root, "ArrowRight");

    // Then
    assert_eq!(heard.get(), Vec::<String>::new());
}

#[test]
fn a_component_of_only_a_window_tag_listens_and_unmounts() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let presses = Signal::new(0);
    let only = KeyOnly::mount(&dom, root, KeyOnlyProps { presses });

    // When
    RecordingDom::dispatch(&dom, root, Event::new("keyup").with_key("a"));
    let heard = presses.get();
    only.unmount(&dom);
    RecordingDom::dispatch(&dom, root, Event::new("keyup").with_key("a"));

    // Then
    assert_eq!((heard, presses.get()), (1, 1));
}
