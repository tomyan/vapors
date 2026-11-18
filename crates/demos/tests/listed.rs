//! One list of the demos (`vapors_demos::DEMOS`): every elements demo
//! the crate mounts is in it once, in the same order, and nothing else
//! of that kind; the index shows every listed demo, apps included, as a
//! link to its address, in the list's order. No line opens with an
//! internal stage code.

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

/// Whether `line` opens with a stage code such as `F2c S1:` or `3-E3a:`:
/// every word before its first colon holds a digit.
fn opens_with_stage_code(line: &str) -> bool {
    line.split_once(':').is_some_and(|(before, _)| {
        before
            .split_whitespace()
            .all(|word| word.chars().any(|c| c.is_ascii_digit()))
    })
}

#[test]
fn no_listed_line_opens_with_a_stage_code() {
    // Given the public page shows each line in its bar and index
    assert!(
        opens_with_stage_code("F2c S1: a panel"),
        "the rule finds one"
    );
    assert!(!opens_with_stage_code("Top-down racing: the platform"));
    assert!(!opens_with_stage_code("a 3D scene: three cubes"));

    // Then
    let coded: Vec<&str> = DEMOS
        .iter()
        .map(|demo| demo.line)
        .filter(|line| opens_with_stage_code(line))
        .collect();
    assert!(
        coded.is_empty(),
        "lines open with a stage code {coded:?}; drop it in crates/demos/src/listed.rs"
    );
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

#[test]
fn the_index_shows_each_listed_demos_title_and_line_in_its_link() {
    // Given
    let index = vapors_demos::find(INDEX).expect("the index");
    let document = vapors_demos::document(&index).document;
    if let Err(errors) = document.borrow_mut().update(VIEWPORT) {
        panic!("the index was refused: {errors:#?}");
    }

    // When: each link's accessible name, from its text
    let tree = document.borrow().ax_tree().expect("an updated document");
    let names: Vec<String> = tree
        .iter()
        .filter(|node| node.role == "link")
        .map(|node| node.name.clone())
        .collect();

    // Then
    let listed: Vec<String> = DEMOS
        .iter()
        .map(|demo| format!("{} {}", demo.title, demo.line))
        .collect();
    assert_eq!(names, listed);
}

#[test]
fn an_unknown_name_is_refused_listing_every_listed_demo() {
    // When
    let Err(refused) = vapors_demos::find("nope") else {
        panic!("nope is no demo");
    };

    // Then
    let names: Vec<&str> = DEMOS.iter().map(|demo| demo.name).collect();
    assert_eq!(
        refused,
        format!(
            "no demo is called `nope`; the demos are {}",
            names.join(", ")
        )
    );
}
