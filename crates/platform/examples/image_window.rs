//! Open a PNG and show it fitted to the window. Scroll to zoom about the
//! pointer; `0` fits, `1` is actual size, `+` and `-` zoom about the
//! centre. `--screenshot out.png` saves the second frame and exits.

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vapors_canvas::{ImageView, RgbaImage};
use vapors_platform::{App, Event, Frame, Next, WindowOptions, run};
use vapors_scene::{Color, Size};

const BACKGROUND: Color = Color {
    r: 40.0 / 255.0,
    g: 40.0 / 255.0,
    b: 40.0 / 255.0,
    a: 1.0,
};

struct ImageWindow {
    view: ImageView,
    screenshot: Option<PathBuf>,
}

impl App for ImageWindow {
    fn event(&mut self, event: Event) {
        self.view.event(&event);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        self.view.draw(frame);
        support::after_first_frame(frame, self.screenshot.as_deref())
    }
}

fn main() -> ExitCode {
    let Some(path) = support::flag_value("--open") else {
        eprintln!("usage: image_window --open <file.png> [--screenshot out.png]");
        return ExitCode::FAILURE;
    };
    let image = match decode_png(Path::new(&path)) {
        Ok(image) => image,
        Err(message) => {
            eprintln!("image_window: {message}");
            return ExitCode::FAILURE;
        }
    };
    let app = ImageWindow {
        view: ImageView::new(image, BACKGROUND),
        screenshot: support::flag_value("--screenshot").map(PathBuf::from),
    };
    let options = WindowOptions {
        title: format!("vapors: {path}"),
        width: 640,
        height: 400,
    };
    support::exit_code(run(options, app), "image_window")
}

/// Decode an 8-bit RGB or RGBA PNG to RGBA. Other PNGs are refused by name.
fn decode_png(path: &Path) -> Result<RgbaImage, String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("could not open {}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{} is not a PNG this example can read: {e}", path.display()))?;
    let mut data = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut data)
        .map_err(|e| format!("could not decode {}: {e}", path.display()))?;
    let rgba = match info.color_type {
        png::ColorType::Rgba => data,
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => {
            return Err(format!(
                "{} has colour type {other:?}; need RGB or RGBA",
                path.display()
            ));
        }
    };
    Ok(RgbaImage {
        size: Size {
            width: info.width,
            height: info.height,
        },
        rgba,
    })
}
