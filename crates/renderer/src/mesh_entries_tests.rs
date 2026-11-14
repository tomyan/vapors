//! A mesh entry's bytes: each field of each vertex at its
//! `offset_of!`, a vertex `size_of::<MeshVertex>()` long, padded to 16.

use std::mem::offset_of;

use vapors_shapes::MeshVertex;

use super::{VERTEX_BYTES, vertex_bytes};

fn float_at(bytes: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

#[test]
fn each_field_is_at_its_offset_in_a_vertex_of_its_size() {
    // Given: one triangle whose third vertex's values are all distinct
    let blank = MeshVertex {
        position: [0.0; 2],
        normal: [0.0; 2],
        edges: [0.0; 3],
        shift: [0.0; 3],
    };
    let third = MeshVertex {
        position: [1.0, 2.0],
        normal: [3.0, 4.0],
        edges: [5.0, 6.0, 7.0],
        shift: [8.0, 9.0, 10.0],
    };

    // When
    let bytes = vertex_bytes(&[[blank, blank, third]]);

    // Then
    let start = 2 * size_of::<MeshVertex>();
    assert_eq!(VERTEX_BYTES, size_of::<MeshVertex>() as u64);
    assert_eq!(
        bytes.len(),
        (3 * size_of::<MeshVertex>()).next_multiple_of(16)
    );
    assert_eq!(
        float_at(&bytes, start + offset_of!(MeshVertex, position) + 4),
        2.0
    );
    assert_eq!(
        float_at(&bytes, start + offset_of!(MeshVertex, normal)),
        3.0
    );
    assert_eq!(
        float_at(&bytes, start + offset_of!(MeshVertex, edges) + 8),
        7.0
    );
    assert_eq!(
        float_at(&bytes, start + offset_of!(MeshVertex, shift) + 8),
        10.0
    );
}
