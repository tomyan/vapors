//! `line-height: normal` as Chromium computes it: the primary font's
//! ascent, descent and line gap, each rounded to a whole device pixel
//! before they are added (Blink's `SimpleFontData` rounds them; with
//! zoom-for-DSF the font is sized in device pixels). Parley's own
//! `MetricsRelative(1.0)` adds them unrounded, which made a 15 px Geist
//! line 19.5 px tall where Chromium's is 19 (the F2c layout corpus).

use parley::LineHeight;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::Metrics;

use crate::fonts::{GEIST, GEIST_MONO};
use crate::style::Family;

/// The line height of `normal` for `family` at `size` logical pixels,
/// laid out at `scale`. Only the bundled faces are read here; a family
/// by name keeps Parley's unrounded metrics, since it is resolved by
/// the font collection only during layout.
pub(crate) fn normal(family: &Family<'_>, size: f32, scale: f32) -> LineHeight {
    let Some(metrics) = device_metrics(family, size, scale) else {
        return LineHeight::MetricsRelative(1.0);
    };
    let device = metrics.ascent.round() + (-metrics.descent).round() + metrics.leading.round();
    LineHeight::Absolute(device / scale)
}

/// The content area of `family` at `size`, laid out at `scale`: its
/// ascent and descent in logical pixels, each rounded to a whole device
/// pixel as Blink rounds them. An inline element's box is this tall
/// around the baseline. `None` for a family by name, whose font only
/// the layout resolves.
#[must_use]
pub fn content_area(family: &Family<'_>, size: f32, scale: f32) -> Option<(f32, f32)> {
    let metrics = device_metrics(family, size, scale)?;
    Some((
        metrics.ascent.round() / scale,
        (-metrics.descent).round() / scale,
    ))
}

fn device_metrics(family: &Family<'_>, size: f32, scale: f32) -> Option<Metrics> {
    let bytes = match family {
        Family::Geist => GEIST,
        Family::GeistMono => GEIST_MONO,
        Family::Named(_) => return None,
    };
    let font = skrifa::FontRef::new(bytes).expect("the bundled fonts parse (fonts.rs tests)");
    Some(Metrics::new(
        &font,
        Size::new(size * scale),
        LocationRef::default(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geists_content_area_at_15px_is_15_above_and_4_below_the_baseline() {
        // When
        let area = content_area(&Family::Geist, 15.0, 1.0);

        // Then
        assert_eq!(area, Some((15.0, 4.0)));
    }

    #[test]
    fn a_named_familys_content_area_is_unknown_here() {
        // Then
        assert_eq!(content_area(&Family::Named("Inter"), 15.0, 1.0), None);
    }

    #[test]
    fn geist_at_15px_is_19px_tall_at_1x_as_in_chromium() {
        // Given: ascent 15.075 rounds to 15, descent 4.425 to 4, no gap
        let (size, scale) = (15.0, 1.0);

        // When
        let height = normal(&Family::Geist, size, scale);

        // Then
        assert_eq!(height, LineHeight::Absolute(19.0));
    }

    #[test]
    fn at_2x_the_rounding_is_in_device_pixels() {
        // Given: ascent 30.15 rounds to 30, descent 8.85 to 9
        let (size, scale) = (15.0, 2.0);

        // When
        let height = normal(&Family::Geist, size, scale);

        // Then
        assert_eq!(height, LineHeight::Absolute(19.5));
    }
}
