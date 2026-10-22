//! Copying a texture into host memory: tests assert on it, screenshots
//! and the automation API's frame capture are built on it.

/// Rows in a copied buffer start on 256-byte boundaries (wgpu's
/// `COPY_BYTES_PER_ROW_ALIGNMENT`).
const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Read an 8-bit RGBA or BGRA texture back as tightly packed RGBA bytes,
/// row by row from the top. BGRA sources are swizzled to RGBA.
///
/// # Panics
///
/// If the texture format is not one of `Rgba8Unorm`, `Rgba8UnormSrgb`,
/// `Bgra8Unorm`, `Bgra8UnormSrgb`, or the buffer cannot be mapped; both
/// are programming errors in the caller, and the message says which.
#[must_use]
pub fn read_rgba8(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let swap_red_blue = match texture.format() {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => false,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => true,
        other => panic!("read_rgba8 needs an 8-bit RGBA or BGRA texture, got {other:?}"),
    };
    let (width, height) = (texture.width(), texture.height());
    let padded_row = padded_bytes_per_row(width * 4);
    let buffer = copy_to_buffer(device, queue, texture, padded_row);
    let padded = map_and_copy(device, &buffer);
    let mut bytes = unpad_rows(&padded, width * 4, padded_row, height);
    if swap_red_blue {
        for pixel in bytes.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
    }
    bytes
}

fn padded_bytes_per_row(unpadded: u32) -> u32 {
    unpadded.div_ceil(ROW_ALIGN) * ROW_ALIGN
}

fn copy_to_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    padded_row: u32,
) -> wgpu::Buffer {
    let size = u64::from(padded_row) * u64::from(texture.height());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
    queue.submit([encoder.finish()]);
    buffer
}

fn map_and_copy(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).expect("receiver alive");
    });
    device
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
