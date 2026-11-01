//! The part of the frame the image is placed in: the frame less its
//! insets, in device pixels, for an app that draws over part of its
//! window (touché's side panel). Fitting, actual size and the keys' and
//! wheel's default anchor use the area; the frame keeps its full size
//! for drawing, so pixels outside the area are drawn as the transform
//! puts them, and the app's own drawing covers them. A frame narrower or
//! shorter than its insets has an empty area and shows nothing, as a
//! minimised window does.

use vapors_scene::{Size, Transform};

use super::ImageView;

/// Device pixels kept clear of the image's fit on each side of the frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Insets {
    /// From the left edge.
    pub left: u32,
    /// From the top edge.
    pub top: u32,
    /// From the right edge.
    pub right: u32,
    /// From the bottom edge.
    pub bottom: u32,
}

impl ImageView {
    /// Place the image in the frame less `insets`, and fit it there.
    pub fn set_insets(&mut self, insets: Insets) {
        self.insets = insets;
        self.fit();
    }

    /// The area's size: the frame less its insets, zero where they meet.
    #[must_use]
    pub fn area_size(&self) -> Size {
        let i = self.insets;
        Size {
            width: self
                .viewport
                .width
                .saturating_sub(i.left.saturating_add(i.right)),
            height: self
                .viewport
                .height
                .saturating_sub(i.top.saturating_add(i.bottom)),
        }
    }

    /// Whether the area has no pixels, so nothing is drawn.
    pub(crate) fn area_is_empty(&self) -> bool {
        let area = self.area_size();
        area.width == 0 || area.height == 0
    }

    /// `transform`, worked out in the area, moved to the area's place in
    /// the frame.
    #[allow(clippy::cast_precision_loss)] // insets are far below 2^24
    pub(crate) fn placed(&self, transform: Transform) -> Transform {
        let [x, y] = transform.translate;
        Transform {
            scale: transform.scale,
            translate: [x + self.insets.left as f32, y + self.insets.top as f32],
        }
    }

    /// The area's centre, in frame pixels.
    #[allow(clippy::cast_precision_loss)] // sizes are far below 2^24
    pub(crate) fn centre(&self) -> [f32; 2] {
        let area = self.area_size();
        [
            self.insets.left as f32 + area.width as f32 / 2.0,
            self.insets.top as f32 + area.height as f32 / 2.0,
        ]
    }
}

#[cfg(test)]
#[path = "image_view_area_tests.rs"]
mod tests;
