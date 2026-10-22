//! The one-line label at the top of the image app: file name and zoom in
//! Geist 13 px on a 44 px strip, the touche design's bar.

use vapors_platform::Frame;
use vapors_renderer::SolidPipeline;
use vapors_scene::{Color, Size};
use vapors_text::{
    AtlasConfig, BlendParams, Family, FontSystem, GlyphAtlas, Layout, LayoutOptions, RenderTarget,
    Style, TextPipeline,
};

/// Height of the strip in logical pixels; scaled by the frame's scale factor.
pub const STRIP_HEIGHT: u32 = 44;
/// The strip's colour, `#171b1f`.
pub const STRIP_COLOR: Color = Color {
    r: 23.0 / 255.0,
    g: 27.0 / 255.0,
    b: 31.0 / 255.0,
    a: 1.0,
};
/// The label's ink, `#e4e8ec`.
pub const INK: Color = Color {
    r: 228.0 / 255.0,
    g: 232.0 / 255.0,
    b: 236.0 / 255.0,
    a: 1.0,
};
/// Inset of the text from the left edge, logical pixels.
const INSET: f32 = 12.0;
/// Font size in logical pixels.
const SIZE: f32 = 13.0;

/// The strip's height in device pixels at `scale_factor`.
#[must_use]
pub fn strip_height(scale_factor: f64) -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // 44 * a small positive scale
    let height = (f64::from(STRIP_HEIGHT) * scale_factor).round() as u32;
    height
}

/// `"<file name>  <zoom>%"`, `no file` before anything is opened; the
/// zoom is the viewport pixels per image pixel, as a whole percentage.
#[must_use]
pub fn label_text(file_name: Option<&str>, scale: f32) -> String {
    let name = file_name.unwrap_or("no file");
    format!("{name}  {}%", (scale * 100.0).round())
}

/// GPU resources for the strip and its text, made on the first frame.
pub struct LabelDraw {
    solid: SolidPipeline,
    text: TextPipeline,
    atlas: GlyphAtlas,
}

impl LabelDraw {
    /// Pipelines for frames of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let atlas = GlyphAtlas::new(AtlasConfig::default());
        Self {
            solid: SolidPipeline::new(device, format),
            text: TextPipeline::new(device, format, &atlas),
            atlas,
        }
    }

    /// Draw the strip across the top of `frame` with `text` on it, sized
    /// by the frame's scale factor.
    pub fn draw(&mut self, frame: &Frame<'_>, fonts: &mut FontSystem, text: &str) {
        let viewport = Size {
            width: frame.texture.width(),
            height: frame.texture.height(),
        };
        #[allow(clippy::cast_possible_truncation)] // scale factors are small
        let scale = frame.scale_factor as f32;
        let strip = px(strip_height(frame.scale_factor));
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.solid.fill(
            frame.device,
            frame.queue,
            &view,
            viewport,
            [0.0, 0.0, px(viewport.width), strip],
            STRIP_COLOR,
        );
        let options = LayoutOptions {
            scale,
            ..LayoutOptions::default()
        };
        let layout = Layout::new(fonts, text, &style(), &options);
        let origin = [INSET * scale, ((strip - layout.height()) / 2.0).round()];
        let target = RenderTarget {
            device: frame.device,
            queue: frame.queue,
            view: &view,
            viewport,
        };
        self.atlas.begin_frame();
        if let Err(error) = self.text.draw(
            &target,
            &mut self.atlas,
            &layout,
            origin,
            BlendParams::default(),
        ) {
            vapors_platform::report_error(&format!("label: {error}"));
        }
    }
}

fn style() -> Style<'static> {
    Style {
        tabular_figures: true,
        color: INK,
        ..Style::new(Family::Geist, SIZE)
    }
}

#[allow(clippy::cast_precision_loss)] // viewport sizes are far below 2^24
fn px(pixels: u32) -> f32 {
    pixels as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strip_is_44_logical_pixels_at_any_scale() {
        assert_eq!(strip_height(1.0), 44);
        assert_eq!(strip_height(2.0), 88);
        assert_eq!(strip_height(1.5), 66);
    }

    #[test]
    fn the_label_names_the_file_and_the_zoom_as_a_whole_percentage() {
        // Given
        let cases = [
            (None, 1.0, "no file  100%"),
            (Some("gradient.png"), 1.5, "gradient.png  150%"),
            (Some("a.png"), 1.0 / 3.0, "a.png  33%"),
            (Some("a.png"), 12.0, "a.png  1200%"),
        ];

        for (name, scale, expected) in cases {
            // When
            let text = label_text(name, scale);

            // Then
            assert_eq!(text, expected);
        }
    }
}
