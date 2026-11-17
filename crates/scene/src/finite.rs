//! Whether a scene holds only finite coordinates: the first item, or
//! item's clip, with an edge, origin, transform term or blur that is
//! not finite, and which kind it is. vapors-elements refuses a box whose
//! paint holds one (`paint_refusals.rs`), so nothing infinite is drawn.

use crate::display::{Item, Rect, Scene};

/// The first coordinate of a scene that is not finite: the item's
/// index and what holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonFinite {
    /// The item's index in [`Scene::items`].
    pub index: usize,
    /// What holds the coordinate: `"shadow"`, `"quad"`, `"image"`,
    /// `"icon"`, `"surface"`, `"shape"`, `"text"`, or `"clip"` for the
    /// clip it is drawn under.
    pub kind: &'static str,
}

impl Scene {
    /// The first item or clip with a coordinate that is not finite;
    /// `None` when every one is.
    #[must_use]
    pub fn non_finite(&self) -> Option<NonFinite> {
        let items = self.items().iter().zip(self.clips());
        items.enumerate().find_map(|(index, (item, clip))| {
            let kind = item_non_finite(item)
                .or_else(|| clip.filter(|clip| !rect_finite(*clip)).map(|_| "clip"))?;
            Some(NonFinite { index, kind })
        })
    }
}

/// `item`'s kind when one of its coordinates is not finite.
fn item_non_finite(item: &Item) -> Option<&'static str> {
    let (finite, kind) = match *item {
        Item::Shadow(shadow) => (
            rect_finite(shadow.rect) && rect_finite(shadow.caster) && shadow.blur.is_finite(),
            "shadow",
        ),
        Item::Quad(quad) => (rect_finite(quad.rect), "quad"),
        Item::Image { rect, .. } => (rect_finite(rect), "image"),
        Item::Icon { rect, .. } => (rect_finite(rect), "icon"),
        Item::Surface { rect, .. } => (rect_finite(rect), "surface"),
        Item::Shape(shape) => (
            shape.transform.iter().all(|term| term.is_finite()) && rect_finite(shape.bounds),
            "shape",
        ),
        Item::Text { origin, ink, .. } => (
            origin.iter().all(|at| at.is_finite()) && rect_finite(ink),
            "text",
        ),
    };
    (!finite).then_some(kind)
}

fn rect_finite(rect: Rect) -> bool {
    [rect.left, rect.top, rect.right, rect.bottom]
        .iter()
        .all(|edge| edge.is_finite())
}

#[cfg(test)]
#[path = "finite_tests.rs"]
mod tests;
