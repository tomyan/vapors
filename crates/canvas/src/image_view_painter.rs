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
///
/// - `target` is a view of the frame's own texture (`frame.texture`),
///   `viewport` device pixels, already cleared to the view's background.
/// - `transform` is in device pixels, as the viewport is (it comes from
///   `Event::Resized`): image pixel (x, y) lands at
///   `translate + scale * (x, y)`. `frame.scale_factor` (device pixels
///   per logical pixel) is already in it; a painter does not apply it
///   again.
/// - No clip is applied: the painter owns the whole viewport. The view
///   and its app draw their own overlays (`ImageApp`'s label strip)
///   after the painter, so those stay on top.
/// - The painter must not keep `frame`, `target`, the device or the
///   queue beyond the call: a later frame may come from another device.
/// - It is called from inside `ImageView::draw` with the view mutably
///   borrowed, so it cannot reach the view; an app that shares the view
///   through a `RefCell` would panic there.
pub trait ImagePainter {
    /// Draw the image into `target`, placed by `transform`.
    ///
    /// # Errors
    ///
    /// Why nothing (or not all) was drawn, naming the fix;
    /// `ImageView::draw` returns it.
    fn paint(
        &mut self,
        frame: &Frame<'_>,
        target: &wgpu::TextureView,
        viewport: Size,
        transform: Transform,
    ) -> Result<(), String>;
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

    /// Hand the painter, if one is shown, this frame's target: `None`
    /// when the view shows pixels or nothing, else the painter's result.
    pub(crate) fn paint(
        &mut self,
        frame: &Frame<'_>,
        target: &wgpu::TextureView,
    ) -> Option<Result<(), String>> {
        let (viewport, transform) = (self.viewport, self.transform());
        let Some(Shown::Painted(painted)) = self.shown.as_mut() else {
            return None;
        };
        Some(painted.painter.paint(frame, target, viewport, transform))
    }
}

#[cfg(test)]
#[path = "image_view_painter_tests.rs"]
mod tests;
