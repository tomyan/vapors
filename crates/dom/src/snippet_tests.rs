use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::recording::RecordingDom;
use vapors_signals::{Effect, Signal};

fn dom() -> (Rc<RefCell<RecordingDom>>, NodeId) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    (dom, root)
}

/// A snippet that places one `<p>` and records it among its roots.
fn paragraph() -> Snippet {
    Snippet::new(|dom, parent, before, _| {
        let p = dom.borrow_mut().create_element("p");
        dom.borrow_mut().insert_before(parent, p, before);
        vec![Root::Node(p)]
    })
}

#[test]
fn a_snippet_builds_its_nodes_where_it_is_rendered_and_returns_them() {
    // Given
    let (dom, root) = dom();

    // When
    let roots = paragraph().render(&dom, root, None, || ());

    // Then
    assert_eq!(dom.borrow().to_html(), "<p></p>");
    assert_eq!(roots.len(), 1);
}

#[test]
fn a_snippet_renders_into_a_dom_it_was_already_given_as_dyn() {
    // Given: inside another snippet, the Dom is already `dyn Dom`
    let (dom, root) = dom();
    let outer =
        Snippet::new(|dom, parent, before, _| paragraph().render(dom, parent, before, || ()));

    // When
    outer.render(&dom, root, None, || ());

    // Then
    assert_eq!(dom.borrow().to_html(), "<p></p>");
}

#[test]
fn the_empty_snippet_renders_nothing() {
    // Given
    let (dom, root) = dom();

    // When
    let roots = Snippet::<()>::default().render(&dom, root, None, || ());

    // Then
    assert_eq!(dom.borrow().to_html(), "");
    assert!(roots.is_empty());
}

/// A snippet of one text node, its content the argument, kept current
/// by an effect.
fn said() -> Snippet<(String,)> {
    Snippet::new(|dom, parent, before, args: Args<(String,)>| {
        let node = dom.borrow_mut().create_text("");
        dom.borrow_mut().insert_before(parent, node, before);
        let text = Rc::clone(dom);
        Effect::new(move || text.borrow_mut().set_text(node, &args().0));
        vec![Root::Node(node)]
    })
}

#[test]
fn a_snippet_builds_with_the_arguments_it_is_rendered_with() {
    // Given
    let (dom, root) = dom();

    // When
    said().render(&dom, root, None, || (String::from("a"),));
    said().render(&dom, root, None, || (String::from("b"),));

    // Then
    assert_eq!(dom.borrow().to_html(), "ab");
}

#[test]
fn a_snippet_reads_its_arguments_when_they_change() {
    // Given: rendered with an argument read from a signal
    let (dom, root) = dom();
    let word = Signal::new(String::from("before"));
    said().render(&dom, root, None, move || (word.get(),));

    // When
    word.set(String::from("after"));

    // Then: the same node, its text the new argument
    assert_eq!(dom.borrow().to_html(), "after");
}

#[test]
fn a_snippet_is_renderable_owned_borrowed_or_optional() {
    // Given
    let one = paragraph();

    // Then: the same snippet back, or none
    assert_eq!(Renderable::snippet(&one), Some(one.clone()));
    assert_eq!(Renderable::snippet(&&one), Some(one.clone()));
    assert_eq!(Renderable::snippet(&Some(&one)), Some(one.clone()));
    assert!(Renderable::<()>::snippet(&None::<Snippet>).is_none());
}

#[test]
fn two_snippets_built_alike_are_not_equal() {
    // Then: equality is identity
    assert_ne!(paragraph(), paragraph());
}
