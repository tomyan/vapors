use super::*;

#[test]
fn the_vertex_attributes_fill_exactly_the_vertex_size_the_scene_checks() {
    // Given
    let last = VERTEX_ATTRIBUTES.last().expect("attributes");

    // When
    let filled = last.offset + last.format.size();

    // Then
    assert_eq!(
        filled, VERTEX_BYTES as u64,
        "the pipeline's vertex attributes end at {filled} bytes, the scene's VERTEX_BYTES is {VERTEX_BYTES}: change both together (vapors-scene3d mesh_error.rs and pipeline.rs)"
    );
}
