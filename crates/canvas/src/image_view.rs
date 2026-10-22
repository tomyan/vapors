//! One image in a viewport, with the transform the user is steering.

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
            Event::PointerMoved { x, y } => self.pointer = [*x, *y],
            Event::Scroll { steps } => self.zoom(self.pointer, ZOOM_STEP.powf(*steps)),
            Event::Character(text) => self.key(text),
            _ => {}
        }
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
mod tests {
    use super::*;

    fn view() -> ImageView {
        let image = RgbaImage {
            size: Size {
                width: 200,
                height: 100,
            },
            rgba: vec![0; 200 * 100 * 4],
        };
        let mut view = ImageView::new(image, Color::from_rgba8(40, 40, 40, 255));
        view.event(&Event::Resized {
            width: 400,
            height: 400,
        });
        view
    }

    #[test]
    fn resize_fits_the_image() {
        // Given / When
        let view = view();

        // Then
        assert_eq!(view.transform().scale, 2.0);
        assert_eq!(view.transform().translate, [0.0, 100.0]);
    }

    #[test]
    fn scroll_zooms_about_the_pointer() {
        // Given
        let mut view = view();
        view.event(&Event::PointerMoved { x: 100.0, y: 200.0 });
        let under_pointer = view.transform().image_point([100.0, 200.0]);

        // When
        view.event(&Event::Scroll { steps: 1.0 });

        // Then
        assert!((view.transform().scale - 2.5).abs() < 1e-5);
        let after = view.transform().image_point([100.0, 200.0]);
        assert!((after[0] - under_pointer[0]).abs() < 1e-4);
        assert!((after[1] - under_pointer[1]).abs() < 1e-4);
    }

    #[test]
    fn keys_fit_actual_size_and_zoom() {
        // Given
        let mut view = view();

        // When / Then
        view.event(&Event::Character("1".into()));
        assert_eq!(view.transform().scale, 1.0);
        view.event(&Event::Character("+".into()));
        assert!((view.transform().scale - 1.25).abs() < 1e-5);
        view.event(&Event::Character("0".into()));
        assert_eq!(view.transform().scale, 2.0);
    }
}
