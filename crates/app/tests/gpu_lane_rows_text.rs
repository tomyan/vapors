//! GPU tier: text in a framed lane's rows, drawn from kept bytes through
//! the transform table (`docs/plans/flat-scene-f2.md`): after the first
//! row grows, moving the rows below it while each batch keeps its items'
//! rows and so its bytes, each row's text is where a painter that kept
//! nothing draws it, and the first two rows are as a page of those two
//! alone (no lane, so no row entries) draws them.

use crate::support_surface as support;

use support::{Rig, SIZE, document, update};
use vapors_dom::Dom;
use vapors_elements::Document;

/// Rows 8 CSS px tall, each a label in 7 px text: four in the 32 px
/// target.
const CSS: &str = ".row { height: 8px; font-size: 7px; line-height: 8px; color: #000; } \
                   .tall { height: 12px; }";

/// `count` rows, each labelled with its number, updated at scale 1.
fn labelled_rows(count: usize) -> Document {
    let rows: Vec<_> = (0..count).map(|_| (None, "div", "row")).collect();
    let mut doc = document(CSS, &rows);
    let body = doc.body();
    for (at, row) in doc.children(body).into_iter().enumerate() {
        let label = doc.create_text(&format!("Row {at}"));
        doc.append(row, label);
    }
    update(&mut doc, 1.0);
    doc
}

/// `doc` with its first row made taller, updated.
fn first_row_grown(doc: &mut Document) {
    let first = doc.children(doc.body())[0];
    doc.set_attribute(first, "class", "row tall");
    update(doc, 1.0);
}

#[test]
fn gpu_kept_text_in_rows_moved_by_a_taller_row_is_where_a_fresh_painter_draws_it() {
    // Given: drawn once
    let mut doc = labelled_rows(40);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: the first row made taller, so the rows below it move while
    // each batch keeps its items' rows
    first_row_grown(&mut doc);
    let kept = rig.draw(&doc).expect("paints");

    // Then: the lane is framed, its text drawn from kept bytes (none
    // written), and the pixels are a fresh painter's
    let texts = rig.painter.frame_counts().texts;
    assert!(
        texts.draw_calls > 0 && texts.buffer_bytes == 0,
        "the rows' text was not drawn from kept bytes: {texts:?}"
    );
    let counts = doc.update_counts();
    assert!(counts.lanes_framed > 0, "{counts:?}");
    let fresh = Rig::new().draw(&doc).expect("paints");
    assert!(kept.iter().any(|&v| v < 128), "the text drew nothing");
    assert!(kept == fresh, "kept text in rows differs from a fresh draw");
}

#[test]
fn gpu_kept_text_in_rows_moved_by_a_taller_row_is_where_a_page_with_no_lane_draws_it() {
    // Given: the first two rows alone, too few for a lane
    let mut alone = labelled_rows(2);
    first_row_grown(&mut alone);
    let counts = alone.update_counts();
    assert_eq!(counts.lanes_framed, 0, "{counts:?}");
    let expected = Rig::new().draw(&alone).expect("paints");

    // When: kept in a lane's rows, the first made taller
    let mut doc = labelled_rows(40);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");
    first_row_grown(&mut doc);
    let kept = rig.draw(&doc).expect("paints");

    // Then: the two rows' 20 px are the same
    let two_rows = (20 * SIZE * 4) as usize;
    assert!(
        kept[..two_rows] == expected[..two_rows],
        "the first two rows' text differs from a page with no lane"
    );
}
