//! The `{render expr()}` block runtime (1-R1, R1d): the snippet the
//! callee yields is built before the anchor, and rebuilt only when the
//! callee yields a different snippet (compared by identity); `None`
//! builds nothing. The arguments stay reactive inside the snippet.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::blocks::dynamic;
use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Args, Dom, NodeId, Root, Snippet};
use vapors_signals::{Effect, Owner, Signal};

type Shared = Rc<RefCell<RecordingDom>>;

fn setup() -> (Shared, NodeId, NodeId) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let anchor = dom.borrow_mut().create_text("");
    dom.borrow_mut().append(root, anchor);
    (dom, root, anchor)
}

/// `<tag>{argument}</tag>`, its text following the argument.
fn tagged(tag: &'static str) -> Snippet<(String,)> {
    Snippet::new(move |dom, parent, before, args: Args<(String,)>| {
        let element = dom.borrow_mut().create_element(tag);
        dom.borrow_mut().insert_before(parent, element, before);
        let text = dom.borrow_mut().create_text("");
        dom.borrow_mut().append(element, text);
        let writer = Rc::clone(dom);
        Effect::new(move || writer.borrow_mut().set_text(text, &args().0));
        vec![Root::Node(element)]
    })
}

#[test]
fn builds_the_snippet_the_callee_yields_and_swaps_when_it_changes() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let (bold, italic) = (tagged("b"), tagged("i"));
    let current = Signal::new(Some(bold.clone()));
    dynamic(
        &dom,
        root,
        anchor,
        move || current.get(),
        || (String::from("x"),),
    );
    assert_eq!(dom.borrow().to_html(), "<b>x</b>");

    // When
    current.set(Some(italic));

    // Then
    assert_eq!(dom.borrow().to_html(), "<i>x</i>");
}

#[test]
fn none_builds_nothing_and_some_builds_again() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let current: Signal<Option<Snippet<(String,)>>> = Signal::new(None);
    dynamic(
        &dom,
        root,
        anchor,
        move || current.get(),
        || (String::from("x"),),
    );
    assert_eq!(dom.borrow().to_html(), "");

    // When
    current.set(Some(tagged("b")));

    // Then
    assert_eq!(dom.borrow().to_html(), "<b>x</b>");
}

#[test]
fn the_same_snippet_again_causes_no_remount() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let bold = tagged("b");
    let current = Signal::new(Some(bold.clone()));
    dynamic(
        &dom,
        root,
        anchor,
        move || current.get(),
        || (String::from("x"),),
    );
    dom.borrow_mut().clear_calls();

    // When: a clone of the same snippet, set again
    current.set(Some(bold.clone()));

    // Then
    assert_eq!(dom.borrow().calls(), &[] as &[Call]);
}

#[test]
fn the_arguments_stay_reactive_inside_the_snippet() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let word = Signal::new(String::from("one"));
    let bold = tagged("b");
    dynamic(
        &dom,
        root,
        anchor,
        move || Some(bold.clone()),
        move || (word.get(),),
    );

    // When
    word.set(String::from("two"));

    // Then
    assert_eq!(dom.borrow().to_html(), "<b>two</b>");
}

#[test]
fn a_swapped_out_snippets_effects_stop() {
    // Given
    let owner = Owner::new();
    owner.set();
    let (dom, root, anchor) = setup();
    let word = Signal::new(String::from("one"));
    let current = Signal::new(Some(tagged("b")));
    dynamic(
        &dom,
        root,
        anchor,
        move || current.get(),
        move || (word.get(),),
    );
    current.set(None);
    dom.borrow_mut().clear_calls();

    // When
    word.set(String::from("two"));

    // Then: nothing is written to the removed node
    assert_eq!(dom.borrow().calls(), &[] as &[Call]);
}
