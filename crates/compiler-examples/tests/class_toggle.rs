//! `class:lit={on.get()}` (`src/class_toggle.wisp`) on the recording
//! Dom: the class is on while the signal is true, off when it flips
//! false, and on again when it flips back.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::class_toggle::{ClassToggle, ClassToggleProps};
use vapors_dom::NodeId;
use vapors_dom::recording::RecordingDom;
use vapors_signals::Signal;

fn mounted(on: bool) -> (Rc<RefCell<RecordingDom>>, NodeId, Signal<bool>) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let on = Signal::new(on);
    let toggle = ClassToggle::mount(&dom, root, ClassToggleProps { on });
    let div = toggle.roots()[0];
    (dom, div, on)
}

fn class(dom: &Rc<RefCell<RecordingDom>>, div: NodeId) -> Option<String> {
    dom.borrow().attribute(div, "class")
}

#[test]
fn the_directive_class_follows_the_signal_both_ways() {
    // Given
    let (dom, div, on) = mounted(false);
    assert_eq!(class(&dom, div).as_deref(), Some("card"));

    // When
    on.set(true);

    // Then
    assert_eq!(class(&dom, div).as_deref(), Some("card lit"));

    // When
    on.set(false);

    // Then
    assert_eq!(class(&dom, div).as_deref(), Some("card"));
}
