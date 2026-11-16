//! A list of rows under body, for the tests of inserting and removing
//! a row (`narrow_rebuild.rs`, `child_edits.rs`).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, UpdateCounts, Viewport};

use super::{document, element, text, viewport};

/// Rows of flex cells, each 20px tall.
pub const ROWS: &str = ".row { display: flex; height: 20px; } .cell { width: 40px; }";

/// The viewport the list updates at.
pub const VIEW: fn() -> Viewport = || viewport(400.0, 300.0);

/// A list element under body and its rows, in a document.
pub struct List {
    pub doc: Rc<RefCell<Document>>,
    pub parent: NodeId,
    pub rows: Vec<NodeId>,
}

/// A row: `div.row > div.cell > text`, three boxes (row, cell, run).
pub fn row(doc: &mut Document, label: &str) -> NodeId {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    let cell = element(doc, row, "div", "cell");
    text(doc, cell, label);
    row
}

/// `count` rows in a list under body, styled by `css`, updated once.
pub fn list(count: usize, css: &str) -> List {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    let rows = (0..count)
        .map(|index| {
            let node = row(&mut doc, &format!("row {index}"));
            doc.append(list, node);
            node
        })
        .collect();
    doc.add_stylesheet("rows.css", css).expect("parses");
    doc.update(VIEW()).expect("updates");
    List {
        doc: Rc::new(RefCell::new(doc)),
        parent: list,
        rows,
    }
}

impl List {
    /// Updates and returns the update's counts.
    pub fn update(&self) -> UpdateCounts {
        let mut doc = self.doc.borrow_mut();
        doc.update(VIEW()).expect("updates");
        doc.update_counts()
    }

    /// Updates and returns how many boxes the update built.
    pub fn built(&self) -> usize {
        self.update().boxes_built
    }

    /// Inserts a new row before row `at`; the new row.
    pub fn insert_at(&self, at: usize) -> NodeId {
        let mut doc = self.doc.borrow_mut();
        let node = row(&mut doc, "new");
        doc.insert_before(self.parent, node, Some(self.rows[at]));
        node
    }
}
