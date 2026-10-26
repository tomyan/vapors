//! The seam where an app draws the image itself
//! (docs/design/ARCHITECTURE.md, "Seams"): the view keeps the size, the
//! transform and the gestures, and on each frame hands the painter the
//! cleared target, the viewport and the transform it would have drawn
//! pixels with. touché paints a photo's visible region from its own
//! textures this way, never uploading its pixels to the view.

use vapors_platform::Frame;
use vapors_scene::{Size, Transform};

use crate::{ImageView, RgbaImage};

/// Draws an image whose pixels the app keeps.
pub trait ImagePainter {
    /// Draw the image into `target`, which is `viewport` device pixels
    /// and already cleared to the view's background, placed by
    /// `transform`: image pixel (x, y) lands at
    /// `translate + scale * (x, y)`.
    fn paint(
        &mut self,
        frame: &Frame<'_>,
        target: &wgpu::TextureView,
        viewport: Size,
        transform: Transform,
    );
}

/// What the view shows: pixels it uploads and draws, or an image the
/// app paints (`image_view_painter.rs`).
pub(crate) enum Shown {
    Pixels(RgbaImage),
    Painted(Painted),
}

impl Shown {
    pub(crate) fn size(&self) -> Size {
        match self {
            Self::Pixels(image) => image.size,
            Self::Painted(painted) => painted.size,
        }
    }
}

/// What a painter is told to draw.
pub(crate) struct Painted {
    pub(crate) size: Size,
    pub(crate) painter: Box<dyn ImagePainter>,
}

impl ImageView {
    /// Show an image of `size` that `painter` draws, fitted; replaces
    /// whatever was shown, as [`ImageView::set_image`] does.
    pub fn set_painter(&mut self, size: Size, painter: Box<dyn ImagePainter>) {
        self.replace(Shown::Painted(Painted { size, painter }));
    }

    /// Hand the painter, if one is shown, this frame's target.
    pub(crate) fn paint(&mut self, frame: &Frame<'_>, target: &wgpu::TextureView) -> bool {
        let (viewport, transform) = (self.viewport, self.transform());
        let Some(Shown::Painted(painted)) = self.shown.as_mut() else {
            return false;
        };
        painted.painter.paint(frame, target, viewport, transform);
        true
    }
}

#[cfg(test)]
#[path = "image_view_painter_tests.rs"]
mod tests;
