use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_scene::Rect;
use vapors_text::FontSystem;

use super::*;

/// A key the page made for its one batch.
fn a_key() -> BatchKey {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", "div { height: 2px; background-color: #f00; }")
        .expect("parses");
    let div = doc.create_element("div");
    doc.append(doc.body(), div);
    let view = Viewport {
        width: 10.0,
        height: 10.0,
        scale: 1.0,
    };
    let painted = doc.update(view).expect("updates");
    painted.kept_batches(Rect::new(0.0, 0.0, 10.0, 10.0)).keys[0]
}

#[test]
fn a_batch_drawn_at_the_same_offsets_keeps_its_table() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    let kept = tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    let again = tables.table(key, &[[0.0, 4.0]]);

    // Then
    assert_eq!(again, kept);
    assert_eq!(again, RowOffsets::whole(&[[0.0, 4.0]]));
}

#[test]
fn a_batch_drawn_at_other_offsets_gets_a_table_made_from_them() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    let moved = tables.table(key, &[[0.0, 8.0]]);

    // Then
    assert_eq!(moved, RowOffsets::whole(&[[0.0, 8.0]]));
}

#[test]
fn a_batch_not_drawn_in_a_frame_has_its_table_dropped() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    tables.release_undrawn();

    // Then
    assert!(tables.tables.is_empty());
}

#[test]
fn a_batch_drawn_again_keeps_its_table_past_the_frames_end() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);
    tables.release_undrawn();

    // Then
    assert_eq!(tables.tables.len(), 1);
}
