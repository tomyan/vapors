//! `check_pixels <demo> <screenshot.png> [scale]`: the demo's pixel
//! checks on a screenshot taken elsewhere, the web page's in particular.
//! Prints each failure and exits 1 when any check fails.

use std::process::ExitCode;

use vapors_demos::pixels::{Image, failures, parse};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("FAIL check_pixels: {message}");
            eprintln!(
                "     reproduce: cargo run -p vapors-demos --bin check_pixels -- <demo> <png> [scale]"
            );
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [name, png, rest @ ..] = args.as_slice() else {
        return Err("usage: check_pixels <demo> <screenshot.png> [scale]".to_owned());
    };
    let scale: f32 = rest
        .first()
        .map_or(Ok(1.0), |s| s.parse())
        .map_err(|_| "the scale is not a number".to_owned())?;
    let demo = vapors_demos::find(name)?;
    let checks = parse(
        name,
        demo.pixels
            .ok_or(format!("demo `{name}` has no .pixels file"))?,
    )?;
    let (width, height, rgba) = read_png(png)?;
    let found = failures(
        &checks,
        &Image {
            width,
            height,
            scale,
            rgba: &rgba,
        },
    );
    if found.is_empty() {
        println!("ok   {name}: {} pixel checks hold on {png}", checks.len());
        return Ok(());
    }
    Err(format!(
        "{png} fails {name}'s pixel checks:\n  {}",
        found.join("\n  ")
    ))
}

/// The PNG as RGBA8, from the 8-bit RGB or RGBA a browser writes.
fn read_png(path: &str) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| format!("{path}: {e}"))?;
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or(format!("{path}: too large"))?
    ];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| format!("{path}: {e}"))?;
    let pixels = &buffer[..info.buffer_size()];
    let rgba = match (info.color_type, info.bit_depth) {
        (png::ColorType::Rgba, png::BitDepth::Eight) => pixels.to_vec(),
        (png::ColorType::Rgb, png::BitDepth::Eight) => pixels
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        (colour, depth) => {
            return Err(format!(
                "{path}: {colour:?} at {depth:?} bits; wanted 8-bit RGB or RGBA"
            ));
        }
    };
    Ok((info.width, info.height, rgba))
}
