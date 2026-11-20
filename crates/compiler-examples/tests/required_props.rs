//! A required prop (`src/required_label.wisp`, `#[prop] let label:
//! String;`): a tag in the same crate passes it (`src/required_host.wisp`,
//! and the compiler refuses a tag that does not); a mount from Rust with
//! the props left at their default refuses at run time, naming the
//! component and the prop, and never fills a default.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::required_host::{RequiredHost, RequiredHostProps};
use vapors_compiler_examples::required_label::{RequiredLabel, RequiredLabelProps};
use vapors_dom::Required;
use vapors_dom::recording::RecordingDom;

fn dom() -> Rc<RefCell<RecordingDom>> {
    Rc::new(RefCell::new(RecordingDom::new()))
}

#[test]
fn a_tag_passes_the_required_prop() {
    // When
    let dom = dom();
    let root = dom.borrow().root();
    let _host = RequiredHost::mount(&dom, root, RequiredHostProps::default());

    // Then
    assert_eq!(dom.borrow().to_html(), "<b>given 3</b>");
}

#[test]
fn a_props_literal_passes_it_as_given() {
    // When
    let dom = dom();
    let root = dom.borrow().root();
    let props = RequiredLabelProps {
        label: Required::Given(String::from("by hand")),
        ..RequiredLabelProps::default()
    };
    let _label = RequiredLabel::mount(&dom, root, props);

    // Then
    assert_eq!(dom.borrow().to_html(), "<b>by hand 1</b>");
}

#[test]
#[should_panic(expected = "`RequiredLabel` was mounted without its required prop `label`")]
fn a_mount_without_it_is_refused_naming_component_and_prop() {
    // When: the default props, as a tag the compiler cannot check leaves them
    let dom = dom();
    let root = dom.borrow().root();
    let _label = RequiredLabel::mount(&dom, root, RequiredLabelProps::default());
}
