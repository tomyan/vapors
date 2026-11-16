//! A mesh entry's bytes: each field of each corner at its
//! `offset_of!`, a triangle `size_of::<MeshTriangle>()` long, padded to
//! 16.

use std::mem::offset_of;

use vapors_shapes::{INSIDE, MeshCorner, MeshTriangle};

use super::{TRIANGLE_BYTES, corner_offset, triangle_bytes};

fn float_at(bytes: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

#[test]
fn each_field_is_at_its_offset_in_a_triangle_of_its_size() {
    // Given: two triangles, the second's third corner's values distinct
    let blank = MeshCorner {
        position: [0.0; 2],
        normals: [[0.0; 2]; 2],
        kind: INSIDE,
        reaches: [0.0; 2],
    };
    let third = MeshCorner {
        position: [1.0, 2.0],
        normals: [[3.0, 4.0], [5.0, 6.0]],
        kind: 7.0,
        reaches: [8.0, 9.0],
    };
    let triangles = [
        MeshTriangle {
            corners: [blank; 3],
        },
        MeshTriangle {
            corners: [blank, blank, third],
        },
    ];

    // When
    let bytes = triangle_bytes(&triangles);

    // Then
    let start = corner_offset(1, 2);
    assert_eq!(
        start,
        size_of::<MeshTriangle>() + 2 * size_of::<MeshCorner>()
    );
    assert_eq!(TRIANGLE_BYTES, size_of::<MeshTriangle>() as u64);
    assert_eq!(
        bytes.len(),
        (2 * size_of::<MeshTriangle>()).next_multiple_of(16)
    );
    let at = |field: usize, k: usize| float_at(&bytes, start + field + 4 * k);
    assert_eq!(at(offset_of!(MeshCorner, position), 1), 2.0);
    assert_eq!(at(offset_of!(MeshCorner, normals), 0), 3.0);
    assert_eq!(at(offset_of!(MeshCorner, normals), 3), 6.0);
    assert_eq!(at(offset_of!(MeshCorner, kind), 0), 7.0);
    assert_eq!(at(offset_of!(MeshCorner, reaches), 1), 9.0);
}
