//! Every pointer event carries the kind of device that made it
//! (`PointerEvent.pointerType`): the mouse's events `Mouse`, a finger's
//! `Touch`, including the taps' `click` and `dblclick` and the capture
//! events. winit 0.30 reports no pen, so no path here makes `Pen`: a
//! path that did would fail the kind check below, to be updated with it.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::num::NonZeroU32;
use std::rc::Rc;

use crate::support::pointer::{SHEET, setup};
use vapors_dom::{Dom, Event, NodeId, PointerKind};
use vapors_elements::Document;

type Doc = Rc<RefCell<Document>>;
type Drive = (&'static str, fn(&Doc), PointerKind);
type Kinds = Rc<RefCell<Vec<(String, PointerKind)>>>;

const NAMES: [&str; 8] = [
    "pointerdown",
    "pointermove",
    "pointerup",
    "pointercancel",
    "click",
    "dblclick",
    "gotpointercapture",
    "lostpointercapture",
];

fn kinds(doc: &Doc, node: NodeId) -> Kinds {
    let seen = Kinds::default();
    for name in NAMES {
        let into = Rc::clone(&seen);
        doc.borrow_mut().listen(
            node,
            name,
            Box::new(move |e: &mut Event| {
                let kind = e.pointer().expect("a pointer event").kind;
                into.borrow_mut().push((e.name().to_owned(), kind));
            }),
        );
    }
    seen
}

fn capturing(doc: &Doc, node: NodeId) {
    doc.borrow_mut().listen(
        node,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
}

fn mouse(doc: &Doc) {
    let at = [5.0, 5.0];
    Document::pointer_moved(doc, at);
    Document::pointer_pressed(doc, at);
    Document::pointer_moved(doc, [6.0, 5.0]);
    Document::pointer_released(doc, [6.0, 5.0]);
    Document::pointer_pressed_clicks(doc, at, NonZeroU32::new(2).unwrap());
    Document::pointer_released(doc, at);
    Document::pointer_pressed(doc, at);
    Document::cancel_pointers(doc);
}

fn touch(doc: &Doc) {
    let at = [5.0, 5.0];
    Document::touch_pressed(doc, 7, at);
    Document::touch_moved(doc, 7, [6.0, 5.0]);
    Document::touch_released(doc, 7, [6.0, 5.0]);
    Document::touch_tapped(doc, at);
    Document::touch_double_tapped(doc, at);
    Document::touch_pressed(doc, 8, at);
    Document::touch_cancelled(doc, 8, at);
}

#[test]
fn each_input_path_sets_the_kind_it_receives() {
    let paths: [Drive; 2] = [
        ("mouse", mouse, PointerKind::Mouse),
        ("touch", touch, PointerKind::Touch),
    ];
    for (path, drive, expected) in paths {
        // Given
        let (doc, _row, a, _b) = setup(SHEET);
        capturing(&doc, a);
        let seen = kinds(&doc, a);

        // When
        drive(&doc);

        // Then: every event type came to the listener, each of the path's kind
        let seen = seen.borrow();
        let names: BTreeSet<&str> = seen.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, BTreeSet::from(NAMES), "{path}: {seen:?}");
        for (name, kind) in seen.iter() {
            assert_eq!(*kind, expected, "{path}: {name}");
        }
    }
}

#[test]
fn the_names_a_page_reads_are_the_webs() {
    let names = [
        (PointerKind::Mouse, "mouse"),
        (PointerKind::Touch, "touch"),
        (PointerKind::Pen, "pen"),
    ];
    for (kind, name) in names {
        assert_eq!(kind.as_str(), name);
    }
}
