//! Shelf packing on plain sizes, no fonts.

use super::*;

#[test]
fn allocations_run_along_a_shelf_then_start_the_next() {
    // Given: a 10 px page, glyphs 4x4 need 5x5 with padding
    let mut page = AtlasPage::new(10, 1);

    // When
    let first = page.allocate(4, 4);
    let second = page.allocate(4, 4);
    let third = page.allocate(4, 4);
    let fifth = {
        page.allocate(4, 4);
        page.allocate(4, 4)
    };

    // Then
    assert_eq!(first, Some([0, 0]));
    assert_eq!(second, Some([5, 0]));
    assert_eq!(third, Some([0, 5]));
    assert_eq!(fifth, None, "page is full");
}

/// Shelf packing is order dependent: the reviewer's shape. Four 4x4
/// and two 4x1 glyphs (5x5 and 5x2 with padding) in a 13 px page need
/// three 5-high shelves when interleaved (15 px) and two 5-high plus
/// one 2-high when tallest first (12 px).
#[test]
fn tallest_first_fits_where_interleaved_order_does_not() {
    // Given
    let interleaved = [(4, 1), (4, 4), (4, 1), (4, 4), (4, 4), (4, 4)];
    let tallest_first = [(4, 4), (4, 4), (4, 4), (4, 4), (4, 1), (4, 1)];

    // When
    let mut page = AtlasPage::new(13, 1);
    let in_order: Vec<_> = interleaved
        .iter()
        .map(|&(w, h)| page.allocate(w, h))
        .collect();
    page.clear();
    let sorted: Vec<_> = tallest_first
        .iter()
        .map(|&(w, h)| page.allocate(w, h))
        .collect();

    // Then
    assert!(in_order.iter().any(Option::is_none), "{in_order:?}");
    assert!(sorted.iter().all(Option::is_some), "{sorted:?}");
}

#[test]
fn a_glyph_larger_than_the_page_is_refused() {
    // Given
    let mut page = AtlasPage::new(8, 1);

    // When / Then
    assert!(!page.fits(8, 2));
    assert_eq!(page.allocate(8, 2), None);
    assert_eq!(page.allocate(2, 8), None);
    assert!(page.fits(7, 7));
}

#[test]
fn blit_writes_rows_at_the_origin_and_marks_the_page_dirty() {
    // Given
    let mut page = AtlasPage::new(4, 1);
    page.mark_clean();

    // When
    page.blit([1, 2], 2, 2, &[1, 2, 3, 4]);

    // Then
    assert!(page.dirty());
    assert_eq!(&page.pixels()[8..12], &[0, 1, 2, 0]);
    assert_eq!(&page.pixels()[12..16], &[0, 3, 4, 0]);
}

#[test]
fn a_new_page_is_dirty_over_its_whole_area() {
    // Given / When
    let page = AtlasPage::new(4, 1);

    // Then
    assert_eq!(page.dirty_rect(), Some([0, 0, 4, 4]));
}

#[test]
fn a_blit_after_upload_dirties_only_its_rectangle() {
    // Given
    let mut page = AtlasPage::new(8, 1);
    page.mark_clean();

    // When
    page.blit([1, 2], 2, 3, &[1; 6]);

    // Then
    assert_eq!(page.dirty_rect(), Some([1, 2, 2, 3]));
}

#[test]
fn blits_between_uploads_dirty_the_rectangle_bounding_them() {
    // Given
    let mut page = AtlasPage::new(8, 1);
    page.mark_clean();

    // When
    page.blit([5, 0], 2, 2, &[1; 4]);
    page.blit([0, 3], 3, 1, &[1; 3]);

    // Then
    assert_eq!(page.dirty_rect(), Some([0, 0, 7, 4]));
}

#[test]
fn clearing_dirties_the_whole_page_and_mark_clean_dirties_nothing() {
    // Given
    let mut page = AtlasPage::new(8, 1);
    page.mark_clean();
    page.blit([1, 1], 1, 1, &[1]);

    // When
    page.clear();
    let cleared = page.dirty_rect();
    page.mark_clean();

    // Then
    assert_eq!(cleared, Some([0, 0, 8, 8]));
    assert_eq!(page.dirty_rect(), None);
    assert!(!page.dirty());
}

#[test]
fn region_pixels_are_the_rectangles_rows_packed_tight() {
    // Given: an RGBA page, two pixels written at (1, 1) and (1, 2)
    let mut page = AtlasPage::new(4, 4);
    page.blit([1, 1], 1, 2, &[1, 2, 3, 4, 5, 6, 7, 8]);

    // When
    let region = page.region_pixels([1, 1, 2, 2]);

    // Then
    assert_eq!(region, [1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8, 0, 0, 0, 0]);
}
