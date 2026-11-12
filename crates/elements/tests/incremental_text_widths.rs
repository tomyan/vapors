//! A text whose width changes in a shrink-to-fit box, as the racer's
//! clock does in its HUD: the run that changed is laid out at
//! max-content and min-content, and a start-aligned run at a width no
//! narrower than its max-content takes that layout instead of a new
//! one, its own and its neighbours' alike. The result matches a full
//! recomputation (`support::oracle`).

mod support;

use std::{cell::RefCell, rc::Rc};
use support::oracle::assert_matches_full;
use support::{document, element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

const CSS: &str = "
.stats { position: absolute; top: 16px; right: 16px; display: flex; gap: 18px; }
.value { font-size: 22px; font-weight: 700; }
";

const VIEW: Viewport = Viewport {
    width: 1100.0,
    height: 720.0,
    scale: 1.0,
};

/// A row of two stats, caption over value; the clock's text node.
fn hud() -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("hud.css", CSS).expect("parses");
    let body = doc.body();
    let stats = element(&mut doc, body, "div", "stats");
    let mut clock = None;
    for (caption, value) in [("LAP", "1 / 3"), ("TIME", "00:00.00")] {
        let stat = element(&mut doc, stats, "div", "stat");
        let label = element(&mut doc, stat, "div", "caption");
        text(&mut doc, label, caption);
        let shown = element(&mut doc, stat, "div", "value");
        clock = Some(text(&mut doc, shown, value));
    }
    doc.update(VIEW).expect("updates");
    (Rc::new(RefCell::new(doc)), clock.expect("two stats"))
}

#[test]
fn a_clock_tick_lays_out_only_the_clock_at_max_and_min_content() {
    // Given
    let (doc, clock) = hud();

    // When: ticks that change the clock's width, the second after
    // one where the caption was asked for only at its box's width
    for tick in ["00:00.01", "00:00.03"] {
        doc.borrow_mut().set_text(clock, tick);
        doc.borrow_mut().update(VIEW).expect("updates");
    }

    // Then: the clock at max-content and min-content; at its box's
    // width it fits, as its caption does at the new width
    assert_eq!(doc.borrow().text_layouts_built(), 2);
    assert_matches_full(&doc, VIEW, "after a clock tick");
}
