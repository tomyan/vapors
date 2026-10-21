//! Placing an image in a viewport: uniform scale and translation, in
//! pixels. `viewport = image * scale + translate`.

/// Uniform scale and translation from image pixels to viewport pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Viewport pixels per image pixel.
    pub scale: f32,
    /// Viewport position of the image's top-left corner.
    pub translate: [f32; 2],
}

/// Width and height in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Transform {
    /// Scale the image to fit inside the viewport, centred, never upscaled
    /// past what fits and never cropped.
    #[must_use]
    pub fn fit(image: Size, viewport: Size) -> Self {
        let scale = (as_f32(viewport.width) / as_f32(image.width))
            .min(as_f32(viewport.height) / as_f32(image.height));
        Self::centred(image, viewport, scale)
    }

    /// One image pixel per viewport pixel, centred.
    #[must_use]
    pub fn actual_size(image: Size, viewport: Size) -> Self {
        Self::centred(image, viewport, 1.0)
    }

    fn centred(image: Size, viewport: Size, scale: f32) -> Self {
        let translate = [
            (as_f32(viewport.width) - as_f32(image.width) * scale) / 2.0,
            (as_f32(viewport.height) - as_f32(image.height) * scale) / 2.0,
        ];
        Self { scale, translate }
    }

    /// Multiply the scale by `factor`, keeping the image point under the
    /// viewport point `anchor` where it is.
    #[must_use]
    pub fn zoomed_about(self, anchor: [f32; 2], factor: f32) -> Self {
        let scale = self.scale * factor;
        let translate = [
            anchor[0] - (anchor[0] - self.translate[0]) * factor,
            anchor[1] - (anchor[1] - self.translate[1]) * factor,
        ];
        Self { scale, translate }
    }

    /// Viewport-pixel corners of the image: top-left and bottom-right.
    #[must_use]
    pub fn image_bounds(self, image: Size) -> [[f32; 2]; 2] {
        let [x, y] = self.translate;
        [
            [x, y],
            [
                x + as_f32(image.width) * self.scale,
                y + as_f32(image.height) * self.scale,
            ],
        ]
    }

    /// The image pixel under a viewport point.
    #[must_use]
    pub fn image_point(self, viewport_point: [f32; 2]) -> [f32; 2] {
        [
            (viewport_point[0] - self.translate[0]) / self.scale,
            (viewport_point[1] - self.translate[1]) / self.scale,
        ]
    }
}

#[allow(clippy::cast_precision_loss)] // pixel counts are far below 2^24
fn as_f32(pixels: u32) -> f32 {
    pixels as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: Size = Size {
        width: 200,
        height: 100,
    };
    const VIEW: Size = Size {
        width: 400,
        height: 400,
    };

    #[test]
    fn fit_scales_to_the_tighter_axis_and_centres_the_other() {
        // Given: a 2:1 image in a square viewport

        // When
        let transform = Transform::fit(IMAGE, VIEW);

        // Then: width-limited, scale 2, vertically centred
        assert_eq!(transform.scale, 2.0);
        assert_eq!(transform.translate, [0.0, 100.0]);
    }

    #[test]
    fn actual_size_centres_at_scale_one() {
        // When
        let transform = Transform::actual_size(IMAGE, VIEW);

        // Then
        assert_eq!(transform.scale, 1.0);
        assert_eq!(transform.translate, [100.0, 150.0]);
    }

    #[test]
    fn zooming_about_a_point_keeps_that_image_point_under_it() {
        // Given
        let transform = Transform::fit(IMAGE, VIEW);
        let anchor = [50.0, 150.0];
        let before = transform.image_point(anchor);

        // When
        let zoomed = transform.zoomed_about(anchor, 3.0);

        // Then
        assert_eq!(zoomed.scale, 6.0);
        assert_eq!(zoomed.image_point(anchor), before);
    }

    #[test]
    fn bounds_follow_scale_and_translation() {
        // Given
        let transform = Transform {
            scale: 2.0,
            translate: [10.0, 20.0],
        };

        // When
        let bounds = transform.image_bounds(IMAGE);

        // Then
        assert_eq!(bounds, [[10.0, 20.0], [410.0, 220.0]]);
    }
}
