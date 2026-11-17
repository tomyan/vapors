//! The golden pictures `gpu_world_tests.rs` compares the scene with:
//! the Track renderer's pictures of the same race and cameras, recorded
//! on the UHD 630 (ANV) at 2f8669e6 (main after #484), before R3a2 deleted the
//! Track (`tests/golden/*.png`, one-scene 2.2).

use vapors_app::SurfaceSize;

/// The golden picture `name`'s RGBA, which must be `size`.
///
/// # Panics
/// When `name` has no golden, or its PNG is not 8-bit RGBA of `size`,
/// naming the file.
pub fn golden(name: &str, size: SurfaceSize) -> Vec<u8> {
    let bytes: &[u8] = match name {
        "fixed" => include_bytes!("../tests/golden/fixed.png"),
        "following" => include_bytes!("../tests/golden/following.png"),
        "following-portrait" => include_bytes!("../tests/golden/following-portrait.png"),
        "close" => include_bytes!("../tests/golden/close.png"),
        other => panic!("no golden picture {other}: add tests/golden/{other}.png"),
    };
    let file = format!("crates/racer/tests/golden/{name}.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap_or_else(|error| panic!("{file}: {error}"));
    let mut pixels = vec![0; reader.output_buffer_size().expect("a small image")];
    let info = reader
        .next_frame(&mut pixels)
        .unwrap_or_else(|error| panic!("{file}: {error}"));
    let shape = (info.width, info.height, info.color_type, info.bit_depth);
    let wanted = (
        size.width,
        size.height,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
    );
    assert_eq!(
        shape, wanted,
        "{file} is not the 8-bit RGBA picture the check draws: record it again"
    );
    pixels.truncate(info.buffer_size());
    pixels
}
