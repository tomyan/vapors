use super::rowed_vertex_bytes;
use crate::pipeline::FLOATS_PER_VERTEX;

#[test]
#[should_panic(expected = "not a whole number of")]
fn text_vertex_floats_short_of_a_whole_vertex_are_refused() {
    rowed_vertex_bytes(&[0.0; FLOATS_PER_VERTEX + 1], (&[1], &[]));
}

#[test]
#[should_panic(expected = "the draws' vertices are the floats'")]
fn draws_counting_other_vertices_than_the_floats_hold_are_refused() {
    rowed_vertex_bytes(&[0.0; 2 * FLOATS_PER_VERTEX], (&[1], &[]));
}
