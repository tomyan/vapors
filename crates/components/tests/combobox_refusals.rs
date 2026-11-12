//! The combobox when a dispatch is refused (an app listener asks for
//! pointer capture on a key's event): the refusal goes to the error
//! report, and the default action is not taken; it does not panic.
//! And two comboboxes on their default ids never share one.

mod support;

use std::cell::RefCell;

use support::mounted;
use vapors_dom::{Dom, Event, set_error_report};

thread_local! {
    static REPORTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn reported(message: &str) {
    REPORTED.with(|seen| seen.borrow_mut().push(message.to_owned()));
}

#[test]
fn a_refused_optionchoose_is_reported_and_chooses_nothing() {
    // Given: open on Alabama, and a listener that releases capture
    set_error_report("reported", reported);
    let combobox = mounted();
    combobox.key("ArrowDown");
    let root = combobox.dom.borrow().root();
    combobox.dom.borrow_mut().listen(
        root,
        "optionchoose",
        Box::new(|e: &mut Event| e.release_pointer_capture()),
    );

    // When
    combobox.key("Enter");

    // Then
    REPORTED.with(|seen| {
        let seen = seen.borrow();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(seen[0].contains("`optionchoose`"), "{seen:?}");
    });
    assert!(combobox.expanded.get());
    assert_eq!(combobox.value.get(), "");
}

#[test]
fn two_comboboxes_on_their_default_ids_do_not_share_one() {
    // Given
    use std::rc::Rc;
    use vapors_components::combobox::{Combobox, ComboboxProps};
    use vapors_dom::recording::RecordingDom;
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When
    let _first = Combobox::mount(&dom, root, ComboboxProps::default());
    let _second = Combobox::mount(&dom, root, ComboboxProps::default());

    // Then: each field's id, and the list each controls, its own
    let fields = dom.borrow().children(root);
    let ids: Vec<_> = fields
        .iter()
        .filter_map(|node| dom.borrow().attribute(*node, "id"))
        .collect();
    let controls: Vec<_> = fields
        .iter()
        .filter_map(|node| dom.borrow().attribute(*node, "aria-controls"))
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    assert_ne!(controls[0], controls[1]);
}
