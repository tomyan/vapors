//! A flex-column list (M10c): `count` rows under a body laid out as a
//! flex column, each a block holding its text directly. An edit to a
//! row's text builds the row again: a flex item, which its parent
//! asks for its size before laying it out, so the replay of that size
//! query decides whether the column is laid out again.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

use super::basics::{document, element, text, viewport};

/// A row's height in pixels.
pub const ROW_HEIGHT: u32 = 24;

/// A vertical margin for the rows (`flex_list`'s `extra`): a block
/// answers a size query with its margins, and Taffy's cache keeps no
/// such answer, so the replay compares with what the row recorded.
pub const ROW_MARGIN: &str = "margin: 4px 0;";

/// One row's nodes: the element and its text.
pub struct ListRow {
    pub element: NodeId,
    pub text: NodeId,
}

pub struct FlexList {
    pub doc: Rc<RefCell<Document>>,
    pub rows: Vec<ListRow>,
}

/// The viewport the list updates at: 16 whole rows and part of the 17th.
pub fn list_viewport() -> Viewport {
    viewport(640.0, 400.0)
}

/// `count` rows, each `extra` styled besides its height, updated once.
pub fn flex_list(count: usize, extra: &str) -> FlexList {
    let mut doc = document();
    let rows = (0..count).map(|index| list_row(&mut doc, index)).collect();
    let css = format!(
        "body {{ margin: 0; display: flex; flex-direction: column; }} \
         .row {{ display: block; height: {ROW_HEIGHT}px; {extra} }}"
    );
    doc.add_stylesheet("list.css", &css).expect("parses");
    doc.update(list_viewport()).expect("updates");
    FlexList {
        doc: Rc::new(RefCell::new(doc)),
        rows,
    }
}

/// A row appended under the body, numbered `index`, with its value.
fn list_row(doc: &mut Document, index: usize) -> ListRow {
    let body = doc.body();
    let element = element(doc, body, "div", "row");
    let text = text(doc, element, &row_text(index, index * 7));
    ListRow { element, text }
}

/// Row `index`'s text showing `value`: the same length for any
/// five-digit value, so a value edit changes no size.
pub fn row_text(index: usize, value: usize) -> String {
    format!("Row {index}: {value:05}")
}

impl FlexList {
    /// Sets the middle row's value: a same-length text, so no box
    /// changes size.
    pub fn set_middle_value(&self, value: usize) {
        self.set_middle_text(&row_text(self.rows.len() / 2, value));
    }

    /// Sets the middle row's text.
    pub fn set_middle_text(&self, text: &str) {
        let middle = self.rows.len() / 2;
        self.doc.borrow_mut().set_text(self.rows[middle].text, text);
    }
}
