//! A field's caret moves with its glyphs onto the baseline Blink gives
//! the line (#368). The editor lays its text out in CSS pixels, so at 2x
//! its baseline (9 px below the line's top, for 10 px Geist in a 12 px
//! line) is half a pixel above the glyphs' (19 device px, 9.5 CSS px);
//! the caret comes down to them. Headless Chromium 155 puts a span's
//! content area, where the caret stands, 1 px above the line at 1x and
//! 0.5 px at 2x (`inline_baseline.rs`); the field's content box starts
//! 2 px down (a 1 px border and 1 px padding).

mod support;

use support::text_page::{Page, click, key, page};
use vapors_scene::{Color, Item};

/// The top of the one mark painted over the first field (a caret in
/// black, or a selection in any other colour, as `black` says), device
/// pixels, the field at `scale`.
fn mark_top(page: &Page, scale: f32, black: bool) -> f32 {
    let mut doc = page.doc.borrow_mut();
    let mut at = support::viewport(400.0, 100.0);
    at.scale = scale;
    let painted = doc.update(at).expect("updates");
    let scene = painted.scene().to_scene();
    let marks: Vec<f32> = scene
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(q)
                if q.rect.top < 24.0 * scale
                    && q.background.a > 0.0
                    && q.background != Color::from_rgba8(255, 255, 255, 255)
                    && (q.background == Color::from_rgba8(0, 0, 0, 255)) == black =>
            {
                Some(q.rect.top)
            }
            _ => None,
        })
        .collect();
    let [top] = marks[..] else {
        panic!("one mark, got {marks:?}");
    };
    top
}

/// The text page, its first field 10 px text in 12 px lines.
fn small() -> Page {
    let page = page();
    let small = "input { font-size: 10px; line-height: 12px; }";
    page.doc
        .borrow_mut()
        .add_stylesheet("small.css", small)
        .expect("parses");
    page
}

/// `small`, focused by a click after its text: a caret.
fn focused() -> Page {
    let page = small();
    click(&page, [120.0, 12.0]);
    page
}

#[test]
fn at_1x_the_caret_starts_1_px_above_the_line() {
    // Given
    let page = focused();

    // When
    let top = mark_top(&page, 1.0, true);

    // Then: the content box at 2, less 1
    assert_eq!(top, 1.0);
}

#[test]
fn at_2x_the_caret_comes_down_half_a_pixel_with_the_glyphs() {
    // Given
    let page = focused();

    // When
    let top = mark_top(&page, 2.0, true);

    // Then: the content box at 4 device px, less 1 (0.5 CSS px)
    assert_eq!(top, 3.0);
}

#[test]
fn at_2x_a_selection_comes_down_with_the_glyphs_too() {
    // Given: focused by Tab, which selects the whole text
    let page = small();
    key(&page, "Tab");

    // When
    let top = mark_top(&page, 2.0, false);

    // Then: as the caret
    assert_eq!(top, 3.0);
}

/// The top of the underline an IME's composition gets, device pixels,
/// the field at `scale`: the black mark wider than the caret.
fn underline_top(page: &Page, scale: f32) -> f32 {
    let mut doc = page.doc.borrow_mut();
    let mut at = support::viewport(400.0, 100.0);
    at.scale = scale;
    let painted = doc.update(at).expect("updates");
    let scene = painted.scene().to_scene();
    let lines: Vec<f32> = scene
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(q)
                if q.background == Color::from_rgba8(0, 0, 0, 255)
                    && q.rect.right - q.rect.left > 4.0 * scale =>
            {
                Some(q.rect.top)
            }
            _ => None,
        })
        .collect();
    let [top] = lines[..] else {
        panic!("one underline, got {lines:?}");
    };
    top
}

#[test]
fn at_2x_an_imes_underline_comes_down_with_the_glyphs_too() {
    // Given: an IME composing "th" after "Warm"
    let page = focused();
    let edit = vapors_elements::TextEdit {
        before: "Warm".into(),
        text: "Warmth".into(),
        selection: (6, 6),
        composing: Some((4, 6)),
    };
    vapors_elements::Document::text_edited(&page.doc, &edit).expect("applies");

    // When
    let top = underline_top(&page, 2.0);

    // Then: the composition's content area ends 12.5 CSS px below the
    // line's top (its top -0.5, 13 tall), 29 device px down the field;
    // the 2 device px underline stands just inside it
    assert_eq!(top, 27.0);
}
