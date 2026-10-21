//! Shared by the examples: argument parsing, clearing, screenshots, exit codes.

#![allow(dead_code)] // each example uses a subset

use std::path::Path;
use std::process::ExitCode;

use vapors_gpu_harness::read_rgba8;
use vapors_platform::{Frame, Next, PlatformError};
use vapors_scene::Color;

/// The value after `flag` on the command line, if present.
pub fn flag_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }
    }
    None
}

/// Clear the frame's texture to `color`.
pub fn clear(frame: &Frame<'_>, color: Color) {
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    vapors_renderer::clear_view(frame.device, frame.queue, &view, color);
}

/// Report the first frame; on the second, save the screenshot if asked and exit.
pub fn after_first_frame(frame: &Frame<'_>, screenshot: Option<&Path>) -> Next {
    if frame.index == 0 {
        println!("first frame presented");
    }
    match screenshot {
        Some(path) if frame.index >= 1 => {
            save(frame, path);
            Next::Exit
        }
        _ => Next::Continue,
    }
}

/// Write the frame's texture as a PNG and print its centre pixel and adapter.
pub fn save(frame: &Frame<'_>, path: &Path) {
    let pixels = read_rgba8(frame.device, frame.queue, frame.texture);
    let (width, height) = (frame.texture.width(), frame.texture.height());
    let file = std::fs::File::create(path).expect("create screenshot file");
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(&pixels).expect("png data");
    let centre = ((height / 2) * width + width / 2) as usize * 4;
    println!(
        "screenshot written {} {width}x{height} centre={},{},{} adapter={}",
        path.display(),
        pixels[centre],
        pixels[centre + 1],
        pixels[centre + 2],
        frame.adapter.get_info().name
    );
}

/// Zero on success; otherwise print the error and return one.
pub fn exit_code(result: Result<(), PlatformError>, name: &str) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{name} failed: {error}");
            ExitCode::FAILURE
        }
    }
}
