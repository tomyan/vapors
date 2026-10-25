//! A scrolling list for the scroll tests (F2c S9).

use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

use super::{document, element};

/// A 100 x 50 `.list` with `list` holding four 40 px rows (160 px of
/// content: 110 px to scroll).
pub struct List {
    pub doc: Document,
    pub container: NodeId,
    pub rows: Vec<NodeId>,
}

pub fn list(list_css: &str) -> List {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    let rows = (0..4)
        .map(|_| element(&mut doc, list, "div", "row"))
        .collect();
    let css = format!(
        "body {{ margin: 0; }} .list {{ width: 100px; height: 50px; {list_css} }} .row {{ height: 40px; }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    List {
        doc,
        container: list,
        rows,
    }
}

pub fn at(scale: f32) -> Viewport {
    Viewport {
        width: 200.0,
        height: 200.0,
        scale,
    }
}

pub fn top(doc: &mut Document, node: NodeId) -> f32 {
    doc.update(at(1.0))
        .expect("updates")
        .border_box(node)
        .expect("a box")
        .y
}

pub fn scroll(doc: &mut Document, delta: [f32; 2]) -> bool {
    doc.update(at(1.0)).expect("updates");
    doc.scroll_by([10.0, 10.0], delta)
}
