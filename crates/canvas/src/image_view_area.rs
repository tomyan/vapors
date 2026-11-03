//! The part of the frame the image is placed in: the frame less its
//! insets, in device pixels, for an app that draws over part of its
//! window (touché's side panel). Fitting, actual size and the keys' and
//! wheel's default anchor use the area; the frame keeps its full size
//! for drawing, so pixels outside the area are drawn as the transform
//! puts them, and the app's own drawing covers them. A pointer gesture
//! that starts outside the area is not the view's (`image_view_input.rs`).
//! A frame narrower or shorter than its insets has an empty area and
//! draws only the background; [`ImageView::area_is_empty`] tells the app,
//! which passes smaller insets when its frame is too narrow for its
//! panel.

use vapors_scene::{Rect, Size, Transform};

use super::ImageView;

/// Device pixels kept clear of the image's fit on each side of the frame.
/// The app converts its panel's CSS size by the scale factor and sets the
/// insets again when the scale factor changes; the view does not.
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
    /// Insets equal to the view's change nothing, so a host may set them
    /// on every layout without losing the person's zoom and pan.
    pub fn set_insets(&mut self, insets: Insets) {
        if insets == self.insets {
            return;
        }
        self.insets = insets;
        self.fit();
    }

    /// The insets set.
    #[must_use]
    pub fn insets(&self) -> Insets {
        self.insets
    }

    /// The area in the frame, in device pixels.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // see `placed`
    pub fn area_rect(&self) -> Rect {
        let area = self.area_size();
        let (left, top) = (self.insets.left as f32, self.insets.top as f32);
        Rect::new(
            left,
            top,
            left + area.width as f32,
            top + area.height as f32,
        )
    }

    /// Whether `point` (device pixels) is in the area.
    #[must_use]
    pub fn area_contains(&self, point: [f32; 2]) -> bool {
        let area = self.area_rect();
        (area.left..area.right).contains(&point[0]) && (area.top..area.bottom).contains(&point[1])
    }

    /// A new frame size: fitted again, unless the view is kept and has a
    /// placement worth keeping (an area it was sized to, at a finite,
    /// positive scale; a 0x0 resize before layout gives neither).
    pub(crate) fn resized(&mut self, width: u32, height: u32) {
        self.viewport = Size { width, height };
        let scale = self.transform().scale;
        let keep = self.keep_view && self.sized && scale.is_finite() && scale > 0.0;
        self.sized |= !self.area_is_empty();
        if !keep {
            self.fit();
        }
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

    /// Whether the area has no pixels, so only the background is drawn:
    /// the frame is no wider or taller than its insets.
    #[must_use]
    pub fn area_is_empty(&self) -> bool {
        let area = self.area_size();
        area.width == 0 || area.height == 0
    }

    /// `transform`, worked out in the area, moved to the area's place in
    /// the frame.
    // f32 holds every whole number to 2^24 exactly; an inset beyond that
    // (larger than any frame) is rounded, and only empties the area
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn placed(&self, transform: Transform) -> Transform {
        let [x, y] = transform.translate;
        Transform {
            scale: transform.scale,
            translate: [x + self.insets.left as f32, y + self.insets.top as f32],
        }
    }

    /// The area's centre, in frame pixels.
    #[allow(clippy::cast_precision_loss)] // see `placed`
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
