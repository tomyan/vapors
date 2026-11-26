//! A long search (entry 44): the search input's text scrolls inside its
//! content box as Chromium 157's does, so after 60 characters its caret
//! still shows in the content box's last column and its text is cut off
//! at the content box (`crates/elements/tests/field_scroll.rs` holds the
//! field to Chromium's numbers for this box).
#![allow(clippy::float_cmp)] // the caret's edges are snapped to whole device pixels
use super::behaviour_top::centre_of;
use super::support::*;
use crate::dashboard::*;
use vapors_elements::{Document, TextEdit};
use vapors_scene::{Color, Item};

/// The search input's text colour, which the caret is drawn in.
const INK: Color = Color {
    r: 17.0 / 255.0,
    g: 24.0 / 255.0,
    b: 39.0 / 255.0,
    a: 1.0,
};

#[test]
fn a_long_search_keeps_its_caret_and_text_in_the_inputs_content_box() {
    // Given: the content box, inside a 1 px border and 12 px padding
    let mounted = laid_out();
    let content = (SEARCH_X + 13.0, SEARCH_X + SEARCH_WIDTH - 13.0);
    click(&mounted, centre_of(&mounted, "search-input", 0));

    // When
    let text = "W".repeat(60);
    let edit = TextEdit {
        before: String::new(),
        text,
        selection: (60, 60),
        composing: None,
    };
    Document::text_edited(&mounted.document, &edit).expect("the field takes the edit");
    refresh(&mounted);

    // Then: one caret, in the last column; the text under the content clip
    let doc = mounted.document.borrow();
    let items: Vec<_> = doc.painted().expect("laid out").scene().items().collect();
    let carets: Vec<f32> = items
        .iter()
        .filter_map(|(item, _)| match item {
            Item::Quad(q) if q.background == INK && q.rect.right - q.rect.left == 1.0 => {
                Some(q.rect.left)
            }
            _ => None,
        })
        .collect();
    assert_eq!(carets, [content.1 - 1.0]);
    let clips: Vec<_> = items
        .iter()
        .filter(|(item, _)| matches!(item, Item::Text { .. }))
        .filter_map(|(_, clip)| clip.map(|c| (c.left, c.right)))
        .filter(|clip| *clip == content)
        .collect();
    assert_eq!(clips.len(), 1, "the search text under the content box");
}
