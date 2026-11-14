//! Which children's leaves read the height bases.

use taffy::{Dimension, LengthPercentage, LengthPercentageAuto, Position, Size, Style};

use super::reads_heights;

fn sized(height: Dimension) -> Style {
    Style {
        size: Size {
            width: Dimension::auto(),
            height,
        },
        ..Style::default()
    }
}

fn relative(top: LengthPercentageAuto, bottom: LengthPercentageAuto) -> Style {
    let mut style = Style {
        position: Position::Relative,
        ..Style::default()
    };
    style.inset.top = top;
    style.inset.bottom = bottom;
    style
}

#[test]
fn a_percentage_in_any_height_or_a_relative_vertical_inset_reads_the_heights() {
    let mut min = Style::default();
    min.min_size.height = LengthPercentageAuto::percent(0.5);
    let mut max = Style::default();
    max.max_size.height = LengthPercentageAuto::percent(0.5);
    let pct = LengthPercentageAuto::percent(0.1);
    let cases = [
        ("height", sized(Dimension::percent(0.5))),
        ("min-height", min),
        ("max-height", max),
        ("top", relative(pct, LengthPercentageAuto::auto())),
        ("bottom", relative(LengthPercentageAuto::auto(), pct)),
    ];
    for (name, style) in cases {
        assert!(reads_heights(&style), "{name}");
    }
}

#[test]
fn lengths_widths_and_a_static_inset_do_not() {
    let mut width = Style::default();
    width.size.width = Dimension::percent(0.5);
    let mut fixed = relative(
        LengthPercentageAuto::length(4.0),
        LengthPercentageAuto::auto(),
    );
    fixed.padding.top = LengthPercentage::percent(0.1);
    let mut static_inset = Style::default();
    static_inset.inset.top = LengthPercentageAuto::percent(0.1);
    let cases = [
        ("height in px", sized(Dimension::length(20.0))),
        ("width %", width),
        ("relative top in px", fixed),
        ("static top %", static_inset),
    ];
    for (name, style) in cases {
        assert!(!reads_heights(&style), "{name}");
    }
}
