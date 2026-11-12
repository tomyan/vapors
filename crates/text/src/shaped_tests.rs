//! A paragraph shaped once and broken at each width gives the layout
//! shaping and breaking together would.

use super::*;
use crate::{Align, Family, FontSystem, Layout, LayoutOptions, Span, Style};
use parley::BaseDirection as Direction;

const TEXT: &str = "Some words to wrap, and a bold one";

fn bold() -> Span<'static> {
    Span {
        range: 26..30,
        style: Style {
            weight: 700.0,
            ..Style::new(Family::Geist, 16.0)
        },
    }
}

fn shaped(system: &mut FontSystem, scale: f32) -> Shaped {
    let base = Style::new(Family::Geist, 16.0);
    Shaped::new(system, TEXT, &base, &[bold()], scale, Direction::Ltr)
}

/// The reference: shaped and broken in one go, on a layout no other
/// width has broken, as `Layout::styled` did before `Shaped`.
fn shaped_and_broken(system: &mut FontSystem, options: &LayoutOptions) -> Layout {
    let base = Style::new(Family::Geist, 16.0);
    let mut builder = system.ranged_builder(TEXT, options.scale);
    builder.set_base_direction(options.direction);
    for property in base.properties(options.scale) {
        builder.push_default(property);
    }
    for property in bold().style.properties(options.scale) {
        builder.push(property, bold().range);
    }
    let mut inner = builder.build(TEXT);
    inner.break_all_lines(options.width.map(|w| w * options.scale));
    let align = crate::layout_runs::alignment(options.align);
    inner.align(align, parley::AlignmentOptions::default());
    Layout::broken(inner, TEXT.to_owned(), options.scale)
}

/// A layout's lines, every glyph's id, x, y and advance, and its size.
type Seen = (Vec<crate::LineInfo>, Vec<(u32, f32, f32, f32)>, f32, f32);

/// What a reader of a layout sees: its lines and every placed glyph.
fn seen(layout: &Layout) -> Seen {
    let glyphs = layout.glyphs().map(|g| (g.id, g.x, g.y, g.advance));
    let size = (layout.width(), layout.height());
    (layout.lines().collect(), glyphs.collect(), size.0, size.1)
}

#[test]
fn a_shaped_paragraph_broken_at_a_width_is_the_layout_at_that_width() {
    for (width, align, scale) in [
        (None, Align::Start, 1.0),
        (Some(0.0), Align::Start, 1.0),
        (Some(90.0), Align::Center, 1.0),
        (Some(90.0), Align::Justify, 2.0),
    ] {
        // Given
        let mut system = FontSystem::bundle_only();
        let options = LayoutOptions {
            width,
            align,
            scale,
            direction: Direction::Ltr,
        };
        let once = shaped(&mut system, scale);

        // When
        let broken = once.lay_out(width, align);

        // Then
        let expected = shaped_and_broken(&mut system, &options);
        assert_eq!(
            seen(&broken),
            seen(&expected),
            "{width:?} {align:?} {scale}"
        );
    }
}

#[test]
fn content_widths_are_the_widths_at_min_and_max_content() {
    for scale in [1.0, 1.5] {
        // Given
        let mut system = FontSystem::bundle_only();
        let once = shaped(&mut system, scale);

        // When
        let widths = once.content_widths();

        // Then
        assert_eq!(widths.min, once.lay_out(Some(0.0), Align::Start).width());
        assert_eq!(widths.max, once.lay_out(None, Align::Start).width());
    }
}
