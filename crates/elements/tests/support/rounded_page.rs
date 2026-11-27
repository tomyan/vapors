//! A rounded card page for the rounded clip tests (`rounded_clip*.rs`).

use super::{document, element, viewport};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};
use vapors_scene::{Clip, Color, Item};

/// A 100 x 60 card, a 1px border and 12px corners, clipping; `.fill`
/// is a tall blue block.
pub const CARD: &str = "body { margin: 0; } \
    .card { overflow: hidden; width: 100px; height: 60px; border: 1px solid black; border-radius: 12px; } \
    .fill { height: 200px; background-color: blue; }";

pub const BLUE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

pub fn at(scale: f32) -> Viewport {
    Viewport {
        scale,
        ..viewport(400.0, 300.0)
    }
}

/// A card holding `inner` (a class, or empty for none) holding a fill.
pub fn card(css: &str, inner: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let card = element(&mut doc, body, "div", "card");
    let parent = if inner.is_empty() {
        card
    } else {
        element(&mut doc, card, "div", inner)
    };
    element(&mut doc, parent, "div", "fill");
    doc.add_stylesheet("page.css", &format!("{CARD} {css}"))
        .expect("parses");
    (doc, card)
}

/// The clip of the blue fill's quad.
pub fn fill_clip(doc: &Document) -> Option<Clip> {
    let painted = doc.painted().expect("painted");
    let mut fills = painted
        .scene()
        .items()
        .filter_map(|(item, clip)| match item {
            Item::Quad(quad) if quad.background == BLUE => Some(clip),
            _ => None,
        });
    fills.next().expect("the fill is painted")
}

/// A rounded card holding a list of 40 rows, a lane taller than the card,
/// so its content clip is the card's rounded one; `css` added. The card,
/// and the rows.
pub fn card_list(css: &str) -> (Document, NodeId, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let card = element(&mut doc, body, "div", "card");
    let list = element(&mut doc, card, "div", "list");
    let rows: Vec<NodeId> = (0..40)
        .map(|_| element(&mut doc, list, "div", "row"))
        .collect();
    let css = format!("{CARD} .list {{ height: 400px; }} .row {{ height: 10px; }} {css}");
    doc.add_stylesheet("page.css", &css).expect("parses");
    (doc, card, rows)
}
