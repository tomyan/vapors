// A top-level component's mount whose result is dropped at once: its
// effects would be disposed with it, so `#[must_use]` refuses it.
#![deny(unused_must_use)]

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::class_toggle::{ClassToggle, ClassToggleProps};
use vapors_dom::recording::RecordingDom;
use vapors_signals::Signal;

fn main() {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    ClassToggle::mount(&dom, root, ClassToggleProps { on: Signal::new(false) });
}
