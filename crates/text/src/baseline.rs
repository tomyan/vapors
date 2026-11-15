//! Where a layout's lines put their baselines (#368): as Blink places
//! them for a line of one font size and line height, by the primary
//! font's ascent and descent (`baseline_below_top`), whatever fallback
//! fonts the line also uses, as Blink takes only the primary font's
//! metrics for a fixed line height. A layout this cannot place so (a span
//! of another size, line height or vertical metrics; a family by name,
//! whose primary font only the layout resolves) is its own state,
//! `Unresolvable`: its glyphs keep Parley's baselines and the layout
//! names why (`Layout::baseline_refusal`), never a guess.

use vapors_scene::Color;

use crate::line_metrics::{baseline_below_top, device_metrics};
use crate::style::{Span, Style};

/// What a layout's lines put their baselines by.
#[derive(Clone, Debug, PartialEq)]
pub enum BaselineRule {
    /// The primary font's metrics, device pixels, unrounded.
    Primary {
        /// Its ascent.
        ascent: f32,
        /// Its descent, positive below the baseline.
        descent: f32,
    },
    /// Lines this cannot place as Blink does; why.
    Unresolvable(String),
}

/// The rule for text in `base` with `spans`, laid out at `scale`.
#[must_use]
pub fn baseline_rule(base: &Style<'_>, spans: &[Span<'_>], scale: f32) -> BaselineRule {
    let Some(metrics) = device_metrics(&base.family, base.size, scale) else {
        return BaselineRule::Unresolvable(format!(
            "the family {:?}'s primary font is resolved only by the layout, so its metrics are not known here",
            base.family
        ));
    };
    let (ascent, descent) = (metrics.ascent, -metrics.descent);
    let primary = (ascent.round(), descent.round());
    if let Some(span) = spans
        .iter()
        .find(|span| !alike(base, &span.style, scale, primary))
    {
        return BaselineRule::Unresolvable(format!(
            "bytes {:?} are {} px with line height {:?} where the text is {} px with {:?}: Blink takes each inline box's own half-leading, which is not placed here (elements refuses such a line: inline_check.rs)",
            span.range, span.style.size, span.style.line_height, base.size, base.line_height
        ));
    }
    BaselineRule::Primary { ascent, descent }
}

/// `style`'s ascent and descent rounded to device pixels, when known.
fn rounded(style: &Style<'_>, scale: f32) -> Option<(f32, f32)> {
    let metrics = device_metrics(&style.family, style.size, scale)?;
    Some((metrics.ascent.round(), (-metrics.descent).round()))
}

/// Whether `span` puts its text on the same line box as `base`'s: the
/// same specified size (bit for bit) and line height, and the same
/// rounded vertical metrics.
fn alike(base: &Style<'_>, span: &Style<'_>, scale: f32, primary: (f32, f32)) -> bool {
    span.size.to_bits() == base.size.to_bits()
        && span.line_height == base.line_height
        && rounded(span, scale) == Some(primary)
}

/// How far `rule` moves `line`'s baseline from Parley's, in the line's
/// own units, which are `to_device` device pixels each (1 for a layout
/// in device pixels, the scale for an editor's in CSS pixels); `None`
/// when the rule is unresolvable.
#[must_use]
pub fn line_shift(
    line: &parley::Line<'_, Color>,
    rule: &BaselineRule,
    to_device: f32,
) -> Option<f32> {
    let BaselineRule::Primary { ascent, descent } = rule else {
        return None;
    };
    let metrics = line.metrics();
    let top = metrics.block_min_coord * to_device;
    let placed = top + baseline_below_top(metrics.line_height * to_device, *ascent, *descent);
    Some((placed - metrics.baseline * to_device) / to_device)
}

/// How far the baseline of the line at `y` (the editor layout's units,
/// `scale` device pixels each) moves under `rule`: what a field's caret,
/// selection and composing rects move by to stay with its glyphs: 0 with
/// no line, `None` when the rule is unresolvable.
#[must_use]
pub fn editor_line_shift(
    layout: &parley::Layout<Color>,
    y: f32,
    rule: &BaselineRule,
    scale: f32,
) -> Option<f32> {
    let line = layout
        .lines()
        .find(|line| y < line.metrics().block_max_coord)
        .or_else(|| layout.lines().last());
    match line {
        Some(line) => line_shift(&line, rule, scale),
        None => Some(0.0),
    }
}

#[cfg(test)]
#[path = "baseline_tests.rs"]
mod tests;
