//! The golden pictures `gpu_world_tests.rs` compares the scene with:
//! the Track renderer's pictures of the same race and cameras
//! (`tests/golden/*.png`, one-scene 2.2).
//!
//! How they were made: on the UHD 630 (ANV), on R3a2's branch at
//! 2f8669e6 (main after #484, the Track still there), a throwaway test
//! in `gpu_world_tests.rs` (not committed) drew each case with R3a1's
//! `by_track` (a `Track` mounted, resized to the size and rendered into a
//! fresh texture, read back) and wrote it with `png::Encoder`, 8-bit RGBA,
//! `Compression::High`: `fixed`, `following` and `close` at 320 x 240,
//! `following-portrait` at 240 x 320, the race and cameras
//! `gpu_world_tests.rs` gives. R3a2 deleted the Track, so they cannot be
//! recorded again: a damaged one is restored from git.

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
        .unwrap_or_else(|error| panic!("{file}: {error}; {}", restore(&file)));
    let mut pixels = vec![0; reader.output_buffer_size().expect("a small image")];
    let info = reader
        .next_frame(&mut pixels)
        .unwrap_or_else(|error| panic!("{file}: {error}; {}", restore(&file)));
    let shape = (info.width, info.height, info.color_type, info.bit_depth);
    let wanted = (
        size.width,
        size.height,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
    );
    assert_eq!(
        shape,
        wanted,
        "{file} is not the 8-bit RGBA picture the check draws; {}",
        restore(&file)
    );
    pixels.truncate(info.buffer_size());
    pixels
}

/// How to restore golden `file`: it cannot be recorded again, as the
/// Track that drew it is gone (R3a2).
fn restore(file: &str) -> String {
    format!(
        "the Track that drew it is gone (R3a2), so it cannot be recorded again: restore it from the commit that added it, `git log --diff-filter=A --format=%h -- {file}`, with `git show <that commit>:{file} > {file}`"
    )
}
