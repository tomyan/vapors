//! Every pointer event carries the kind of device that made it
//! (`PointerEvent.pointerType`): the mouse's events `Mouse`, a finger's
//! `Touch`, including the taps' `click` and `dblclick` and the capture
//! events. winit 0.30 reports no pen, so no path here makes `Pen`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::pointer::{SHEET, setup};
use vapors_dom::{Dom, Event, PointerKind};
use vapors_elements::Document;

type Doc = Rc<RefCell<Document>>;
type Drive = fn(&Doc);
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

fn kinds(doc: &Doc, node: vapors_dom::NodeId) -> Kinds {
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

fn mouse(doc: &Doc) {
    Document::pointer_moved(doc, [5.0, 5.0]);
    Document::pointer_pressed(doc, [5.0, 5.0]);
    Document::pointer_moved(doc, [6.0, 5.0]);
    Document::pointer_released(doc, [6.0, 5.0]);
    Document::pointer_pressed_clicks(doc, [6.0, 5.0], std::num::NonZeroU32::new(2).unwrap());
}

fn touch(doc: &Doc) {
    Document::touch_pressed(doc, 7, [5.0, 5.0]);
    Document::touch_moved(doc, 7, [6.0, 5.0]);
    Document::touch_released(doc, 7, [6.0, 5.0]);
    Document::touch_tapped(doc, [6.0, 5.0]);
    Document::touch_double_tapped(doc, [6.0, 5.0]);
    Document::touch_pressed(doc, 8, [5.0, 5.0]);
    Document::touch_cancelled(doc, 8, [5.0, 5.0]);
}

#[test]
fn each_input_path_sets_the_kind_it_receives() {
    let paths: [(&str, Drive, PointerKind); 2] = [
        ("mouse", mouse, PointerKind::Mouse),
        ("touch", touch, PointerKind::Touch),
    ];
    for (path, drive, expected) in paths {
        // Given
        let (doc, _row, a, _b) = setup(SHEET);
        let seen = kinds(&doc, a);

        // When
        drive(&doc);

        // Then: events came, every one of the path's kind
        let seen = seen.borrow();
        assert!(seen.len() >= 5, "{path}: {seen:?}");
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
