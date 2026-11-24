//! The `list` scenario (`LIST_ROWS_TOP` in the spec): the orders header
//! over vapors' `VirtualList` of `count` order rows, keyed by order.
use crate::dashboard::{TABLE_HEADER_HEIGHT, WINDOW, list_order};
use crate::list_app::{ListApp, ListAppProps};
use std::{cell::RefCell, rc::Rc};
use vapors_elements::Document;
use vapors_text::FontSystem;

/// The list's window: the window below the header.
pub const LIST_HEIGHT: f32 = WINDOW.1 - TABLE_HEADER_HEIGHT;

/// The list's document, mounted.
pub struct MountedList {
    pub document: Rc<RefCell<Document>>,
    _root: ListApp,
}

/// Mounts the list of `count` orders into a fresh document.
pub fn mount(count: usize) -> MountedList {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    let body = document.borrow().body();
    let root = ListApp::mount(&document, body, ListAppProps { count });
    MountedList {
        document,
        _root: root,
    }
}

/// The rows' keys in the order shown: each row's order index, its stable
/// identity, in `list_order`.
pub fn keys(count: usize, sorted: bool) -> Vec<usize> {
    let order = list_order(count, sorted);
    order.into_iter().map(|i| i as usize).collect()
}
