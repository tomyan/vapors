//! A focus ring at fractional scales: its reach, `(offset + width) *
//! scale`, is not whole there, and the ring is drawn as it is at whole
//! scales, the border box grown by that reach (flat-scene F1's item
//! check takes a ring by the box it is grown from).

use std::cell::RefCell;
use std::rc::Rc;

use crate::support;

use support::{document, element};
use vapors_elements::{Document, Viewport};
use vapors_scene::Item;

/// The UA sheet's focus ring, on a 60x20 button at (0, 0).
const CSS: &str = "body { margin: 0; } button { display: block; width: 60px; height: 20px; } \
    button:focus { outline: 2px solid #0060df; outline-offset: 1px; }";

/// The last item drawn after the button takes focus, at `scale`.
fn ring_at(scale: f32) -> Item {
    let view = Viewport {
        width: 200.0,
        height: 100.0,
        scale,
    };
    let mut doc = document();
    let body = doc.body();
    let button = element(&mut doc, body, "button", "");
    doc.add_stylesheet("app.css", CSS).expect("parses");
    doc.update(view).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    Document::focus(&doc, Some(button));
    let mut doc = doc.borrow_mut();
    let painted = doc.update(view).expect("updates");
    let items = painted.scene().to_scene().items().to_vec();
    items.last().copied().expect("an item")
}

#[test]
fn a_focus_ring_at_a_fractional_scale_rings_the_snapped_border_box() {
    for (scale, reach) in [(1.5_f32, 4.5_f32), (2.75, 7.75)] {
        // Given, when
        let ring = ring_at(scale);

        // Then: the border box, whole, grown by the unsnapped reach
        let Item::Quad(ring) = ring else {
            panic!("the ring last at {scale}: {ring:?}")
        };
        let right = (60.0 * scale).round();
        let bottom = (20.0 * scale).round();
        let expected = [-reach, -reach, right + reach, bottom + reach];
        let edges = [
            ring.rect.left,
            ring.rect.top,
            ring.rect.right,
            ring.rect.bottom,
        ];
        // to a thousandth: the reach's width is a snapped width divided
        // by the scale and multiplied back (7.7500005 at 2.75)
        let near = edges
            .iter()
            .zip(expected)
            .all(|(edge, want)| (edge - want).abs() < 1e-3);
        assert!(near, "at {scale}: {edges:?}, expected {expected:?}");
    }
}
