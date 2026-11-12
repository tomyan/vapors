//! A component that uses itself (`src/branch.wisp`), or two that use
//! each other (`src/ping.wisp`, `src/pong.wisp`): each type's events are
//! listed once, and mounting them ends.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::branch::{Branch, BranchProps};
use vapors_compiler_examples::ping::{Ping, PingProps};
use vapors_compiler_examples::pong::Pong;
use vapors_dom::recording::{Call, RecordingDom};

fn divs(dom: &Rc<RefCell<RecordingDom>>) -> usize {
    dom.borrow()
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::CreateKeyedElement { key, .. } if key.tag == "div"))
        .count()
}

#[test]
fn a_component_that_uses_itself_lists_its_events_once() {
    assert_eq!(Branch::events(), ["fold"]);
}

#[test]
fn two_components_that_use_each_other_list_both_once() {
    assert_eq!(Ping::events(), ["ping", "pong"]);
    assert_eq!(Pong::events(), ["pong", "ping"]);
}

#[test]
fn a_component_that_uses_itself_mounts_to_its_depth() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When
    let _branch = Branch::mount(&dom, root, BranchProps::default());

    // Then: depths 0, 1 and 2
    assert_eq!(divs(&dom), 3);
}

#[test]
fn two_components_that_use_each_other_mount_to_their_depth() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When
    let _ping = Ping::mount(&dom, root, PingProps::default());

    // Then
    assert_eq!(divs(&dom), 3);
}
