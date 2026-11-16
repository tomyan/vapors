//! A child component mounted by a parent's generated code keeps its
//! effects though its struct is dropped inside the parent's mount
//! (`owned_by_parent`, #353): `src/bound_host.wisp` passes its signal to
//! `src/bound_word.wisp`, and the child's text follows it after mount.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::bound_host::{BoundHost, BoundHostProps};
use vapors_dom::recording::RecordingDom;
use vapors_signals::Signal;

#[test]
fn a_child_s_text_follows_its_parent_s_signal_after_mount() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let word = Signal::new(String::from("before"));
    let _host = BoundHost::mount(&dom, root, BoundHostProps { word });

    // When
    word.set(String::from("after"));

    // Then
    assert_eq!(dom.borrow().to_html(), "<p><b>after</b></p>");
}
