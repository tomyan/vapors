//! Decoding the PNGs the examples and the web demo open. Other formats
//! come with the image pipeline later; a PNG this cannot read is refused
//! by its colour type.

use vapors_scene::Size;

use crate::RgbaImage;

/// Decode an 8-bit RGB or RGBA PNG to RGBA.
///
/// # Errors
///
/// A message saying why `bytes` is not a PNG this can show: not a PNG,
/// truncated, or a colour type other than RGB and RGBA.
pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("not a PNG this can read: {e}"))?;
    let mut data = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut data)
        .map_err(|e| format!("could not decode: {e}"))?;
    let rgba = match info.color_type {
        png::ColorType::Rgba => data,
        png::ColorType::Rgb => rgb_to_rgba(&data),
        other => return Err(format!("colour type {other:?}; need RGB or RGBA")),
    };
    Ok(RgbaImage {
        size: Size {
            width: info.width,
            height: info.height,
        },
        rgba,
    })
}

fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(color: png::ColorType, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(data).expect("data");
        drop(writer);
        bytes
    }

    #[test]
    fn rgb_gains_an_opaque_alpha() {
        // Given
        let bytes = png_bytes(png::ColorType::Rgb, 2, 1, &[1, 2, 3, 4, 5, 6]);

        // When
        let image = decode_png(&bytes).expect("decodes");

        // Then
        assert_eq!(
            image.size,
            Size {
                width: 2,
                height: 1
            }
        );
        assert_eq!(image.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn rgba_passes_through() {
        // Given
        let bytes = png_bytes(png::ColorType::Rgba, 1, 1, &[9, 8, 7, 6]);

        // When
        let image = decode_png(&bytes).expect("decodes");

        // Then
        assert_eq!(image.rgba, [9, 8, 7, 6]);
    }

    #[test]
    fn not_a_png_is_refused_with_a_reason() {
        // Given
        let bytes = b"GIF89a";

        // When
        let error = decode_png(bytes).expect_err("not a PNG");

        // Then
        assert!(error.starts_with("not a PNG"), "{error}");
    }
}
