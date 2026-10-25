//! `check_pixels <file.pixels> <screenshot.png> [scale]`: a demo's
//! pixel checks on a screenshot taken elsewhere, the web page's in
//! particular. Prints each failure and exits 1 when any check fails.
//! The section checked is the one for the screenshot's size divided by
//! the scale. `check_pixels --inputs <file.pixels> [WxH]` prints the
//! inputs of the `WxH` section (640x400 without one), one `click X Y` or
//! `wheel X Y DX DY` per line, for the page-contract script to make
//! before its screenshot.
//! `check_pixels --viewports <file.pixels>` prints each section's size,
//! one `W H` per line, for the script to take a screenshot at each.

use std::process::ExitCode;

use vapors_pixels::{DEFAULT_VIEWPORT, Image, Spec, failures, parse, section};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("FAIL check_pixels: {message}");
            eprintln!(
                "     reproduce: cargo run -p vapors-pixels --bin check_pixels -- <file.pixels> <png> [scale]"
            );
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, pixels] if flag == "--viewports" => return print_viewports(pixels),
        [flag, pixels] if flag == "--inputs" => return print_inputs(pixels, DEFAULT_VIEWPORT),
        [flag, pixels, size] if flag == "--inputs" => return print_inputs(pixels, viewport(size)?),
        _ => {}
    }
    let [pixels, png, rest @ ..] = args.as_slice() else {
        return Err("usage: check_pixels <file.pixels> <screenshot.png> [scale]".to_owned());
    };
    let scale: f32 = rest
        .first()
        .map_or(Ok(1.0), |s| s.parse())
        .map_err(|_| "the scale is not a number".to_owned())?;
    let text = std::fs::read_to_string(pixels).map_err(|e| format!("{pixels}: {e}"))?;
    let name = pixels.trim_end_matches(".pixels");
    let (width, height, rgba) = read_png(png)?;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let viewport = [
        (width as f32 / scale) as u32,
        (height as f32 / scale) as u32,
    ];
    let checks = section(&parse(name, &text)?, viewport)?.checks.clone();
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

fn specs(pixels: &str) -> Result<Vec<Spec>, String> {
    let text = std::fs::read_to_string(pixels).map_err(|e| format!("{pixels}: {e}"))?;
    parse(pixels.trim_end_matches(".pixels"), &text)
}

fn print_inputs(pixels: &str, viewport: [u32; 2]) -> Result<(), String> {
    for input in &section(&specs(pixels)?, viewport)?.inputs {
        println!("{}", input.line());
    }
    Ok(())
}

fn print_viewports(pixels: &str) -> Result<(), String> {
    for spec in specs(pixels)? {
        let [width, height] = spec.viewport;
        println!("{width} {height}");
    }
    Ok(())
}

/// `WxH` as a section's size, refused naming the shape wanted.
fn viewport(size: &str) -> Result<[u32; 2], String> {
    let parsed = size
        .split_once('x')
        .and_then(|(w, h)| Some([w.parse().ok()?, h.parse().ok()?]));
    parsed.ok_or(format!(
        "`{size}` is not a size; give one as WxH, such as 390x800"
    ))
}
