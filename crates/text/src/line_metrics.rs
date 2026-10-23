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
    let bytes = match family {
        Family::Geist => GEIST,
        Family::GeistMono => GEIST_MONO,
        Family::Named(_) => return LineHeight::MetricsRelative(1.0),
    };
    let font = skrifa::FontRef::new(bytes).expect("the bundled fonts parse (fonts.rs tests)");
    let metrics = Metrics::new(&font, Size::new(size * scale), LocationRef::default());
    let device = metrics.ascent.round() + (-metrics.descent).round() + metrics.leading.round();
    LineHeight::Absolute(device / scale)
}

#[cfg(test)]
mod tests {
    use super::*;

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
