//! A layout whose lines this cannot place as Blink does is its own
//! state: the layout names why, and its glyphs keep Parley's baselines.

use crate::{Family, FontSystem, Layout, LayoutOptions, LineHeight, Span, Style};

fn style(family: Family<'static>, size: f32, line: f32) -> Style<'static> {
    let mut style = Style::new(family, size);
    style.line_height = LineHeight::Px(line);
    style
}

#[test]
fn a_span_of_another_size_and_line_height_is_refused_naming_it() {
    // Given: 10 px text in 30 px lines, with a 20 px span in 20 px lines
    let base = style(Family::Geist, 10.0, 30.0);
    let spans = [Span {
        range: 4..7,
        style: style(Family::Geist, 20.0, 20.0),
    }];

    // When
    let options = LayoutOptions::default();
    let layout = Layout::styled(
        &mut FontSystem::bundle_only(),
        "Row big",
        &base,
        &spans,
        &options,
    );

    // Then
    let refusal = layout.baseline_refusal().expect("a mixed line is refused");
    assert!(
        refusal.contains("4..7") && refusal.contains("20 px"),
        "{refusal}"
    );
}

#[test]
fn a_span_differing_only_in_weight_and_face_is_placed() {
    // Given: Geist Mono has Geist's vertical metrics
    let base = style(Family::Geist, 10.0, 12.0);
    let mut mono = style(Family::GeistMono, 10.0, 12.0);
    mono.weight = 700.0;
    let spans = [Span {
        range: 0..3,
        style: mono,
    }];

    // When
    let options = LayoutOptions::default();
    let layout = Layout::styled(
        &mut FontSystem::bundle_only(),
        "Row 0",
        &base,
        &spans,
        &options,
    );

    // Then
    assert_eq!(layout.baseline_refusal(), None);
    assert_eq!(layout.lines().next().expect("a line").baseline, 9.0);
}

#[test]
fn a_named_family_is_refused_saying_its_primary_font_is_unknown_here() {
    // Given
    let base = style(Family::Named("Inter"), 10.0, 12.0);

    // When
    let options = LayoutOptions::default();
    let layout = Layout::new(&mut FontSystem::bundle_only(), "Row 0", &base, &options);

    // Then
    let refusal = layout.baseline_refusal().expect("refused");
    assert!(
        refusal.contains("Inter") && refusal.contains("primary font"),
        "{refusal}"
    );
}
