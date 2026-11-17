//! The mesh pipeline's item bytes (`meshes.rs` `bytes_in`): the item's
//! floats, then its row and its frame's entry, padded to 16 bytes.

use vapors_scene::Color;

use crate::{MeshPipeline, MeshShape};

fn shape(x: f32) -> MeshShape {
    let red = Color::from_rgba8(255, 0, 0, 255);
    MeshShape::new([1.0, 0.0, 0.0, 1.0, x, 0.0], red)
        .expect("drawable")
        .expect("has area")
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

#[test]
fn each_mesh_item_carries_its_row_then_its_frame_after_its_floats() {
    // Given
    let shapes = [shape(1.0), shape(2.0)];

    // When
    let bytes = MeshPipeline::bytes_in(&shapes, 7, &[3, 5]);

    // Then
    let one = MeshPipeline::INSTANCE_BYTES;
    assert_eq!((one, bytes.len()), (64, 128));
    assert_eq!((word(&bytes, 48), word(&bytes, 52)), (3, 7));
    assert_eq!((word(&bytes, one + 48), word(&bytes, one + 52)), (5, 7));
    assert_eq!(
        bytes[one..one + 48],
        crate::instanced::float_bytes(shapes[1].instance())
    );
}

#[test]
fn mesh_items_with_no_rows_are_each_in_row_zero_of_the_page() {
    // When
    let bytes = MeshPipeline::bytes(&[shape(1.0)]);

    // Then
    assert_eq!((word(&bytes, 48), word(&bytes, 52)), (0, 0));
    assert_eq!(bytes[56..64], [0; 8]);
}

#[test]
#[should_panic(expected = "meshes: 1 row indexes for 2 items")]
fn mesh_items_with_a_row_missing_are_refused() {
    let _ = MeshPipeline::bytes_in(&[shape(1.0), shape(2.0)], 0, &[4]);
}
