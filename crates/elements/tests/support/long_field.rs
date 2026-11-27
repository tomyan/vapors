//! The long-input page (entry 44): the gui-bench search field's box,
//! a 280 x 36 px text field at (676, 14) with 12 px side padding, a
//! 1 px border and an 8 px radius, in 14 px text, focused by a click
//! and typed into as the platform's text input sends it. Its content
//! box spans x 689..943. Chromium's answers for the same page are
//! `CHROMIUM` (`node scripts/chromium-field-scroll.mjs`).

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Modifiers, TextEdit};
use vapors_scene::{Color, Item, Rect};

use super::{document, viewport};

pub const SHEET: &str = "body { margin: 0; } input { position: absolute; left: 676px; top: 14px; box-sizing: border-box; width: 280px; height: 36px; margin: 0; padding: 0 12px; border: 1px solid #d1d5db; border-radius: 8px; background-color: #f9fafb; color: #111827; font-size: 14px; display: block; }";

/// The content box's x range, device pixels at 1x.
pub const CONTENT: (f32, f32) = (689.0, 943.0);

/// The text's colour, which the caret is drawn in.
pub const INK: Color = Color {
    r: 17.0 / 255.0,
    g: 24.0 / 255.0,
    b: 39.0 / 255.0,
    a: 1.0,
};

/// Chromium 157.0.8103.0 on the same page, typing "W" 60 times with
/// `Input.insertText` and deleting with Backspace: per text length, the
/// field's `scrollLeft` and the column its red caret was painted in
/// (`caret-color: red`, from a screenshot), as
/// `node scripts/chromium-field-scroll.mjs` prints them.
pub const CHROMIUM_TYPED: [(usize, f32, f32); 6] = [
    (18, 0.0, 926.0),
    (19, 0.0, 940.0),
    (20, 10.0, 942.0),
    (21, 23.0, 942.0),
    (30, 142.0, 942.0),
    (60, 538.0, 942.0),
];

/// As [`CHROMIUM_TYPED`], deleting from 60.
pub const CHROMIUM_DELETED: [(usize, f32, f32); 3] =
    [(59, 526.0, 941.0), (30, 142.0, 942.0), (19, 0.0, 940.0)];

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    pub field: NodeId,
    pub width: f32,
}

/// The page at a 1280 px window, the field empty and focused by a
/// click.
pub fn page() -> Page {
    page_with(SHEET, 1280.0)
}

/// A page whose field is styled by `sheet`, at a `width` px window,
/// focused by a click at the field's left.
pub fn page_with(sheet: &str, width: f32) -> Page {
    page_of("text", sheet, width)
}

/// As [`page_with`], the field an `<input type=kind>`.
pub fn page_of(kind: &str, sheet: &str, width: f32) -> Page {
    let mut doc = document();
    let body = doc.body();
    let field = doc.create_element("input");
    doc.set_attribute(field, "type", kind);
    doc.append(body, field);
    doc.add_stylesheet("app.css", sheet).expect("parses");
    let page = Page {
        doc: Rc::new(RefCell::new(doc)),
        field,
        width,
    };
    update(&page);
    let [x, y, _, h] = border_box(&page);
    click(&page, [x + 14.0, y + h / 2.0]);
    page
}

/// The update at the page's window.
pub fn update(page: &Page) {
    page.doc
        .borrow_mut()
        .update(viewport(page.width, 800.0))
        .expect("updates");
}

pub fn border_box(page: &Page) -> [f32; 4] {
    let doc = page.doc.borrow();
    let rect = doc
        .painted()
        .and_then(|p| p.border_box(page.field))
        .expect("laid out");
    [rect.x, rect.y, rect.width, rect.height]
}

/// A press and release at `point`, then an update.
pub fn click(page: &Page, point: [f32; 2]) {
    Document::pointer_moved(&page.doc, point);
    Document::pointer_pressed(&page.doc, point);
    Document::pointer_released(&page.doc, point);
    update(page);
}

/// One more "W" at the caret, as the platform's text input sends it,
/// then an update.
pub fn type_w(page: &Page) {
    type_text(page, "W");
}

/// `typed` at the caret, as the platform's text input sends it, then
/// an update.
pub fn type_text(page: &Page, typed: &str) {
    let before = page.doc.borrow().value(page.field).expect("a value");
    let text = format!("{before}{typed}");
    let end = text.encode_utf16().count();
    let edit = TextEdit {
        before,
        text,
        selection: (end, end),
        composing: None,
    };
    Document::text_edited(&page.doc, &edit).expect("applies");
    update(page);
}

/// Backspace, then an update.
pub fn backspace(page: &Page) {
    Document::key_pressed(&page.doc, "Backspace", Modifiers::default());
    Document::key_released(&page.doc, "Backspace", Modifiers::default());
    update(page);
}

/// The field's text length.
pub fn length(page: &Page) -> usize {
    page.doc.borrow().value(page.field).expect("a value").len()
}

/// The field's scroll offset on x.
pub fn offset(page: &Page) -> f32 {
    let offset = page.doc.borrow().scroll_offset(page.field);
    offset.expect("the field scrolls")[0]
}

/// The caret's column as drawn: its left edge cut to its clip, `None`
/// when the clip hides it or none is painted.
#[allow(clippy::float_cmp)] // a caret's edges are snapped to whole device pixels
pub fn caret_column(page: &Page) -> Option<f32> {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("updated");
    let items = painted.scene().items();
    let shown = items.filter_map(|(item, clip)| match item {
        Item::Quad(q) if q.background == INK && q.rect.right - q.rect.left == 1.0 => {
            let clip = clip.map_or(q.rect, |clip| clip.rect);
            (q.rect.left >= clip.left && q.rect.right <= clip.right).then_some(q.rect.left)
        }
        _ => None,
    });
    shown.last()
}

/// The clip the field's text is drawn under, device pixels.
pub fn text_clip(page: &Page) -> Option<Rect> {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("updated");
    let texts: Vec<Option<Rect>> = painted
        .scene()
        .items()
        .filter_map(|(item, clip)| {
            matches!(item, Item::Text { .. }).then_some(clip.map(|clip| clip.rect))
        })
        .collect();
    let [clip] = texts[..] else {
        panic!("one text item, got {texts:?}");
    };
    clip
}
