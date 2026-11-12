//! A paragraph shaped and not yet broken into lines: Parley's
//! itemising, font selection and shaping, done once, then broken at
//! each width asked for. Breaking a clone of the shaped paragraph gives
//! the layout shaping and breaking together would (`shaped_tests.rs`).

use parley::{AlignmentOptions, BaseDirection, ContentWidths};
use vapors_scene::Color;

use crate::font_system::FontSystem;
use crate::layout::Layout;
use crate::layout_runs::alignment;
use crate::layout_types::Align;
use crate::style::{Span, Style};

/// `text` shaped in its styles at a scale, in device pixels.
#[derive(Clone)]
pub struct Shaped {
    inner: parley::Layout<Color>,
    text: String,
    scale: f32,
}

impl Shaped {
    /// Shapes `text` in `base`, with `spans` overriding ranges of it,
    /// at `scale` device pixels per logical pixel.
    #[must_use]
    pub fn new(
        system: &mut FontSystem,
        text: &str,
        base: &Style<'_>,
        spans: &[Span<'_>],
        scale: f32,
        direction: BaseDirection,
    ) -> Self {
        let mut builder = system.ranged_builder(text, scale);
        builder.set_base_direction(direction);
        for property in base.properties(scale) {
            builder.push_default(property);
        }
        for span in spans {
            for property in span.style.properties(scale) {
                builder.push(property, span.range.clone());
            }
        }
        Self {
            inner: builder.build(text),
            text: text.to_owned(),
            scale,
        }
    }

    /// The widths with every soft break taken (`min`) and none (`max`),
    /// in device pixels, read from the shaped clusters with no breaking.
    #[must_use]
    pub fn content_widths(&self) -> ContentWidths {
        self.inner.calculate_content_widths()
    }

    /// Breaks a copy into lines at `width` logical pixels (`None` never
    /// wraps) and aligns them; this paragraph is left unbroken.
    #[must_use]
    pub fn lay_out(&self, width: Option<f32>, align: Align) -> Layout {
        self.clone().into_layout(width, align)
    }

    /// Breaks this paragraph into lines at `width` and aligns them.
    #[must_use]
    pub fn into_layout(mut self, width: Option<f32>, align: Align) -> Layout {
        self.inner.break_all_lines(width.map(|w| w * self.scale));
        self.inner
            .align(alignment(align), AlignmentOptions::default());
        Layout::broken(self.inner, self.text, self.scale)
    }

    /// The text shaped.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Device pixels per logical pixel this was shaped at.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }
}

#[cfg(test)]
#[path = "shaped_tests.rs"]
mod tests;
