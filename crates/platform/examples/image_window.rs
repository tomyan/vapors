//! Open a PNG and show it fitted to the window. Scroll to zoom about the
//! pointer; `0` fits, `1` is actual size, `+` and `-` zoom about the
//! centre. `--screenshot out.png` saves the second frame and exits.

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vapors_platform::{App, Event, Frame, Next, WindowOptions, run};
use vapors_renderer::{ImagePipeline, upload_rgba8};
use vapors_scene::{Color, Size, Transform};

const BACKGROUND: Color = Color {
    r: 40.0 / 255.0,
    g: 40.0 / 255.0,
    b: 40.0 / 255.0,
    a: 1.0,
};
const ZOOM_STEP: f32 = 1.25;

struct Decoded {
    size: Size,
    rgba: Vec<u8>,
}

struct Uploaded {
    pipeline: ImagePipeline,
    texture: wgpu::Texture,
}

struct ImageWindow {
    image: Decoded,
    uploaded: Option<Uploaded>,
    viewport: Size,
    transform: Transform,
    pointer: [f32; 2],
    screenshot: Option<PathBuf>,
}

impl ImageWindow {
    fn fit(&mut self) {
        self.transform = Transform::fit(self.image.size, self.viewport);
    }

    fn zoom(&mut self, anchor: [f32; 2], factor: f32) {
        self.transform = self.transform.zoomed_about(anchor, factor);
    }

    #[allow(clippy::cast_precision_loss)] // viewport sizes are far below 2^24
    fn centre(&self) -> [f32; 2] {
        [
            self.viewport.width as f32 / 2.0,
            self.viewport.height as f32 / 2.0,
        ]
    }

    fn upload(&mut self, frame: &Frame<'_>) -> &Uploaded {
        self.uploaded.get_or_insert_with(|| Uploaded {
            pipeline: ImagePipeline::new(frame.device, frame.texture.format()),
            texture: upload_rgba8(
                frame.device,
                frame.queue,
                self.image.size.width,
                self.image.size.height,
                &self.image.rgba,
            ),
        })
    }
}

impl App for ImageWindow {
    fn event(&mut self, event: Event) {
        match event {
            Event::Resized { width, height } => {
                self.viewport = Size { width, height };
                self.fit();
            }
            Event::PointerMoved { x, y } => self.pointer = [x, y],
            Event::Scroll { steps } => self.zoom(self.pointer, ZOOM_STEP.powf(steps)),
            Event::Character(text) => match text.as_str() {
                "0" => self.fit(),
                "1" => self.transform = Transform::actual_size(self.image.size, self.viewport),
                "+" | "=" => self.zoom(self.centre(), ZOOM_STEP),
                "-" => self.zoom(self.centre(), 1.0 / ZOOM_STEP),
                _ => {}
            },
            Event::PrimaryButton { .. } => {}
        }
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        support::clear(frame, BACKGROUND);
        let (viewport, transform) = (self.viewport, self.transform);
        let uploaded = self.upload(frame);
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        uploaded.pipeline.draw(
            frame.device,
            frame.queue,
            &view,
            viewport,
            &uploaded.texture,
            transform,
        );
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
        image,
        uploaded: None,
        viewport: Size {
            width: 1,
            height: 1,
        },
        transform: Transform {
            scale: 1.0,
            translate: [0.0, 0.0],
        },
        pointer: [0.0, 0.0],
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
fn decode_png(path: &Path) -> Result<Decoded, String> {
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
    Ok(Decoded {
        size: Size {
            width: info.width,
            height: info.height,
        },
        rgba,
    })
}
