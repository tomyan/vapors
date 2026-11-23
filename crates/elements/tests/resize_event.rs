//! A scroll container's `resize` event (virtual list V4,
//! `docs/plans/virtual-list.md`): `Document::settle_scroll` fires one at
//! each scroll container whose laid-out client size (its padding box)
//! differs from the one its last `resize` carried, carrying the size,
//! and none otherwise.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, element};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Viewport};

/// Each `resize` heard: who heard it (`list` or `body`) and its size.
type Heard = Vec<(&'static str, Option<[f32; 2]>)>;

/// A `.list` 10 px below the top of the window and reaching its bottom,
/// 100 px wide with a 2 px border, styled further by `list_css`, over
/// four 40 px rows; laid out once in a window `height` px tall, with
/// each `resize` its container and the body hear.
struct Page {
    doc: Rc<RefCell<Document>>,
    container: NodeId,
    heard: Rc<RefCell<Heard>>,
}

fn page(list_css: &str, height: f32) -> Page {
    let mut doc = document();
    let body = doc.body();
    let container = element(&mut doc, body, "div", "list");
    for _ in 0..4 {
        element(&mut doc, container, "div", "row");
    }
    let css = format!(
        "body {{ margin: 0; }} .row {{ height: 40px; }} \
         .list {{ position: absolute; top: 10px; bottom: 0; width: 100px; border: 2px solid #000; {list_css} }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    let heard = Rc::new(RefCell::new(Vec::new()));
    for (node, who) in [(container, "list"), (body, "body")] {
        let into = Rc::clone(&heard);
        doc.listen(
            node,
            "resize",
            Box::new(move |event: &mut Event| {
                into.borrow_mut().push((who, event.client_size()));
            }),
        );
    }
    let page = Page {
        doc: Rc::new(RefCell::new(doc)),
        container,
        heard,
    };
    page.update(height);
    page
}

impl Page {
    /// The `resize`s one settle fires.
    fn settle(&self) -> Heard {
        self.heard.borrow_mut().clear();
        Document::settle_scroll(&self.doc);
        self.heard.borrow().clone()
    }

    /// An update in a window 200 px wide and `height` px tall.
    fn update(&self, height: f32) {
        let viewport = Viewport {
            width: 200.0,
            height,
            scale: 1.0,
        };
        self.doc.borrow_mut().update(viewport).expect("updates");
    }
}

#[test]
fn the_first_settle_after_layout_fires_the_containers_client_size() {
    // Given: 200 px tall: the list's border box 104 by 190 px, its borders 2 px
    let page = page("overflow: auto;", 200.0);

    // When
    let heard = page.settle();

    // Then: at the container only; a `resize` does not bubble
    assert_eq!(heard, vec![("list", Some([100.0, 186.0]))]);
}

#[test]
fn a_size_already_reported_fires_nothing() {
    // Given
    let page = page("overflow: auto;", 200.0);
    page.settle();

    // When
    page.update(200.0);
    let heard = page.settle();

    // Then
    assert_eq!(heard, vec![]);
}

#[test]
fn a_taller_window_fires_the_new_size_at_the_next_settle() {
    // Given
    let page = page("overflow: auto;", 200.0);
    page.settle();

    // When
    page.update(300.0);
    let heard = page.settle();

    // Then
    assert_eq!(heard, vec![("list", Some([100.0, 286.0]))]);
}

#[test]
fn a_shorter_window_fires_the_new_size_at_the_next_settle() {
    // Given
    let page = page("overflow: auto;", 200.0);
    page.settle();

    // When
    page.update(120.0);
    let heard = page.settle();

    // Then
    assert_eq!(heard, vec![("list", Some([100.0, 106.0]))]);
}

#[test]
fn the_document_has_changed_while_a_resize_is_owed() {
    // Given: settled, its change flag taken
    let page = page("overflow: auto;", 200.0);
    page.settle();
    page.doc.borrow_mut().take_changed();

    // When
    page.update(300.0);
    let owed = page.doc.borrow_mut().take_changed();
    page.settle();
    let after = page.doc.borrow_mut().take_changed();

    // Then: the frame that fires it is asked for, and none after
    assert!(owed, "a resize owed asks for a frame");
    assert!(!after, "a resize fired asks for nothing more");
}

#[test]
fn a_scroll_alone_fires_no_resize() {
    // Given: 86 px tall inside, 74 px to scroll
    let page = page("overflow: auto;", 100.0);
    page.settle();

    // When
    let container = page.container;
    let moved = page.doc.borrow_mut().scroll_to(container, [0.0, 30.0]);
    page.update(100.0);
    let heard = page.settle();

    // Then
    assert!(moved, "the list scrolled");
    assert_eq!(heard, vec![]);
}

#[test]
fn an_element_that_does_not_scroll_fires_no_resize() {
    // Given: overflow visible
    let page = page("", 200.0);

    // When
    page.update(300.0);
    let heard = page.settle();

    // Then
    assert_eq!(heard, vec![]);
}

#[test]
fn a_scroll_container_no_one_listens_to_owes_no_resize() {
    // Given: as `page`, with no `resize` listener
    let mut doc = document();
    let body = doc.body();
    let container = element(&mut doc, body, "div", "list");
    let css = ".list { position: absolute; top: 10px; bottom: 0; width: 100px; overflow: auto; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let viewport = Viewport {
        width: 200.0,
        height: 200.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    doc.take_changed();

    // When
    doc.update(Viewport {
        height: 300.0,
        ..viewport
    })
    .expect("updates");

    // Then: nothing is owed, so nothing asks for a frame
    assert!(doc.scroll_offset(container).is_some(), "the list scrolls");
    assert!(!doc.take_changed());
}

#[test]
fn a_container_removed_before_the_settle_fires_nothing() {
    // Given: laid out, its `resize` owed
    let page = page("overflow: auto;", 200.0);

    // When
    let container = page.container;
    page.doc.borrow_mut().remove(container);
    let heard = page.settle();

    // Then
    assert_eq!(heard, vec![]);
}
