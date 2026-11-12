use vapors_text::FontSystem;

use super::*;

#[test]
fn an_empty_box_tree_lays_out_nothing() {
    // Given: no boxes, as html `display: none` builds
    let mut fonts = FontSystem::bundle_only();
    let mut cache = TextCache::default();
    let calcs = Calcs::default();

    // When
    let result = lay_out(
        Placed::default(),
        (&mut fonts, &mut cache),
        1.0,
        (200.0, 100.0),
        &calcs,
        Vec::new(),
    );

    // Then
    let (placed, work) = result.unwrap_or_else(|(error, _)| panic!("refused: {error}"));
    assert!(placed.boxes.is_empty());
    assert_eq!((work.laid_out, work.calls), (0, 0));
}
