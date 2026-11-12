//! A frame whose HUD writes equal values does no document work (IU2):
//! after the HUD settles, `layout_hud` with an unchanged race restyles
//! and shapes nothing and notes no change; a changed speed still reaches
//! the drawn text.
//!
//! Box layout is not measured here. The HUD's layout misses its cache
//! for 19 boxes on every update, with or without writes, and with the
//! writes put back on `set` and the binding comparisons removed
//! `boxes_laid_out` stayed equal to an update with no writes, so it
//! would not catch a regression. `equal_writes_bindings` in the elements
//! crate measures it on a document whose layout cache holds.
use super::Racer;
use std::{cell::Cell, rc::Rc};
use vapors_elements::{Document, Viewport};
use vapors_signals::{Effect, Owner};

const VIEW: Viewport = Viewport {
    width: 1100.0,
    height: 720.0,
    scale: 1.0,
};

impl Racer {
    /// What a frame does to the HUD: the racer's writes, then the
    /// `ElementsApp`'s selection settling and update.
    fn layout_hud(&mut self, viewport: Viewport) -> Result<(), String> {
        self.sync_hud(viewport);
        Document::settle_selection(&self.document);
        let mut document = self.document.borrow_mut();
        let updated = document.update(viewport).map(drop);
        updated.map_err(|errors| format!("racer HUD refused: {errors:?}"))
    }
}

/// A racer whose HUD has been laid out twice, its change flag taken.
fn settled() -> Racer {
    let mut racer = Racer::new();
    racer.layout_hud(VIEW).expect("lays out");
    racer.layout_hud(VIEW).expect("lays out");
    racer.document.borrow_mut().take_changed();
    racer
}

/// Every text node's text, in document order.
fn texts(doc: &Document) -> Vec<String> {
    doc.elements()
        .into_iter()
        .flat_map(|element| doc.children(element))
        .filter_map(|node| doc.text(node).map(str::to_owned))
        .collect()
}

#[test]
fn a_frame_of_equal_hud_writes_does_no_document_work() {
    // Given
    let mut racer = settled();

    // When
    racer.layout_hud(VIEW).expect("lays out");

    // Then
    let mut doc = racer.document.borrow_mut();
    let work = (doc.elements_restyled(), doc.text_layouts_built());
    assert_eq!(work, (0, 0));
    assert!(!doc.take_changed());
}

#[test]
fn a_changed_speed_reaches_the_drawn_text() {
    // Given
    let mut racer = settled();
    racer.run.race.speed = 100.0;

    // When
    racer.layout_hud(VIEW).expect("lays out");

    // Then
    let doc = racer.document.borrow();
    assert!(
        texts(&doc).iter().any(|text| text == "72"),
        "{:?}",
        texts(&doc)
    );
    assert!(doc.text_layouts_built() >= 1);
}

/// An effect reading every signal `layout_hud` writes; the counter
/// holds how many times it has run.
fn hud_reader(racer: &Racer) -> Rc<Cell<u32>> {
    let hud = &racer.hud;
    let (flags, texts) = (
        [hud.short, hud.compact, hud.follow, hud.enabled, hud.paused],
        [hud.message, hud.speed, hud.lap, hud.clock, hud.best],
    );
    let runs = Rc::new(Cell::new(0));
    let counter = Rc::clone(&runs);
    Effect::new(move || {
        for flag in flags {
            flag.with(|_| ());
        }
        for text in texts {
            text.with(|_| ());
        }
        counter.set(counter.get() + 1);
    });
    runs
}

#[test]
fn a_frame_of_equal_hud_writes_notifies_no_hud_reader() {
    // Given
    let mut racer = settled();
    let owner = Owner::new();
    owner.set();
    let runs = hud_reader(&racer);

    // When
    racer.layout_hud(VIEW).expect("lays out");

    // Then
    assert_eq!(runs.get(), 1);
}
