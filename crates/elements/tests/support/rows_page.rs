//! The sparse-update workload of IU1: `rows` mounted rows of a label, a
//! value and a button, and the edits `sparse_updates.rs` makes to one
//! of them, each followed by one update. `crates/perf` times the same
//! workload, including this file and `basics.rs` by path.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

use super::basics::{document, element, text, viewport};

/// The value has a fixed size, so a same-length edit to it moves no
/// other box; the label is as wide as its text. The button changes
/// colour on hover and gains an outline on focus. A marked row has a
/// background.
fn css() -> String {
    format!(
        "body {{ margin: 0; }} \
    .row {{ display: flex; height: {ROW_HEIGHT}px; gap: 8px; }} \
    .row.marked {{ background-color: #eef; }} \
    .value {{ width: 80px; height: 20px; overflow: hidden; }} \
    button {{ width: 60px; height: 20px; background-color: #ddd; }} \
    button:hover {{ background-color: #bbb; }} \
    button:focus {{ outline: 2px solid #36c; }}"
    )
}

/// A row's height in pixels.
pub const ROW_HEIGHT: u32 = 24;

/// One row's nodes.
pub struct Row {
    pub element: NodeId,
    pub label: NodeId,
    pub value: NodeId,
    pub button: NodeId,
}

pub struct RowsPage {
    pub doc: Rc<RefCell<Document>>,
    pub rows: Vec<Row>,
}

/// The rows that intersect the viewport: the last one partly.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a positive height of a few hundred pixels
pub fn rows_in_view() -> usize {
    (rows_viewport().height as u32).div_ceil(ROW_HEIGHT) as usize
}

/// The viewport the workload updates at: 16 whole rows and part of the 17th.
pub fn rows_viewport() -> Viewport {
    viewport(640.0, 400.0)
}

/// `count` rows under the body, updated once.
pub fn rows_page(count: usize) -> RowsPage {
    let mut doc = document();
    let rows = (0..count).map(|index| row(&mut doc, index)).collect();
    doc.add_stylesheet("rows.css", &css()).expect("parses");
    doc.update(rows_viewport()).expect("updates");
    RowsPage {
        doc: Rc::new(RefCell::new(doc)),
        rows,
    }
}

/// A row appended under the body, numbered `index`.
pub fn row(doc: &mut Document, index: usize) -> Row {
    row_before(doc, index, None)
}

/// A row numbered `index`, inserted before `before` under the body, or
/// appended.
pub fn row_before(doc: &mut Document, index: usize, before: Option<NodeId>) -> Row {
    let body = doc.body();
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(body, row, before);
    let label = element(doc, row, "span", "label");
    let value = element(doc, row, "span", "value");
    let button = element(doc, row, "button", "");
    text(doc, button, "Edit");
    Row {
        element: row,
        label: text(doc, label, &format!("Row {index}")),
        value: text(doc, value, &format!("{:05}", index * 7)),
        button,
    }
}

/// One edit of the workload, named for reports and failures.
#[derive(Clone, Copy, Debug)]
pub enum Edit {
    /// The middle row's value, the same length: no box changes size.
    FixedSizeText,
    /// The middle row's label, longer: its box and its siblings move.
    SizeChangingText,
    /// The middle row's value set to the text it holds.
    EqualText,
    /// The pointer moves onto the second row's button.
    Hover,
    /// The second row's button takes focus.
    Focus,
    /// The middle row's `marked` class added, or removed when it has it.
    ToggleClass,
}

pub const EDITS: [Edit; 6] = [
    Edit::FixedSizeText,
    Edit::SizeChangingText,
    Edit::EqualText,
    Edit::Hover,
    Edit::Focus,
    Edit::ToggleClass,
];

impl RowsPage {
    /// Makes `edit`; the second row's button is at (`HOVER_X`, 34).
    pub fn edit(&self, edit: Edit) {
        let middle = &self.rows[self.rows.len() / 2];
        match edit {
            Edit::FixedSizeText => self.set_text(middle.value, "99999"),
            Edit::SizeChangingText => self.set_text(middle.label, "A much longer label"),
            Edit::EqualText => {
                let same = self.doc.borrow().text(middle.value).map(str::to_owned);
                self.set_text(middle.value, &same.expect("a text node"));
            }
            Edit::Hover => Document::pointer_moved(&self.doc, [HOVER_X, 34.0]),
            Edit::Focus => {
                let focused = Document::focus(&self.doc, Some(self.rows[1].button));
                assert_eq!(
                    focused,
                    vapors_elements::Focus::Done,
                    "the button takes focus"
                );
            }
            Edit::ToggleClass => self.toggle_marked(middle.element),
        }
    }

    fn toggle_marked(&self, row: NodeId) {
        self.toggle_class(row, "marked");
    }

    /// `class` added to `row`, or removed when it has it.
    fn toggle_class(&self, row: NodeId, class: &str) {
        let mut doc = self.doc.borrow_mut();
        let with = format!("row {class}");
        let has = doc.attribute(row, "class") == Some(with.as_str());
        doc.set_attribute(row, "class", if has { "row" } else { &with });
    }

    /// Adds the rule the `Resize` workload toggles, `.row.tall` at twice
    /// a row's height, and updates once.
    pub fn rows_resizable(&self) {
        let css = format!(".row.tall {{ height: {}px; }}", ROW_HEIGHT * 2);
        self.add_css("tall.css", &css);
    }

    /// The second row's `tall` class added, or removed when it has it:
    /// its boxes are built again, and every row after it moves.
    pub fn toggle_tall(&self) {
        self.toggle_class(self.rows[1].element, "tall");
    }

    /// Makes the body scroll its rows within the viewport's height (html
    /// hides its overflow, as vapors does not scroll the viewport), and
    /// updates once.
    pub fn body_scrolls(&self) {
        let height = rows_viewport().height;
        let css =
            format!("html {{ overflow: hidden; }} body {{ overflow: auto; height: {height}px; }}");
        self.add_css("scroll.css", &css);
    }

    /// Scrolls the body down by `rows` rows (up when negative); false
    /// when it cannot move that way.
    #[allow(clippy::cast_precision_loss)] // a row height of 24
    pub fn scroll_body(&self, rows: f32) -> bool {
        let mut doc = self.doc.borrow_mut();
        let body = doc.body();
        doc.scroll_node_by(body, [0.0, rows * ROW_HEIGHT as f32])
    }

    /// Scrolls the body back to its top.
    pub fn scroll_body_to_top(&self) {
        let mut doc = self.doc.borrow_mut();
        let body = doc.body();
        assert!(
            doc.scroll_to(body, [0.0, 0.0]),
            "the body scrolls to its top"
        );
    }

    fn add_css(&self, name: &str, css: &str) {
        let mut doc = self.doc.borrow_mut();
        doc.add_stylesheet(name, css).expect("parses");
        doc.update(rows_viewport()).expect("updates");
    }

    fn set_text(&self, node: NodeId, value: &str) {
        self.doc.borrow_mut().set_text(node, value);
    }
}

/// A point inside every row's button: the label of "Row 1" is under
/// 50 px wide, the value 80 px, the gaps 8 px each.
const HOVER_X: f32 = 170.0;
