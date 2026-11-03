//! One image in a viewport, with the transform the user is steering:
//! scroll or pinch zooms about the pointer or the fingers, a drag or a
//! one-finger pan moves it, a double tap fits it or shows it actual size.

use vapors_renderer::ImagePipeline;
use vapors_scene::{Color, Size, Transform};

use crate::image_view_area::Insets;
use crate::image_view_input::Steering;
use crate::image_view_painter::Shown;

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
    /// The gesture in progress (`image_view_input.rs`).
    pub(crate) steering: Steering,
    pub(crate) background: Color,
    /// Kept clear of the image's fit on each side (`image_view_area.rs`).
    pub(crate) insets: Insets,
    /// Bumped by every `set_image` and `set_painter`: two images of the same size, fitted
    /// the same, differ only in this.
    generation: u64,
    /// Whether a resize keeps the zoom and position (`keep_view_on_resize`).
    pub(crate) keep_view: bool,
    /// Whether a resize has given the view its size yet.
    pub(crate) sized: bool,
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
            steering: Steering::default(),
            background,
            insets: Insets::default(),
            generation: 0,
            keep_view: false,
            sized: false,
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

    /// Fit the image to the viewport less its insets, centred.
    pub fn fit(&mut self) {
        if let Some(fitted) = self.fitted() {
            self.transform = fitted;
        }
    }

    /// One image pixel per viewport pixel, centred in the viewport less
    /// its insets.
    pub fn actual_size(&mut self) {
        if let Some(size) = self.image_size() {
            self.transform = self.placed(Transform::actual_size(size, self.area_size()));
        }
    }

    fn fitted(&self) -> Option<Transform> {
        let size = self.image_size()?;
        Some(self.placed(Transform::fit(size, self.area_size())))
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

    /// Keep the zoom and position across a resize (`keep`), as while a
    /// phone's keyboard shortens the window under a panel the person is
    /// typing into, or fit the image again on each resize (the default).
    /// The first resize, which gives the view its size, always fits.
    pub fn keep_view_on_resize(&mut self, keep: bool) {
        self.keep_view = keep;
    }

    /// Fit the image unless it is fitted already, then show it actual size.
    pub fn toggle_fit(&mut self) {
        if self.fitted() == Some(self.transform) {
            self.actual_size();
        } else {
            self.fit();
        }
    }
}

#[cfg(test)]
#[path = "image_view_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "image_view_pointer_tests.rs"]
mod pointer_tests;

#[cfg(test)]
#[path = "image_view_resize_tests.rs"]
mod resize_tests;
