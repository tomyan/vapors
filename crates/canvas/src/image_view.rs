//! One image in a viewport, with the transform the user is steering:
//! scroll or pinch zooms about the pointer or the fingers, a drag or a
//! one-finger pan moves it, a double tap fits it or shows it actual size.

use vapors_platform::{Event, Frame};
use vapors_renderer::{ImagePipeline, clear_view, upload_rgba8};
use vapors_scene::{Color, Size, Transform};

/// Factor per scroll step or per `+`/`-` press.
const ZOOM_STEP: f32 = 1.25;

/// Tightly packed 8-bit RGBA pixels with their size.
pub struct RgbaImage {
    /// Width and height in pixels.
    pub size: Size,
    /// `size.width * size.height * 4` bytes, rows from the top.
    pub rgba: Vec<u8>,
}

struct Uploaded {
    pipeline: ImagePipeline,
    texture: wgpu::Texture,
}

/// An image placed in a viewport under a user-steered transform.
pub struct ImageView {
    image: RgbaImage,
    uploaded: Option<Uploaded>,
    viewport: Size,
    transform: Transform,
    pointer: [f32; 2],
    dragging: bool,
    background: Color,
}

impl ImageView {
    /// Show `image` on `background`. The transform is set on the first
    /// resize, which the platform sends before the first frame.
    #[must_use]
    pub fn new(image: RgbaImage, background: Color) -> Self {
        let viewport = Size {
            width: 1,
            height: 1,
        };
        Self {
            transform: Transform::fit(image.size, viewport),
            image,
            uploaded: None,
            viewport,
            pointer: [0.0, 0.0],
            dragging: false,
            background,
        }
    }

    /// The current placement of the image in the viewport.
    #[must_use]
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Fit the image to the viewport, centred.
    pub fn fit(&mut self) {
        self.transform = Transform::fit(self.image.size, self.viewport);
    }

    /// One image pixel per viewport pixel, centred.
    pub fn actual_size(&mut self) {
        self.transform = Transform::actual_size(self.image.size, self.viewport);
    }

    /// Multiply the scale by `factor`, keeping the image under `anchor` still.
    pub fn zoom(&mut self, anchor: [f32; 2], factor: f32) {
        self.transform = self.transform.zoomed_about(anchor, factor);
    }

    /// Move the image by `dx`, `dy` viewport pixels.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.transform.translate[0] += dx;
        self.transform.translate[1] += dy;
    }

    /// Fit the image unless it is fitted already, then show it actual size.
    pub fn toggle_fit(&mut self) {
        if self.transform == Transform::fit(self.image.size, self.viewport) {
            self.actual_size();
        } else {
            self.fit();
        }
    }

    /// Answer an input event.
    pub fn event(&mut self, event: &Event) {
        match event {
            Event::Resized { width, height } => {
                self.viewport = Size {
                    width: *width,
                    height: *height,
                };
                self.fit();
            }
            Event::PointerMoved { x, y } => self.pointer_moved([*x, *y]),
            Event::PrimaryButton { pressed } => self.dragging = *pressed,
            Event::Scroll { steps } => self.zoom(self.pointer, ZOOM_STEP.powf(*steps)),
            Event::Pinch { x, y, factor } => self.zoom([*x, *y], *factor),
            Event::Pan { dx, dy } => self.pan(*dx, *dy),
            Event::DoubleTap { .. } => self.toggle_fit(),
            Event::Character(text) => self.key(text),
            Event::Touch { .. } | Event::Tap { .. } | Event::FileOpened { .. } => {}
        }
    }

    fn pointer_moved(&mut self, to: [f32; 2]) {
        if self.dragging {
            self.pan(to[0] - self.pointer[0], to[1] - self.pointer[1]);
        }
        self.pointer = to;
    }

    fn key(&mut self, text: &str) {
        match text {
            "0" => self.fit(),
            "1" => self.actual_size(),
            "+" | "=" => self.zoom(self.centre(), ZOOM_STEP),
            "-" => self.zoom(self.centre(), 1.0 / ZOOM_STEP),
            _ => {}
        }
    }

    #[allow(clippy::cast_precision_loss)] // viewport sizes are far below 2^24
    fn centre(&self) -> [f32; 2] {
        [
            self.viewport.width as f32 / 2.0,
            self.viewport.height as f32 / 2.0,
        ]
    }

    /// Clear the frame to the background and draw the image.
    pub fn draw(&mut self, frame: &Frame<'_>) {
        clear_frame(frame, self.background);
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

fn clear_frame(frame: &Frame<'_>, color: Color) {
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    clear_view(frame.device, frame.queue, &view, color);
}

#[cfg(test)]
#[path = "image_view_tests.rs"]
mod tests;
