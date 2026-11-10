//! A frame whose HUD writes equal values does no document work (IU2):
//! after the HUD settles, `layout_hud` with an unchanged race restyles
//! and shapes nothing, notes no change, and lays out only the boxes an
//! update with no writes at all lays out; a changed speed still reaches
//! the drawn text.
//!
//! The HUD's layout misses its cache for 19 boxes on every update, with
//! or without writes, so `boxes_laid_out` is compared with that idle
//! update rather than with 0; `equal_writes_bindings` in the elements
//! crate shows 0 on a document whose layout cache holds.
use super::Racer;
use std::{cell::Cell, rc::Rc};
use vapors_elements::{Document, Viewport};
use vapors_signals::{Effect, Owner};

const VIEW: Viewport = Viewport {
    width: 1100.0,
    height: 720.0,
    scale: 1.0,
};

/// Boxes an update with no writes lays out.
fn idle_boxes(racer: &Racer) -> usize {
    let mut doc = racer.document.borrow_mut();
    doc.update(VIEW).expect("updates");
    doc.boxes_laid_out()
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
    let idle = idle_boxes(&racer);

    // When
    racer.layout_hud(VIEW).expect("lays out");

    // Then
    let mut doc = racer.document.borrow_mut();
    let work = (
        doc.elements_restyled(),
        doc.boxes_laid_out(),
        doc.text_layouts_built(),
    );
    assert_eq!(work, (0, idle, 0));
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

#[test]
fn a_frame_of_equal_hud_writes_notifies_no_hud_reader() {
    // Given: an effect reading every signal `layout_hud` writes
    let mut racer = settled();
    let owner = Owner::new();
    owner.set();
    let hud = &racer.hud;
    let (flags, texts) = (
        [hud.short, hud.compact, hud.follow, hud.enabled, hud.paused],
        [hud.message, hud.speed, hud.lap, hud.clock, hud.best],
    );
    let runs = Rc::new(Cell::new(0));
    Effect::new({
        let runs = Rc::clone(&runs);
        move || {
            for flag in flags {
                flag.with(|_| ());
            }
            for text in texts {
                text.with(|_| ());
            }
            runs.set(runs.get() + 1);
        }
    });

    // When
    racer.layout_hud(VIEW).expect("lays out");

    // Then
    assert_eq!(runs.get(), 1);
}
