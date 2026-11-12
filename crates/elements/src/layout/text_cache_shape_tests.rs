//! Shaping kept by content: a run is shaped once per text, styles,
//! visibility and scale, whichever node holds it, and broken at each
//! width asked for.

use vapors_dom::NodeId;
use vapors_text::LineHeight;

use super::super::inline::InlineSpan;
use super::tests::run;
use super::*;
use crate::style::Face;

/// "Edit words" with a span over "Edit" held by `node`.
fn spanned(node: u64) -> InlineRun {
    let base = run("Edit words", Align::Start);
    let span = InlineSpan {
        node: NodeId(node),
        range: 0..4,
        visible: true,
        style: base.style,
    };
    InlineRun {
        spans: vec![span],
        ..base
    }
}

/// How many times `runs` were shaped, each asked for at `width`.
fn shaped(runs: &[InlineRun], width: Option<f32>, scale: f32) -> usize {
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    cache.begin(0);
    for run in runs {
        cache.layout(&mut fonts, scale, run, width);
    }
    cache.shaped()
}

#[test]
fn equal_runs_held_by_two_nodes_are_shaped_once() {
    // Given
    let runs = [spanned(1), spanned(2)];

    // When
    let count = shaped(&runs, Some(80.0), 1.0);

    // Then
    assert_eq!(count, 1);
}

#[test]
fn a_run_asked_for_at_three_widths_is_shaped_once_and_broken_at_each() {
    // Given
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    let run = run("Row 42", Align::Center);
    cache.begin(0);

    // When
    let layouts = [None, Some(0.0), Some(80.0)].map(|w| cache.layout(&mut fonts, 1.0, &run, w));

    // Then
    assert_eq!((cache.shaped(), cache.built()), (1, 3));
    let fresh = text_layout(&mut fonts, 1.0, &run, Some(0.0));
    assert_eq!(layouts[1].lines().count(), fresh.lines().count());
    assert_eq!(layouts[1].width(), fresh.width());
}

/// One input changed in each, from `spanned(1)`.
fn changed() -> Vec<(&'static str, InlineRun)> {
    let base = spanned(1);
    let restyled = |change: fn(&mut TextStyle)| {
        let mut run = base.clone();
        change(&mut run.style);
        run
    };
    let respanned = |change: fn(&mut InlineSpan)| {
        let mut run = base.clone();
        change(&mut run.spans[0]);
        run
    };
    vec![
        (
            "text",
            InlineRun {
                text: "Edit wordz".into(),
                ..base.clone()
            },
        ),
        (
            "visible",
            InlineRun {
                visible: false,
                ..base.clone()
            },
        ),
        ("face", restyled(|s| s.face = Face::GeistMono)),
        ("size", restyled(|s| s.size = 17.0)),
        ("weight", restyled(|s| s.weight = 700.0)),
        (
            "line height",
            restyled(|s| s.line_height = LineHeight::Px(30.0)),
        ),
        ("colour", restyled(|s| s.color.r = 1.0)),
        ("letter spacing", restyled(|s| s.letter_spacing = 1.0)),
        ("italic", restyled(|s| s.italic = true)),
        ("span range", respanned(|s| s.range = 0..3)),
        ("span visible", respanned(|s| s.visible = false)),
        ("span weight", respanned(|s| s.style.weight = 700.0)),
    ]
}

#[test]
fn a_run_that_differs_in_any_input_shaping_reads_is_shaped_again() {
    for (input, other) in changed() {
        // Given
        let runs = [spanned(1), other];

        // When
        let count = shaped(&runs, Some(80.0), 1.0);

        // Then
        assert_eq!(count, 2, "{input}");
    }
}

#[test]
fn a_run_at_another_scale_is_shaped_again() {
    // Given
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    cache.begin(0);

    // When
    cache.layout(&mut fonts, 1.0, &spanned(1), None);
    cache.layout(&mut fonts, 2.0, &spanned(1), None);

    // Then
    assert_eq!(cache.shaped(), 2);
}
