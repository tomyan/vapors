use super::rowed_vertex_bytes;
use crate::pipeline::FLOATS_PER_VERTEX;

#[test]
#[should_panic(expected = "not a whole number of")]
fn text_vertex_floats_short_of_a_whole_vertex_are_refused() {
    rowed_vertex_bytes(&[0.0; FLOATS_PER_VERTEX + 1], (&[1], (0, &[]), &[]));
}

#[test]
#[should_panic(expected = "the draws' vertices are the floats'")]
fn draws_counting_other_vertices_than_the_floats_hold_are_refused() {
    rowed_vertex_bytes(&[0.0; 2 * FLOATS_PER_VERTEX], (&[1], (0, &[]), &[]));
}

#[test]
fn each_vertex_carries_its_row_then_its_frames_entry_then_its_clips() {
    // When: one vertex in row 2 of frame entry 7, under clip entry 9
    let bytes = rowed_vertex_bytes(&[0.0; FLOATS_PER_VERTEX], (&[1], (7, &[(0, 9)]), &[2]));

    // Then
    let floats = FLOATS_PER_VERTEX * 4;
    assert_eq!(bytes[floats..floats + 4], 2u32.to_le_bytes());
    assert_eq!(bytes[floats + 4..floats + 8], 7u32.to_le_bytes());
    assert_eq!(bytes[floats + 8..floats + 12], 9u32.to_le_bytes());
}

#[test]
#[should_panic(
    expected = "text: clip runs [(1, 7)] for 2 items: the first run must start at item 0"
)]
fn clip_runs_that_leave_the_first_draw_out_are_refused() {
    rowed_vertex_bytes(
        &[0.0; 2 * FLOATS_PER_VERTEX],
        (&[1, 1], (0, &[(1, 7)]), &[]),
    );
}
