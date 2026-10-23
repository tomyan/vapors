//! Measuring an anonymous text box for Taffy: Parley lays the inline run
//! out at the width Taffy offers, at the display scale so line breaks
//! match what is drawn, and the size comes back in CSS pixels.

use taffy::{AvailableSpace, Size};
use vapors_text::{FontSystem, Layout, LayoutOptions, Span, Style};

use super::inline::InlineRun;

/// The size of `run`: its width is the widest line, its height the sum
/// of its lines.
pub(crate) fn measure_text(
    fonts: &mut FontSystem,
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
    let layout = text_layout(fonts, scale, run, width);
    Size {
        width: known.width.unwrap_or(layout.width() / scale),
        height: known.height.unwrap_or(layout.height() / scale),
    }
}

/// `run` laid out at `width` CSS pixels (`None` never wraps): the
/// block's style, each inline element's over its range. Text that is
/// not visible is laid out in a transparent ink, so it takes its space.
pub(crate) fn text_layout(
    fonts: &mut FontSystem,
    scale: f32,
    run: &InlineRun,
    width: Option<f32>,
) -> Layout {
    let options = LayoutOptions {
        width,
        scale,
        align: run.style.align,
    };
    let spans: Vec<Span<'static>> = run
        .spans
        .iter()
        .map(|span| Span {
            range: span.range.clone(),
            style: inked(span.style.for_layout(), span.visible),
        })
        .collect();
    let base = inked(run.style.for_layout(), run.visible);
    Layout::styled(fonts, &run.text, &base, &spans, &options)
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
