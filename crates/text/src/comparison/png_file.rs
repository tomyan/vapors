//! PNG files in and out, for the comparison's screenshots.

use std::path::Path;

use super::measure::Rgba;

/// Read an 8-bit RGB or RGBA PNG.
///
/// # Errors
///
/// A message naming the file and what was wrong with it.
pub fn read_png(path: &Path) -> Result<Rgba, String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{} is not a PNG this can read: {e}", path.display()))?;
    let mut data = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut data)
        .map_err(|e| format!("{}: could not decode: {e}", path.display()))?;
    let data = match info.color_type {
        png::ColorType::Rgba => data,
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => {
            return Err(format!(
                "{}: colour type {other:?}; need RGB or RGBA",
                path.display()
            ));
        }
    };
    Ok(Rgba {
        width: info.width,
        height: info.height,
        data,
    })
}

/// Write `image` as an RGBA PNG.
///
/// # Errors
///
/// A message naming the file and the failure.
pub fn write_png(path: &Path, image: &Rgba) -> Result<(), String> {
    let file = std::fs::File::create(path)
        .map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writer
        .write_image_data(&image.data)
        .map_err(|e| format!("{}: {e}", path.display()))
}
