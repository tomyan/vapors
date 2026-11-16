use super::RowOffsets;

#[test]
fn rows_are_packed_two_to_sixteen_bytes_in_order() {
    // When
    let bytes = *RowOffsets::whole(&[[0.0, 3.0], [1.0, 24.0], [0.0, 48.0]]).bytes();

    // Then
    let float = |at: usize| f32::from_le_bytes(bytes[at * 4..at * 4 + 4].try_into().unwrap());
    let floats: Vec<f32> = (0..8).map(float).collect();
    assert_eq!(floats, [0.0, 3.0, 1.0, 24.0, 0.0, 48.0, 0.0, 0.0]);
    assert_eq!(bytes.len(), 1024);
}

#[test]
fn a_full_table_is_kept() {
    let offsets = vec![[0.0, 1.0]; RowOffsets::CAPACITY];
    assert_eq!(
        RowOffsets::whole(&offsets).bytes()[1020..],
        1.0f32.to_le_bytes()
    );
}

#[test]
#[should_panic(expected = "129 row offsets for one batch, over the table's 128")]
fn one_row_over_the_table_is_refused() {
    let _ = RowOffsets::whole(&vec![[0.0, 0.0]; RowOffsets::CAPACITY + 1]);
}

#[test]
#[should_panic(expected = "row 1's offset of [0.0, 2.5] device pixels is not whole")]
fn an_offset_that_is_not_whole_is_refused() {
    let _ = RowOffsets::whole(&[[0.0, 0.0], [0.0, 2.5]]);
}

#[test]
fn the_empty_tables_hash_is_its_bytes_hash() {
    let none = RowOffsets::NONE;
    assert_eq!(none.hash(), crate::word_hash::word_hash(none.bytes()));
    assert_eq!(RowOffsets::whole(&[]).hash(), none.hash());
}

/// Two quads, for the row indexes given with them.
fn two_quads() -> [vapors_scene::Quad; 2] {
    let quad = vapors_scene::Quad::fill(
        vapors_scene::Rect::new(0.0, 0.0, 1.0, 1.0),
        vapors_scene::Color::from_rgba8(1, 2, 3, 255),
    );
    [quad; 2]
}

#[test]
fn every_item_given_a_row_inside_the_table_is_written() {
    let bytes = crate::QuadPipeline::bytes_in_rows(&two_quads(), &[0, 127]);
    assert_eq!(bytes[144 + 128..144 + 132], 127u32.to_le_bytes());
}

#[test]
#[should_panic(expected = "quads: item 1 names row 128, past the 128 rows a batch's table holds")]
fn a_row_past_the_table_is_refused_naming_the_item_and_the_row() {
    let _ = crate::QuadPipeline::bytes_in_rows(&two_quads(), &[0, 128]);
}

#[test]
#[should_panic(expected = "shadows: 1 row indexes for 2 items")]
fn fewer_rows_than_items_are_refused() {
    let shadow = vapors_scene::Shadow {
        rect: vapors_scene::Rect::new(0.0, 0.0, 2.0, 2.0),
        radii: [0.0; 4],
        blur: 1.0,
        color: vapors_scene::Color::from_rgba8(0, 0, 0, 255),
        caster: vapors_scene::Rect::new(0.0, 0.0, 2.0, 2.0),
        caster_radii: [0.0; 4],
    };
    let _ = crate::ShadowPipeline::bytes_in_rows(&[shadow, shadow], &[0]);
}

#[test]
#[should_panic(expected = "names its table's length as ROW_PAIRS once")]
fn a_shader_without_the_table_length_token_is_refused() {
    let _ = RowOffsets::shader("struct View { rows: array<vec4<f32>, 64> }");
}
