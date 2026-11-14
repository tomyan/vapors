//! One list of the demos (`vapors_demos::DEMOS`): every elements demo
//! the crate mounts is in it once, in the same order, and nothing else
//! of that kind; the index shows every listed demo, apps included, as a
//! link to its address, in the list's order.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_demos::{DEMOS, INDEX, Kind};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 1200.0,
    scale: 1.0,
};

#[test]
fn the_listed_elements_demos_are_the_mounted_ones_in_the_same_order() {
    // Given
    let mounted: Vec<&str> = vapors_demos::all()
        .iter()
        .map(|demo| demo.name)
        .filter(|name| *name != INDEX)
        .collect();

    // When
    let listed: Vec<&str> = DEMOS
        .iter()
        .filter(|demo| demo.kind == Kind::Elements)
        .map(|demo| demo.name)
        .collect();

    // Then
    assert_eq!(listed, mounted);
}

#[test]
fn every_listed_name_is_listed_once_with_a_title_and_a_line() {
    for (at, demo) in DEMOS.iter().enumerate() {
        let again = DEMOS[at + 1..].iter().any(|other| other.name == demo.name);
        assert!(!again, "{} is listed twice", demo.name);
        assert!(!demo.title.is_empty() && !demo.line.is_empty(), "{demo:?}");
    }
}

/// Each `a` element's `href` under `node`, in document order.
fn hrefs(document: &Document, node: NodeId, into: &mut Vec<String>) {
    if document.tag(node) == Some("a") {
        into.push(
            document
                .attribute(node, "href")
                .unwrap_or_default()
                .to_owned(),
        );
    }
    for child in document.children(node) {
        hrefs(document, child, into);
    }
}

#[test]
fn the_index_links_every_listed_demo_in_order() {
    // Given
    let index = vapors_demos::find(INDEX).expect("the index");
    let document: Rc<RefCell<Document>> = vapors_demos::document(&index).document;
    if let Err(errors) = document.borrow_mut().update(VIEWPORT) {
        panic!("the index was refused: {errors:#?}");
    }

    // When
    let mut linked = Vec::new();
    let body = document.borrow().body();
    hrefs(&document.borrow(), body, &mut linked);

    // Then
    let listed: Vec<String> = DEMOS.iter().map(vapors_demos::Listed::href).collect();
    assert_eq!(linked, listed);
}
