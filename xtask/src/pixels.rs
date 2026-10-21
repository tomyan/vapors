//! Reading one pixel out of a PNG, and parsing the colour to compare it to.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Parse `r,g,b` with each channel 0..=255.
pub fn parse_rgb(text: &str) -> Result<[u8; 3], String> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(format!("expected r,g,b with three channels, got `{text}`"));
    }
    let mut rgb = [0u8; 3];
    for (slot, part) in rgb.iter_mut().zip(&parts) {
        *slot = part
            .parse()
            .map_err(|_| format!("channel `{part}` in `{text}` is not 0..=255"))?;
    }
    Ok(rgb)
}

/// The RGB value at the centre of a PNG.
pub fn centre_pixel(path: &Path) -> Result<[u8; 3], String> {
    let file = File::open(path).map_err(|e| format!("could not open {}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(BufReader::new(file))
        .read_info()
        .map_err(|e| format!("{} is not a PNG this tool can read: {e}", path.display()))?;
    let mut data = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut data)
        .map_err(|e| format!("could not decode {}: {e}", path.display()))?;
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => {
            return Err(format!(
                "{} has colour type {other:?}; need RGB or RGBA",
                path.display()
            ));
        }
    };
    let x = info.width as usize / 2;
    let y = info.height as usize / 2;
    let start = (y * info.width as usize + x) * channels;
    Ok([data[start], data[start + 1], data[start + 2]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_channels() {
        // Given
        let text = "12, 200,34";

        // When
        let rgb = parse_rgb(text).expect("valid");

        // Then
        assert_eq!(rgb, [12, 200, 34]);
    }

    #[test]
    fn refuses_a_channel_out_of_range() {
        // Given
        let text = "12,999,34";

        // When
        let error = parse_rgb(text).expect_err("999 is not a channel");

        // Then
        assert!(error.contains("999"));
    }

    #[test]
    fn reads_the_centre_pixel_of_a_png() {
        // Given: a 4x3 RGBA image, centre (2,1) painted distinctly
        let dir = std::env::temp_dir().join(format!("vapors-xtask-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("centre.png");
        let mut data = vec![0u8; 4 * 3 * 4];
        let start = (4 + 2) * 4;
        data[start..start + 4].copy_from_slice(&[12, 200, 34, 255]);
        write_rgba_png(&path, 4, 3, &data);

        // When
        let centre = centre_pixel(&path).expect("decodes");

        // Then
        assert_eq!(centre, [12, 200, 34]);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn write_rgba_png(path: &Path, width: u32, height: u32, data: &[u8]) {
        let file = File::create(path).expect("create png");
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(data).expect("data");
    }
}
