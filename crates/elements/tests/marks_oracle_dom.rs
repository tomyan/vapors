//! Box marks (IU3 PR 1b): each change the DOM, the image registry or
//! the viewport makes marks the boxes it changes, so the update after
//! it matches a full update. Each case fails without its mark.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::svg_page::with_attributes;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId, PropertyValue};
use vapors_elements::{Document, Viewport};
use vapors_scene::{RgbaImage, Size};

const CSS: &str = ".row { display: flex; height: 40px; } .cell { width: 20vw; border-top-width: 1.5px; border-top-style: solid; }";

const SVG: &[(&str, &str)] = &[
    ("viewBox", "0 0 10 10"),
    ("width", "10"),
    ("height", "10"),
    ("aria-hidden", "true"),
];
const PATH: &[(&str, &str)] = &[
    ("d", "M0 0H10V10Z"),
    ("fill", "none"),
    ("stroke", "currentColor"),
];

struct Page {
    doc: Rc<RefCell<Document>>,
    rows: [NodeId; 2],
    cell: NodeId,
    words: NodeId,
    img: NodeId,
    path: NodeId,
    checkbox: NodeId,
}

fn image(width: u32, height: u32) -> RgbaImage {
    RgbaImage {
        size: Size { width, height },
        rgba: [0, 0, 255, 255].repeat((width * height) as usize),
    }
}

/// Two rows, the first holding a cell with words, an `img`, an `svg`
/// path and a checkbox; updated once at `view()`.
fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let rows = [0, 1].map(|_| element(&mut doc, body, "div", "row"));
    let cell = element(&mut doc, rows[0], "div", "cell");
    let words = text(&mut doc, cell, "One");
    doc.register_image("small", image(10, 10))
        .expect("drawable");
    doc.register_image("large", image(30, 20))
        .expect("drawable");
    let img = element(&mut doc, rows[0], "img", "");
    doc.set_attribute(img, "src", "small");
    let svg = element(&mut doc, rows[0], "svg", "");
    with_attributes(&mut doc, svg, SVG);
    let path = element(&mut doc, svg, "path", "");
    with_attributes(&mut doc, path, PATH);
    let checkbox = element(&mut doc, rows[1], "input", "");
    doc.set_attribute(checkbox, "type", "checkbox");
    doc.add_stylesheet("app.css", CSS).expect("parses");
    doc.update(view()).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    Page {
        doc,
        rows,
        cell,
        words,
        img,
        path,
        checkbox,
    }
}

fn view() -> Viewport {
    viewport(400.0, 100.0)
}

/// Fails, naming `context`, when the update after `edit` differs from
/// a full update at `at`.
fn assert_edit_matches_full(edit: impl FnOnce(&Page, &mut Document), at: Viewport, context: &str) {
    let page = page();
    edit(&page, &mut page.doc.borrow_mut());
    assert_matches_full(&page.doc, at, context);
}

/// A test per edit, each named for what it checks.
macro_rules! edits_match_a_full_update {
    ($($name:ident: $edit:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_edit_matches_full($edit, view(), stringify!($name));
        }
    )*};
}

edits_match_a_full_update! {
    an_img_src_change_matches_a_full_update: |p, d| d.set_attribute(p.img, "src", "large");
    an_svg_path_change_matches_a_full_update: |p, d| d.set_attribute(p.path, "d", "M0 0H5V5Z");
    a_checked_property_matches_a_full_update: |p, d| {
        d.set_property(p.checkbox, "checked", PropertyValue::Bool(true));
    };
    a_text_change_matches_a_full_update: |p, d| d.set_text(p.words, "One, longer");
    a_cell_moved_to_another_row_matches_a_full_update: |p, d| {
        d.insert_before(p.rows[1], p.cell, None);
    };
    a_removed_cell_matches_a_full_update: |p, d| d.remove(p.cell);
    an_appended_cell_matches_a_full_update: |p, d| {
        element(d, p.rows[1], "div", "cell");
    };
    an_image_registered_again_matches_a_full_update: |_, d| {
        d.register_image("small", image(30, 10)).expect("drawable");
    };
}

#[test]
fn a_resize_that_changes_vw_matches_a_full_update() {
    assert_edit_matches_full(|_, _| {}, viewport(300.0, 100.0), "resize");
}

#[test]
fn a_scale_change_matches_a_full_update() {
    let scaled = Viewport {
        scale: 2.0,
        ..view()
    };
    assert_edit_matches_full(|_, _| {}, scaled, "scale");
}

#[test]
fn forgetting_what_is_kept_builds_every_box() {
    // Given
    let page = page();
    let all = page.doc.borrow().update_counts().boxes_built;
    page.doc.borrow_mut().set_text(page.words, "Two");

    // When
    let mut doc = page.doc.borrow_mut();
    doc.forget_kept();
    doc.update(view()).expect("updates");

    // Then
    assert_eq!(doc.update_counts().boxes_built, all);
}
