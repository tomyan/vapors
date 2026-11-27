//! A text or number field's text, caret and placeholder sit centred on
//! y in its content box, as Chromium 157's `LayoutTextControlSingleLine`
//! centres its inner editor (entry 84 action 33; `node
//! scripts/chromium-field-scroll.mjs centre`). On the long-input page
//! (content box rows 15..49) Chromium draws, for a text and a number
//! field alike, and with an author `display: flex; align-items:
//! flex-start` on the input too, the caret in rows 24..39 (centred on
//! 32) and the typed text's and the placeholder's ink down to row 36.
//! The glyphs are Geist, not Chromium's sans-serif, so the caret's
//! centre is compared, within a row, and the placeholder and the typed
//! text are held to one line; their ink rows against a capture are in
//! the PR (entry 84).

use crate::support;

use support::long_field::{INK, Page, SHEET, page_of, type_text, update};
use vapors_dom::Dom;
use vapors_scene::{Item, Rect};

/// Chromium's caret centre, the middle of rows 24..39.
const CARET_CENTRE: f32 = 32.0;

/// (case, input type, author declarations, typed text), as the probe's.
/// The number fields are given `overflow: visible`: a rounded number
/// field's user-agent `overflow: hidden` is a refused rounded clip
/// (`clipping_refused.rs`), and Chromium's ignores the input's overflow.
const CASES: [(&str, &str, &str, &str); 8] = [
    ("text", "text", "", "Ada"),
    ("number", "number", "overflow: visible;", "123"),
    ("text, author flex start", "text", FLEX_START, "Ada"),
    (
        "number, author flex start",
        "number",
        NUMBER_FLEX_START,
        "123",
    ),
    ("text, author wrapped row end", "text", ROW_END, "Ada"),
    (
        "number, author wrapped row end",
        "number",
        NUMBER_ROW_END,
        "123",
    ),
    ("text, author grid end", "text", GRID_END, "Ada"),
    ("number, author grid end", "number", NUMBER_GRID_END, "123"),
];

const FLEX_START: &str = "display: flex; align-items: flex-start;";
const NUMBER_FLEX_START: &str = "overflow: visible; display: flex; align-items: flex-start;";
const ROW_END: &str =
    "display: flex; flex-direction: row-reverse; flex-wrap: wrap; justify-content: flex-end;";
const NUMBER_ROW_END: &str = "overflow: visible; display: flex; flex-direction: row-reverse; flex-wrap: wrap; justify-content: flex-end;";
const GRID_END: &str = "display: grid; align-items: end;";
const NUMBER_GRID_END: &str = "overflow: visible; display: grid; align-items: end;";

fn page(kind: &str, declarations: &str) -> Page {
    let sheet = format!("{SHEET} input {{ {declarations} }}");
    let page = page_of(kind, &sheet, 1280.0);
    page.doc
        .borrow_mut()
        .set_attribute(page.field, "placeholder", "Search");
    update(&page);
    page
}

/// `typed` into the focused field: a text field's by the platform's
/// text input; a number field, which takes no text input edits (its
/// keys edit it), given it as its value.
fn enter(page: &Page, kind: &str, typed: &str) {
    if kind == "text" {
        type_text(page, typed);
        return;
    }
    page.doc
        .borrow_mut()
        .set_attribute(page.field, "value", typed);
    update(page);
}

/// The rows of the painted caret, a 1 px quad in the text's colour.
fn caret_rows(page: &Page) -> (f32, f32) {
    let caret = caret(page);
    (caret.top, caret.bottom)
}

/// The painted caret, a 1 px quad in the text's colour.
#[allow(clippy::float_cmp)] // a caret's edges are snapped to whole device pixels
fn caret(page: &Page) -> Rect {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("updated");
    let carets: Vec<Rect> = painted
        .scene()
        .items()
        .filter_map(|(item, _)| match item {
            Item::Quad(q) if q.background == INK && q.rect.right - q.rect.left == 1.0 => {
                Some(q.rect)
            }
            _ => None,
        })
        .collect();
    let [caret] = carets[..] else {
        panic!("one caret, got {carets:?}");
    };
    caret
}

/// The tops of the text lines painted: an empty field's blank run and
/// its placeholder, or its value.
fn line_tops(page: &Page) -> Vec<f32> {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("updated");
    painted
        .scene()
        .items()
        .filter_map(|(item, _)| match item {
            Item::Text { origin, .. } => Some(origin[1]),
            _ => None,
        })
        .collect()
}

#[test]
fn the_typed_text_and_caret_are_centred_as_chromiums() {
    for (case, kind, declarations, typed) in CASES {
        // Given
        let page = page(kind, declarations);

        // When
        enter(&page, kind, typed);

        // Then
        let (top, bottom) = caret_rows(&page);
        let centre = f32::midpoint(top, bottom);
        assert!(
            (centre - CARET_CENTRE).abs() <= 1.0,
            "{case}: caret rows {top}..{bottom}"
        );
        assert_eq!(
            line_tops(&page),
            [top],
            "{case}: the text's line is the caret's"
        );
    }
}

#[test]
fn the_placeholder_sits_on_the_typed_texts_line() {
    for (case, kind, declarations, typed) in CASES {
        // Given: the placeholder showing over the blank run
        let page = page(kind, declarations);
        let empty = line_tops(&page);

        // When
        enter(&page, kind, typed);

        // Then: Chromium's both end on row 36
        assert_eq!(empty.len(), 2, "{case}: the blank run and the placeholder");
        assert_eq!(empty, [line_tops(&page)[0]; 2], "{case}");
    }
}

#[test]
fn the_imes_caret_area_is_the_painted_carets_rows() {
    // a number field has no text input, so no IME
    for (case, kind, declarations, typed) in CASES.into_iter().filter(|c| c.1 == "text") {
        // Given
        let page = page(kind, declarations);
        enter(&page, kind, typed);

        // When
        let area = page.doc.borrow_mut().text_input().expect("focused").area;

        // Then: [x, y, width, height]
        let (top, bottom) = caret_rows(&page);
        assert_eq!(
            (area[1].round(), (area[1] + area[3]).round()),
            (top, bottom),
            "{case}"
        );
    }
}

#[test]
fn an_authored_inner_layout_neither_moves_nor_narrows_the_run() {
    for (case, kind, declarations, typed) in CASES {
        // Given: the plain field of the same type, as a reference
        let (plain, at) = if kind == "text" {
            (CASES[0], 0)
        } else {
            (CASES[1], 1)
        };
        let reference = page(kind, plain.2);
        let field = page(kind, declarations);
        let empty = (caret(&reference), caret(&field));

        // When
        enter(&reference, kind, typed);
        enter(&field, kind, typed);

        // Then: Chromium's caret is at column 689 empty, 713 or 712 typed,
        // under every authored layout
        assert_eq!(empty.1, empty.0, "{case} empty (against case {at})");
        assert_eq!(caret(&field), caret(&reference), "{case} typed");
    }
}
