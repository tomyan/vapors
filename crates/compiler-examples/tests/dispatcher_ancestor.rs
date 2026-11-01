//! A component type's events are known before it mounts: `src/pick_host.wisp`
//! listens for `pick` on a `div` whose `{if}` mounts a `Picker` later. The
//! elements document refuses a listener for an event nothing fires, so the
//! host's mount must declare the child's `pick` itself, from the type.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::pick_host::{PickHost, PickHostProps};
use vapors_compiler_examples::picker::Picker;
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

#[test]
fn an_ancestor_listener_is_taken_before_the_dispatching_component_mounts() {
    // Given: the host mounted with its Picker not yet shown
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let host = PickHost::mount(&doc, body, PickHostProps::default());

    // When
    let updated = doc.borrow_mut().update(VIEWPORT).map(|_| ());

    // Then: no refusal of the div's `onpick`
    assert_eq!(
        updated,
        Ok(()),
        "the host's mount declares its child's events"
    );
    assert!(!host.shown.get());
}

#[test]
fn a_type_lists_its_own_events_and_its_childrens() {
    assert_eq!(Picker::events(), ["pick"]);
    assert_eq!(PickHost::events(), ["pick"]);
}
