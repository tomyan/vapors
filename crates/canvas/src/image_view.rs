//! One image in a viewport, with the transform the user is steering:
//! scroll or pinch zooms about the pointer or the fingers, a drag or a
//! one-finger pan moves it, a double tap fits it or shows it actual size.

use vapors_platform::{Event, LINE_PIXELS};
use vapors_renderer::ImagePipeline;
use vapors_scene::{Color, Size, Transform};

use crate::image_view_painter::Shown;

/// Factor per scroll step or per `+`/`-` press.
const ZOOM_STEP: f32 = 1.25;

pub use vapors_scene::RgbaImage;

/// The image on the GPU, made on the first frame that needs it.
pub(crate) struct Uploaded {
    pub(crate) pipeline: ImagePipeline,
    pub(crate) texture: wgpu::Texture,
}

/// An image placed in a viewport under a user-steered transform.
pub struct ImageView {
    pub(crate) shown: Option<Shown>,
    pub(crate) uploaded: Option<Uploaded>,
    pub(crate) viewport: Size,
    transform: Transform,
    /// The last pointer position; `None` until the pointer first moves.
    pointer: Option<[f32; 2]>,
    dragging: bool,
    pub(crate) background: Color,
    /// Bumped by every `set_image` and `set_painter`: two images of the same size, fitted
    /// the same, differ only in this.
    generation: u64,
}

impl ImageView {
    /// Show `image` on `background`. The transform is set on the first
    /// resize, which the platform sends before the first frame.
    #[must_use]
    pub fn new(image: RgbaImage, background: Color) -> Self {
        let mut view = Self::empty(background);
        view.set_image(image);
        view
    }

    /// Only `background`, until [`ImageView::set_image`].
    #[must_use]
    pub fn empty(background: Color) -> Self {
        Self {
            shown: None,
            uploaded: None,
            viewport: Size {
                width: 1,
                height: 1,
            },
            transform: Transform {
                scale: 1.0,
                translate: [0.0, 0.0],
            },
            pointer: None,
            dragging: false,
            background,
            generation: 0,
        }
    }

    /// Replace what is shown with `image`, fitted.
    pub fn set_image(&mut self, image: RgbaImage) {
        self.replace(Shown::Pixels(image));
    }

    pub(crate) fn replace(&mut self, shown: Shown) {
        self.generation += 1;
        self.shown = Some(shown);
        self.uploaded = None;
        self.fit();
    }

    /// How many images have been set: it changes whenever the pixels
    /// shown are replaced, even by an image of the same size.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// True once an image has been set.
    #[must_use]
    pub fn has_image(&self) -> bool {
        self.shown.is_some()
    }

    fn image_size(&self) -> Option<Size> {
        self.shown.as_ref().map(Shown::size)
    }

    /// The current placement of the image in the viewport.
    #[must_use]
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Fit the image to the viewport, centred.
    pub fn fit(&mut self) {
        if let Some(size) = self.image_size() {
            self.transform = Transform::fit(size, self.viewport);
        }
    }

    /// One image pixel per viewport pixel, centred.
    pub fn actual_size(&mut self) {
        if let Some(size) = self.image_size() {
            self.transform = Transform::actual_size(size, self.viewport);
        }
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
        let fitted = self
            .image_size()
            .map(|size| Transform::fit(size, self.viewport));
        if fitted == Some(self.transform) {
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
            Event::PointerLeft => self.pointer = None,
            Event::PrimaryButton { pressed } => self.dragging = *pressed,
            // a notch away from the user (one line up) zooms in a step
            Event::Scroll { dy, .. } => {
                self.zoom(self.scroll_anchor(), ZOOM_STEP.powf(-dy / LINE_PIXELS));
            }
            Event::Pinch { x, y, factor } => self.zoom([*x, *y], *factor),
            Event::Pan { dx, dy } => self.pan(*dx, *dy),
            Event::DoubleTap { .. } => self.toggle_fit(),
            Event::Character(text) => self.key(text),
            Event::Touch { .. }
            | Event::Key { .. }
            | Event::Tap { .. }
            | Event::OpenRequested
            | Event::Paste
            | Event::FileOpened { .. } => {}
        }
    }

    fn pointer_moved(&mut self, to: [f32; 2]) {
        if let (true, Some(from)) = (self.dragging, self.pointer) {
            self.pan(to[0] - from[0], to[1] - from[1]);
        }
        self.pointer = Some(to);
    }

    /// The pointer, or the view centre when no pointer position is known:
    /// winit's wheel event carries no position of its own.
    fn scroll_anchor(&self) -> [f32; 2] {
        self.pointer.unwrap_or_else(|| self.centre())
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
}

#[cfg(test)]
#[path = "image_view_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "image_view_pointer_tests.rs"]
mod pointer_tests;
