use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::recording::RecordingDom;

fn dom() -> (Rc<RefCell<RecordingDom>>, NodeId) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    (dom, root)
}

/// A snippet that places one `<p>` and records it among its roots.
fn paragraph() -> Snippet {
    Snippet::new(|dom, parent, before, ()| {
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
    let roots = paragraph().render(&dom, root, None, ());

    // Then
    assert_eq!(dom.borrow().to_html(), "<p></p>");
    assert_eq!(roots.len(), 1);
}

#[test]
fn a_snippet_renders_into_a_dom_it_was_already_given_as_dyn() {
    // Given: inside another snippet, the Dom is already `dyn Dom`
    let (dom, root) = dom();
    let outer = Snippet::new(|dom, parent, before, ()| paragraph().render(dom, parent, before, ()));

    // When
    outer.render(&dom, root, None, ());

    // Then
    assert_eq!(dom.borrow().to_html(), "<p></p>");
}

#[test]
fn the_empty_snippet_renders_nothing() {
    // Given
    let (dom, root) = dom();

    // When
    let roots = Snippet::<()>::default().render(&dom, root, None, ());

    // Then
    assert_eq!(dom.borrow().to_html(), "");
    assert!(roots.is_empty());
}

#[test]
fn a_snippet_builds_with_the_arguments_it_is_rendered_with() {
    // Given: a snippet of one text, its content an argument
    let (dom, root) = dom();
    let text: Snippet<(String,)> = Snippet::new(|dom, parent, before, (said,): (String,)| {
        let node = dom.borrow_mut().create_text(&said);
        dom.borrow_mut().insert_before(parent, node, before);
        vec![Root::Node(node)]
    });

    // When
    text.render(&dom, root, None, (String::from("a"),));
    text.render(&dom, root, None, (String::from("b"),));

    // Then
    assert_eq!(dom.borrow().to_html(), "ab");
}
