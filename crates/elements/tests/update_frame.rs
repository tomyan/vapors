//! `Document::update_frame` (virtual list V4, `docs/plans/virtual-list.md`):
//! a frame's Update phase fires the `resize` its first update owes
//! before the second, so what a `resize` handler changes is laid out,
//! and drawn, in the frame whose layout changed the size. At most two
//! updates run in a frame.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, element};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Viewport};

/// What one frame did, in order: `scroll`, `update` and `resize`.
type Log = Rc<RefCell<Vec<&'static str>>>;

/// What a `resize` handler does to the page.
#[derive(Clone, Copy)]
enum OnResize {
    /// Nothing.
    Nothing,
    /// Adds a 40 px `.added` row at the top of the list.
    AddsARow,
    /// Makes the list 10 px taller than its size: a size that never settles.
    Grows,
}

/// A `.list`, `overflow: auto`, 10 px below the top of the window and
/// reaching its bottom, 100 px wide, over four 40 px rows; `scroll` and
/// `resize` logged, the `resize` doing `on_resize`.
struct Page {
    doc: Rc<RefCell<Document>>,
    list: NodeId,
    log: Log,
}

fn page(on_resize: OnResize) -> Page {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..4 {
        element(&mut doc, list, "div", "row");
    }
    let css = "body { margin: 0; } .row, .added { height: 40px; } \
         .list { position: absolute; top: 10px; bottom: 0; width: 100px; overflow: auto; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let doc = Rc::new(RefCell::new(doc));
    let log: Log = Rc::default();
    listen_scroll(&doc, list, &log);
    listen_resize(&doc, list, &log, on_resize);
    Page { doc, list, log }
}

fn listen_scroll(doc: &Rc<RefCell<Document>>, list: NodeId, log: &Log) {
    let log = Rc::clone(log);
    let handler = move |_: &mut Event| log.borrow_mut().push("scroll");
    doc.borrow_mut().listen(list, "scroll", Box::new(handler));
}

fn listen_resize(doc: &Rc<RefCell<Document>>, list: NodeId, log: &Log, on_resize: OnResize) {
    let log = Rc::clone(log);
    let weak = Rc::downgrade(doc);
    let handler = move |event: &mut Event| {
        log.borrow_mut().push("resize");
        let doc = weak.upgrade().expect("the page is alive");
        let [_, height] = event.client_size().expect("a resize carries its size");
        resize_does(&mut doc.borrow_mut(), list, on_resize, height);
    };
    doc.borrow_mut().listen(list, "resize", Box::new(handler));
}

fn resize_does(doc: &mut Document, list: NodeId, on_resize: OnResize, height: f32) {
    match on_resize {
        OnResize::Nothing => {}
        OnResize::AddsARow => {
            let row = doc.create_element("div");
            doc.set_attribute(row, "class", "added");
            let first = doc.children(list)[0];
            doc.insert_before(list, row, Some(first));
        }
        OnResize::Grows => {
            let taller = format!("height: {}px;", height + 10.0);
            doc.set_attribute(list, "style", &taller);
        }
    }
}

impl Page {
    /// One frame's Update phase in a window 200 px wide and `height` px
    /// tall, each update logged; what it did, in order.
    fn frame(&self, height: f32) -> Vec<&'static str> {
        self.log.borrow_mut().clear();
        let viewport = Viewport {
            width: 200.0,
            height,
            scale: 1.0,
        };
        let update = || {
            self.log.borrow_mut().push("update");
            let mut doc = self.doc.borrow_mut();
            doc.update(viewport)
                .map(|_| ())
                .map_err(|errors| format!("{errors:?}"))
        };
        Document::update_frame(&self.doc, update).expect("the frame settles");
        self.log.borrow().clone()
    }

    /// The class of the element drawn at window (10, `y`).
    fn class_at(&self, y: f32) -> Option<String> {
        let doc = self.doc.borrow();
        let node = doc.painted().expect("updated").hit([10.0, y])?;
        doc.attribute(node, "class").map(str::to_owned)
    }
}

#[test]
fn the_first_frame_fires_the_resize_after_its_update() {
    // Given
    let page = page(OnResize::Nothing);

    // When
    let did = page.frame(200.0);

    // Then: the handler changed nothing, so one update
    assert_eq!(did, ["update", "resize"]);
}

#[test]
fn what_a_resize_handler_changes_is_drawn_in_the_frame_of_the_resize() {
    // Given
    let page = page(OnResize::AddsARow);

    // When
    let did = page.frame(200.0);

    // Then: the second update laid the added row out at the list's top
    assert_eq!(did, ["update", "resize", "update"]);
    assert_eq!(page.class_at(15.0).as_deref(), Some("added"));
}

#[test]
fn a_window_resize_is_drawn_in_its_own_frame() {
    // Given: settled at 200 px, one row added
    let page = page(OnResize::AddsARow);
    page.frame(200.0);

    // When
    let did = page.frame(300.0);

    // Then: a second row added and drawn in the same frame
    assert_eq!(did, ["update", "resize", "update"]);
    assert_eq!(page.class_at(55.0).as_deref(), Some("added"));
}

#[test]
fn a_scroll_fires_before_the_update_and_the_resize_after_it() {
    // Given: settled at 100 px, 160 px of rows in 90, one added
    let page = page(OnResize::AddsARow);
    page.frame(100.0);
    let list = page.list;
    assert!(page.doc.borrow_mut().scroll_to(list, [0.0, 30.0]));

    // When: scrolled, and the window 10 px taller, which keeps 30 px in range
    let did = page.frame(110.0);

    // Then
    assert_eq!(did, ["scroll", "update", "resize", "update"]);
}

#[test]
fn an_offset_the_new_layout_clamps_fires_its_scroll_before_the_resize() {
    // Given: at the end of the 90 px list, 70 px down
    let page = page(OnResize::Nothing);
    page.frame(100.0);
    let list = page.list;
    assert!(page.doc.borrow_mut().scroll_to(list, [0.0, 70.0]));
    page.frame(100.0);

    // When: 150 px tall, a 140 px list, 20 px to scroll
    let did = page.frame(150.0);

    // Then: the clamp's scroll, then the resize, in the frame of the
    // update that clamped it; their handlers changed nothing, so no second
    assert_eq!(did, ["update", "scroll", "resize"]);
    assert_eq!(page.doc.borrow().scroll_offset(list), Some([0.0, 20.0]));
}

#[test]
fn a_settled_frame_updates_once_and_asks_for_no_other() {
    // Given
    let page = page(OnResize::AddsARow);
    page.frame(200.0);
    page.doc.borrow_mut().take_changed();

    // When
    let did = page.frame(200.0);

    // Then
    assert_eq!(did, ["update"]);
    assert!(!page.doc.borrow_mut().take_changed());
}

#[test]
fn a_size_that_never_settles_runs_two_updates_a_frame_and_owes_the_next_resize() {
    // Given: a handler that makes its list taller at each resize
    let page = page(OnResize::Grows);

    // When
    let did = page.frame(200.0);

    // Then: the frame ends after its second update; the next frame owes a resize
    assert_eq!(did, ["update", "resize", "update"]);
    assert!(page.doc.borrow_mut().take_changed());
}

#[test]
fn the_frame_after_one_that_did_not_settle_still_runs_two_updates() {
    // Given
    let page = page(OnResize::Grows);
    page.frame(200.0);

    // When
    let did = page.frame(200.0);

    // Then: the resize owed fires at the frame's start, the next after its first update
    assert_eq!(did, ["resize", "update", "resize", "update"]);
}
