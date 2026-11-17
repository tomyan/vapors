//! GPU tier: the painter's clip table (flat-scene F3.1,
//! `docs/plans/flat-scene-f3.md`): quads drawn under a clip in a scroll
//! frame take it from the clip table, so the first frame writes their
//! entries; a scroll writes only the entry of the clip entering the view
//! and the poison of the one leaving it, none of those staying (each is
//! in the frame's coordinates), and an idle frame none; a clip scrolled
//! out of view gives its entry
//! back, and scrolled back draws as it first did, its bytes naming the
//! entry it takes then.

use crate::support_surface as support;

use support::{Rig, document, update};
use vapors_app::FrameCounts;
use vapors_dom::Dom;
use vapors_elements::Document;

/// A list 12 CSS px tall of 4 px rows, each a 6 px box that clips a
/// 12 px wide one: each row's quad is drawn under its box's clip.
const CSS: &str = ".list { overflow: auto; width: 12px; height: 12px; } \
    .row { height: 4px; } \
    .cut { overflow: hidden; width: 6px; height: 4px; } \
    .wide { width: 12px; height: 4px; background-color: #f00; } \
    .narrow { width: 3px; }";

/// Device pixels per CSS pixel.
const SCALE: f32 = 2.0;

/// The list of `CSS` with twelve rows, updated.
fn list() -> Document {
    let mut elements = vec![(None, "div", "list")];
    for row in 0..12 {
        let at = 1 + row * 3;
        elements.push((Some(0), "div", "row"));
        elements.push((Some(at), "div", "cut"));
        elements.push((Some(at + 1), "div", "wide"));
    }
    let mut doc = document(CSS, &elements);
    update(&mut doc, SCALE);
    doc
}

/// `doc`'s list scrolled down by `by` CSS pixels, updated.
fn scrolled(doc: &mut Document, by: f32) {
    let list = doc.children(doc.body())[0];
    assert!(doc.scroll_node_by(list, [0.0, by]), "the list scrolls");
    update(doc, SCALE);
}

/// `doc` drawn by `rig`: the pixels and the frame's counts.
fn drawn(rig: &mut Rig, doc: &Document) -> (Vec<u8>, FrameCounts) {
    let pixels = rig.draw(doc).expect("paints");
    (pixels, rig.painter.frame_counts())
}

#[test]
fn gpu_the_first_frame_writes_the_clips_drawn_under_and_a_scroll_none() {
    // Given
    let mut doc = list();
    let mut rig = Rig::new();

    // When
    let (_, first) = drawn(&mut rig, &doc);
    scrolled(&mut doc, 4.0);
    let (_, scroll) = drawn(&mut rig, &doc);
    let (_, idle) = drawn(&mut rig, &doc);

    // Then: one entry a row in view, 48 bytes each; none after
    assert!(first.clip_writes > 0, "{first:?}");
    assert_eq!(first.clip_bytes, first.clip_writes * 48, "{first:?}");
    assert_eq!(
        scroll.clip_writes, 2,
        "the row entering, the poison of the row leaving: {scroll:?}"
    );
    assert_eq!((idle.clip_writes, idle.clip_bytes), (0, 0), "{idle:?}");
}

#[test]
fn gpu_a_clip_scrolled_away_and_back_draws_as_it_first_did() {
    // Given: drawn at the top
    let mut doc = list();
    let mut rig = Rig::new();
    let (top, _) = drawn(&mut rig, &doc);

    // When: scrolled to the end (the first rows' clips given back) and
    // back to the top
    scrolled(&mut doc, 36.0);
    drawn(&mut rig, &doc);
    scrolled(&mut doc, -36.0);
    let (back, counts) = drawn(&mut rig, &doc);

    // Then
    assert!(top.contains(&255), "the rows drew nothing");
    assert!(back == top, "the rows scrolled back differ: {counts:?}");
}

#[test]
fn gpu_a_clip_resized_under_kept_bytes_draws_as_a_fresh_painter_does() {
    // Given: drawn, then the first row's clip made narrower, its quad
    // unchanged, so only the clip entry its kept bytes name changes
    let mut doc = list();
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);
    let list_node = doc.children(doc.body())[0];
    let row = doc.children(list_node)[0];
    let cut = doc.children(row)[0];
    doc.set_attribute(cut, "class", "cut narrow");
    update(&mut doc, SCALE);

    // When
    let (kept, counts) = drawn(&mut rig, &doc);
    let (fresh, _) = drawn(&mut Rig::new(), &doc);

    // Then
    assert!(
        kept == fresh,
        "the resized clip drew other than afresh: {counts:?}"
    );
}

#[test]
fn gpu_text_and_a_quad_under_one_clip_in_a_scrolled_frame_draw_as_afresh() {
    // Given: each row's clipped box holds text as well as its quad, so a
    // text batch (still under the scissor until F3.1b) and a quad batch
    // (on the clip table) share a clip in the scroll frame; the debug
    // build holds the two answers together (painter_clips.rs
    // `assert_agrees`)
    let mut doc = list();
    let list_node = doc.children(doc.body())[0];
    for row in doc.children(list_node) {
        let wide = doc.children(doc.children(row)[0])[0];
        let text = doc.create_text("x");
        doc.append(wide, text);
    }
    update(&mut doc, SCALE);
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);

    // When: scrolled, so the frame's shift is not zero
    scrolled(&mut doc, 4.0);
    let (kept, _) = drawn(&mut rig, &doc);
    let (fresh, _) = drawn(&mut Rig::new(), &doc);

    // Then
    assert!(kept == fresh, "the scrolled rows drew other than afresh");
}
