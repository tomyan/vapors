//! Measuring an anonymous text box for Taffy: Parley lays the inline run
//! out at the width Taffy offers, at the display scale so line breaks
//! match what is drawn, and the size comes back in CSS pixels.

use taffy::{AvailableSpace, Size};
use vapors_text::{FontSystem, Layout, Shaped, Span, Style, fits};

use super::inline::InlineRun;

/// The size of `run`: its width is the widest line, its height the sum
/// of its lines.
pub(crate) fn measure_text(
    (fonts, cache): (&mut FontSystem, &mut super::TextCache),
    scale: f32,
    run: &InlineRun,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
) -> Size<f32> {
    let width = known.width.or(match available.width {
        AvailableSpace::Definite(width) => Some(width),
        AvailableSpace::MinContent => Some(0.0),
        AvailableSpace::MaxContent => None,
    });
    let layout = cache.layout(fonts, scale, run, width);
    Size {
        width: known
            .width
            .unwrap_or_else(|| measured_width(&layout, scale, width)),
        height: known.height.unwrap_or(layout.height() / scale),
    }
}

/// `layout`'s width in CSS pixels, no wider than a definite `width`
/// its lines fit (`vapors_text::fits`): a line within the fit slack of
/// the width it was offered measures that width, not up to
/// 1/(64 * scale) px more.
fn measured_width(layout: &Layout, scale: f32, width: Option<f32>) -> f32 {
    let measured = layout.width() / scale;
    match width {
        Some(width) if fits(layout.width(), width * scale) => measured.min(width),
        _ => measured,
    }
}

/// `run` laid out at `width` CSS pixels (`None` never wraps): shaped
/// as [`text_shaped`], then broken.
#[cfg(test)]
pub(crate) fn text_layout(
    fonts: &mut FontSystem,
    scale: f32,
    run: &InlineRun,
    width: Option<f32>,
) -> Layout {
    text_shaped(fonts, scale, run).into_layout(width, run.style.align)
}

/// `run` shaped and not yet broken: the block's style, each inline
/// element's over its range. Text that is not visible is shaped in a
/// transparent ink, so it takes its space.
pub(crate) fn text_shaped(fonts: &mut FontSystem, scale: f32, run: &InlineRun) -> Shaped {
    let spans: Vec<Span<'static>> = run
        .spans
        .iter()
        .map(|span| Span {
            range: span.range.clone(),
            style: inked(span.style.for_layout(), span.visible),
        })
        .collect();
    let base = inked(run.style.for_layout(), run.visible);
    // Left to right only: right-to-left text is refused (`bidi`).
    let direction = vapors_text::parley::BaseDirection::Ltr;
    Shaped::new(fonts, &run.text, &base, &spans, scale, direction)
}

fn inked(style: Style<'static>, visible: bool) -> Style<'static> {
    if visible {
        return style;
    }
    Style {
        color: vapors_scene::Color::default(),
        ..style
    }
}

#[cfg(test)]
#[path = "measure_tests.rs"]
mod tests;
