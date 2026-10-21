//! Copying a texture into host memory so a test can assert on pixels.

use crate::Gpu;

/// Rows in a copied buffer start on 256-byte boundaries (wgpu's
/// `COPY_BYTES_PER_ROW_ALIGNMENT`).
const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Read an `Rgba8Unorm` texture back as tightly packed RGBA bytes, row
/// by row from the top.
///
/// # Panics
///
/// If the texture is not `Rgba8Unorm` or the buffer cannot be mapped;
/// both are programming errors in the test, and the message says which.
#[must_use]
pub fn read_rgba8(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    assert_eq!(
        texture.format(),
        wgpu::TextureFormat::Rgba8Unorm,
        "read_rgba8 needs an Rgba8Unorm texture"
    );
    let (width, height) = (texture.width(), texture.height());
    let padded_row = padded_bytes_per_row(width * 4);
    let buffer = copy_to_buffer(gpu, texture, padded_row);
    let padded = map_and_copy(gpu, &buffer);
    unpad_rows(&padded, width * 4, padded_row, height)
}

fn padded_bytes_per_row(unpadded: u32) -> u32 {
    unpadded.div_ceil(ROW_ALIGN) * ROW_ALIGN
}

fn copy_to_buffer(gpu: &Gpu, texture: &wgpu::Texture, padded_row: u32) -> wgpu::Buffer {
    let size = u64::from(padded_row) * u64::from(texture.height());
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("readback"),
        });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    buffer
}

fn map_and_copy(gpu: &Gpu, buffer: &wgpu::Buffer) -> Vec<u8> {
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).expect("receiver alive");
    });
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("device poll while waiting for readback");
    receiver
        .recv()
        .expect("map_async callback")
        .expect("readback buffer map");
    let bytes = slice
        .get_mapped_range()
        .expect("mapped range after a successful map")
        .to_vec();
    buffer.unmap();
    bytes
}

fn unpad_rows(padded: &[u8], row: u32, padded_row: u32, rows: u32) -> Vec<u8> {
    let (row, padded_row) = (row as usize, padded_row as usize);
    let mut out = Vec::with_capacity(row * rows as usize);
    for y in 0..rows as usize {
        out.extend_from_slice(&padded[y * padded_row..y * padded_row + row]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_padded_to_the_alignment() {
        // Given
        let unpadded = 4 * 3; // three RGBA pixels

        // When
        let padded = padded_bytes_per_row(unpadded);

        // Then
        assert_eq!(padded, ROW_ALIGN);
    }

    #[test]
    fn unpadding_drops_the_bytes_after_each_row() {
        // Given
        let padded = [1, 2, 0, 0, 3, 4, 0, 0];

        // When
        let tight = unpad_rows(&padded, 2, 4, 2);

        // Then
        assert_eq!(tight, [1, 2, 3, 4]);
    }
}
