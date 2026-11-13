//! The moved subtree: units inside a container offset by a fractional
//! amount, and that offset animated over frames. vapors refuses CSS
//! `transform` and the `animation-*` properties (deferred in
//! `style/honoured.rs`), so the translate is `position: relative` with
//! fractional `left` and `top`, and the animation is the container's
//! class moved to the next of eight offsets each frame. Scale and
//! rotate have no equivalent and are not run.

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

use crate::off_path_page::{Shape, Unit, UnitPage, child_before, value_text};
use crate::rows::basics::{element, text};

/// Rows of a label and a value inside the moved container.
pub const TRANSLATED: Shape = Shape {
    css: ".moved { position: relative; left: 0.5px; top: 0.25px; } \
          .moved.at1 { left: 0.375px; top: 0.125px; } \
          .moved.at2 { left: 0.75px; top: 0.25px; } \
          .moved.at3 { left: 1.125px; top: 0.375px; } \
          .moved.at4 { left: 1.5px; top: 0.5px; } \
          .moved.at5 { left: 1.875px; top: 0.625px; } \
          .moved.at6 { left: 2.25px; top: 0.75px; } \
          .moved.at7 { left: 2.625px; top: 0.875px; } \
          .item { display: flex; height: 24px; gap: 8px; } \
          .label { width: 120px; }",
    container: moved,
    unit: item,
};

/// The offsets the animation steps through: `.moved.at1` to `.at7`,
/// then plain `.moved` again.
pub const STEPS: usize = 8;

/// Moves the container to the offset of frame `sample`.
pub fn animate(page: &UnitPage, sample: usize) {
    let class = match sample % STEPS {
        0 => "moved".to_owned(),
        step => format!("moved at{step}"),
    };
    page.doc
        .borrow_mut()
        .set_attribute(page.container, "class", &class);
}

fn moved(doc: &mut Document) -> NodeId {
    let body = doc.body();
    element(doc, body, "div", "moved")
}

/// Item `index`: its label, then its value.
fn item(doc: &mut Document, parent: NodeId, index: usize, before: Option<NodeId>) -> Unit {
    let item = child_before(doc, parent, "div", "item", before);
    let label = element(doc, item, "span", "label");
    text(doc, label, &format!("Item {index}"));
    let span = element(doc, item, "span", "");
    let value = text(doc, span, &value_text(index));
    Unit {
        element: item,
        value,
        inner: None,
    }
}

#[cfg(test)]
#[path = "off_path_moved_tests.rs"]
mod tests;
