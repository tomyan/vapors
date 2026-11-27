//! A text field keeps the `scrollbar-width` its author asks for (`auto`
//! where none is asked, as Chromium computes), and its text still
//! scrolls, but no thumb is drawn over the text, as in Chromium 157
//! (`node scripts/chromium-field-scroll.mjs scrollbar`): the long-input
//! page with a green `scrollbar-color` paints no green on the scrolled
//! input, while a scrolling `div` styled the same paints a thumb.

use crate::support;

use support::long_field::{Page, SHEET, offset, page_with, type_w};
use vapors_css::value::Value;
use vapors_scene::{Color, Item};

const GREEN: Color = Color {
    r: 0.0,
    g: 1.0,
    b: 0.0,
    a: 1.0,
};

/// The long-input page with a green thumb and the author's
/// `scrollbar-width: width` (none given for `None`), empty.
fn field(width: Option<&str>) -> Page {
    let asked = width.map_or(String::new(), |w| format!("scrollbar-width: {w};"));
    let sheet = format!("{SHEET} input {{ {asked} scrollbar-color: #00ff00 transparent; }}");
    page_with(&sheet, 1280.0)
}

/// As [`field`], 60 characters typed.
fn scrolled_field(width: Option<&str>) -> Page {
    let page = field(width);
    for _ in 0..60 {
        type_w(&page);
    }
    page
}

fn computed_width(page: &Page) -> Value {
    let doc = page.doc.borrow();
    let style = doc.computed_style(page.field).expect("styled");
    style
        .get_by_name("scrollbar-width")
        .expect("a longhand")
        .clone()
}

fn green_quads(page: &Page) -> usize {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("updated");
    let items = painted.scene().items();
    items
        .filter(|(item, _)| matches!(item, Item::Quad(q) if q.background == GREEN))
        .count()
}

#[test]
fn a_text_fields_scrollbar_width_is_the_authors_or_auto() {
    for (asked, computed) in [
        (None, "auto"),
        (Some("auto"), "auto"),
        (Some("thin"), "thin"),
    ] {
        // Given / When
        let page = field(asked);

        // Then: as Chromium computes
        assert_eq!(
            computed_width(&page),
            Value::Keyword(computed.to_owned()),
            "{asked:?}"
        );
    }
}

#[test]
fn a_scrolled_text_field_draws_no_thumb_whatever_its_scrollbar_width() {
    for width in ["auto", "thin"] {
        // Given / When
        let page = scrolled_field(Some(width));

        // Then: scrolled, as Chromium's (538), and no thumb, as Chromium's
        assert!(offset(&page) > 0.0, "{width}: the field scrolls");
        assert_eq!(green_quads(&page), 0, "{width}: a thumb is drawn");
    }
}
