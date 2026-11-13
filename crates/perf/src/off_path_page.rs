//! The page an off-path workload changes (`off_path.rs`): `count` units
//! in a container under a body that scrolls them, each unit holding a
//! five-digit value. A [`Shape`] says what a unit is and how it is
//! styled; the changes are the same for every shape.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

use crate::rows::basics;

/// The viewport every off-path page updates at, as the rows page's.
#[must_use]
pub fn page_viewport() -> Viewport {
    basics::viewport(640.0, 400.0)
}

/// What the body is given on every page: it scrolls within the
/// viewport's height (html hides its overflow, as vapors does not
/// scroll the viewport).
const BODY_CSS: &str =
    "html { overflow: hidden; } body { margin: 0; overflow: auto; height: 400px; } ";

/// One unit's nodes.
#[derive(Clone, Copy, Debug)]
pub struct Unit {
    /// The unit's outermost element, a child of the container.
    pub element: NodeId,
    /// The text node holding the unit's five-digit value.
    pub value: NodeId,
    /// The unit's own scroll container, for the shapes that have one.
    pub inner: Option<NodeId>,
}

/// What a page's units are.
pub struct Shape {
    /// The units' styles; the body's are [`BODY_CSS`].
    pub css: &'static str,
    /// Makes the units' container under the body.
    pub container: fn(&mut Document) -> NodeId,
    /// Makes unit `index` under the container, before a sibling or last.
    pub unit: fn(&mut Document, NodeId, usize, Option<NodeId>) -> Unit,
}

/// The value text of unit `index`: five digits, so every edit is the
/// same length.
#[must_use]
pub fn value_text(index: usize) -> String {
    format!("{:05}", index * 7 % 100_000)
}

/// A page of units, updated once.
pub struct UnitPage {
    /// The page's document.
    pub doc: Rc<RefCell<Document>>,
    /// The units' parent.
    pub container: NodeId,
    /// The units made with the page, in order.
    pub units: Vec<Unit>,
    unit: fn(&mut Document, NodeId, usize, Option<NodeId>) -> Unit,
    /// The unit `toggle_inserted` added and has not yet removed.
    inserted: Option<NodeId>,
}

impl UnitPage {
    /// `count` units of `shape`, updated once.
    ///
    /// # Errors
    /// The update refused the page: each refusal, which names the
    /// element and the property.
    pub fn new(shape: &Shape, count: usize) -> Result<Self, String> {
        let mut doc = basics::document();
        let container = (shape.container)(&mut doc);
        let units = (0..count)
            .map(|index| (shape.unit)(&mut doc, container, index, None))
            .collect();
        let css = format!("{BODY_CSS}{}", shape.css);
        doc.add_stylesheet("off_path.css", &css)
            .map_err(|error| format!("the page's CSS: {error}"))?;
        doc.update(page_viewport())
            .map_err(|errors| distinct(&errors))?;
        Ok(Self {
            doc: Rc::new(RefCell::new(doc)),
            container,
            units,
            unit: shape.unit,
            inserted: None,
        })
    }
}

/// Each distinct refusal once: every unit repeats its shape's.
fn distinct(errors: &[impl ToString]) -> String {
    let mut errors: Vec<_> = errors.iter().map(ToString::to_string).collect();
    errors.sort();
    errors.dedup();
    errors.join("; ")
}

impl UnitPage {
    /// Sets the middle unit's value to a new five-digit text.
    pub fn edit(&self, sample: usize) {
        let value = self.units[self.units.len() / 2].value;
        let text = format!("{:05}", 90_000 + sample % 10_000);
        self.doc.borrow_mut().set_text(value, &text);
    }

    /// Scrolls the body down by `step` pixels, or back to the top when
    /// it cannot move further.
    pub fn scroll_body(&self, step: f32) {
        let body = self.doc.borrow().body();
        self.scroll_or_rewind(body, [0.0, step]);
    }

    /// Scrolls the second unit's own scroller right by `step` pixels,
    /// or back to its start: the second unit is in view on every page.
    ///
    /// # Panics
    /// The shape's units have no scroller of their own: the registry
    /// (`off_path_kinds.rs`) offers this change only for shapes that do.
    pub fn scroll_inner(&self, step: f32) {
        let inner = self.units[1].inner;
        let inner = inner.expect("ScrollInner is listed only for shapes whose units scroll");
        self.scroll_or_rewind(inner, [step, 0.0]);
    }

    fn scroll_or_rewind(&self, node: NodeId, delta: [f32; 2]) {
        let mut doc = self.doc.borrow_mut();
        if doc.scroll_node_by(node, delta) {
            return;
        }
        assert!(
            doc.scroll_to(node, [0.0, 0.0]),
            "the scroller {node:?} neither moves by {delta:?} nor back to its start: \
             it is not a scroll container (cargo test -p vapors-perf --lib off_path)"
        );
    }

    /// Removes the inserted unit, or inserts one before the middle unit.
    pub fn toggle_inserted(&mut self) {
        let mut doc = self.doc.borrow_mut();
        if let Some(inserted) = self.inserted.take() {
            doc.remove(inserted);
            return;
        }
        let middle = self.units[self.units.len() / 2].element;
        let added = (self.unit)(&mut doc, self.container, self.units.len(), Some(middle));
        self.inserted = Some(added.element);
    }
}

#[cfg(test)]
#[path = "off_path_page_tests.rs"]
mod tests;
