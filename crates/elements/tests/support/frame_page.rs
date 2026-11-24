//! Shared by the `Document::update_frame` tests: a scrolling list whose
//! `scroll`, `resize` and each update are logged, the `resize` doing what
//! the test asks, run one frame at a time.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Viewport};

use super::{document, element};

/// What one frame did, in order: `scroll`, `update` and `resize`.
pub type Log = Rc<RefCell<Vec<&'static str>>>;

/// What a `resize` handler does to the page.
#[derive(Clone, Copy)]
pub enum OnResize {
    /// Nothing.
    Nothing,
    /// Adds a 40 px `.added` row at the top of the list.
    AddsARow,
    /// Makes the list 10 px taller than its size: a size that never settles.
    Grows,
}

/// A `.list`, `overflow: auto`, 10 px below the top of the window and
/// reaching its bottom, 100 px wide, over four 40 px rows, styled further
/// by `css`; `scroll` and `resize` logged, the `resize` doing
/// `on_resize`, and the offset each `resize` carried kept.
pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    pub list: NodeId,
    pub log: Log,
    pub resize_offsets: Rc<RefCell<Vec<Option<[f32; 2]>>>>,
}

pub fn page(on_resize: OnResize, css: &str) -> Page {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..4 {
        element(&mut doc, list, "div", "row");
    }
    let sheet = format!(
        "body {{ margin: 0; }} .row, .added {{ height: 40px; }} \
         .list {{ position: absolute; top: 10px; bottom: 0; width: 100px; overflow: auto; }} {css}"
    );
    doc.add_stylesheet("app.css", &sheet).expect("parses");
    let page = Page {
        doc: Rc::new(RefCell::new(doc)),
        list,
        log: Rc::default(),
        resize_offsets: Rc::default(),
    };
    page.listen_scroll();
    page.listen_resize(on_resize);
    page
}

impl Page {
    fn listen_scroll(&self) {
        let log = Rc::clone(&self.log);
        let handler = move |_: &mut Event| log.borrow_mut().push("scroll");
        self.doc
            .borrow_mut()
            .listen(self.list, "scroll", Box::new(handler));
    }

    fn listen_resize(&self, on_resize: OnResize) {
        let (log, offsets) = (Rc::clone(&self.log), Rc::clone(&self.resize_offsets));
        let (weak, list) = (Rc::downgrade(&self.doc), self.list);
        let handler = move |event: &mut Event| {
            log.borrow_mut().push("resize");
            offsets.borrow_mut().push(event.scroll_offset());
            let doc = weak.upgrade().expect("the page is alive");
            let [_, height] = event.client_size().expect("a resize carries its size");
            resize_does(&mut doc.borrow_mut(), list, on_resize, height);
        };
        self.doc
            .borrow_mut()
            .listen(self.list, "resize", Box::new(handler));
    }

    /// One frame's Update phase in a window 200 px wide and `height` px
    /// tall, each update logged; what it did, in order.
    pub fn frame(&self, height: f32) -> Vec<&'static str> {
        self.try_frame(height).expect("the frame settles")
    }

    /// As [`Page::frame`], answering the frame's refusal.
    pub fn try_frame(&self, height: f32) -> Result<Vec<&'static str>, String> {
        self.log.borrow_mut().clear();
        let viewport = Viewport {
            width: 200.0,
            height,
            scale: 1.0,
        };
        let update = || {
            self.log.borrow_mut().push("update");
            let mut doc = self.doc.borrow_mut();
            let laid_out = doc.update(viewport);
            laid_out.map(|_| ()).map_err(|errors| format!("{errors:?}"))
        };
        Document::update_frame(&self.doc, update)?;
        Ok(self.log.borrow().clone())
    }

    /// The class of the element drawn at window (10, `y`).
    pub fn class_at(&self, y: f32) -> Option<String> {
        let doc = self.doc.borrow();
        let node = doc.painted().expect("updated").hit([10.0, y])?;
        doc.attribute(node, "class").map(str::to_owned)
    }

    /// How many `scroll`s the last frame fired.
    pub fn scrolls(&self) -> usize {
        self.log
            .borrow()
            .iter()
            .filter(|did| **did == "scroll")
            .count()
    }
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
