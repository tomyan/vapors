use vapors_dom::NodeId;
use vapors_text::LineHeight;

use super::super::inline::InlineSpan;
use super::*;
use crate::style::Face;

fn style(align: Align, letter_spacing: f32) -> TextStyle {
    TextStyle {
        face: Face::Geist,
        size: 16.0,
        weight: 400.0,
        line_height: LineHeight::Normal,
        color: vapors_scene::Color::default(),
        letter_spacing,
        align,
        italic: false,
    }
}

fn run(text: &str, align: Align) -> InlineRun {
    InlineRun {
        text: text.to_owned(),
        style: style(align, 0.0),
        visible: true,
        spans: Vec::new(),
    }
}

/// The lines and every glyph's id and position: what is drawn.
fn drawn(layout: &Layout) -> (Vec<vapors_text::LineInfo>, Vec<(u32, f32, f32)>) {
    let glyphs = layout.glyphs().map(|g| (g.id, g.x, g.y)).collect();
    (layout.lines().collect(), glyphs)
}

/// `run` asked for at max-content and then at `width` from one cache,
/// with how many it built, and `run` laid out at `width` from scratch.
fn after_max_content(run: &InlineRun, width: f32) -> (Rc<Layout>, usize, Layout) {
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    cache.layout(&mut fonts, 1.0, run, None);
    let kept = cache.layout(&mut fonts, 1.0, run, Some(width));
    let fresh = text_layout(&mut fonts, 1.0, run, Some(width));
    (kept, cache.built(), fresh)
}

#[test]
fn a_run_whose_style_holds_a_nan_is_kept_like_any_other() {
    // Given: a letter spacing that is not a number
    let run = InlineRun {
        style: style(Align::Start, f32::NAN),
        ..run("nan", Align::Start)
    };
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());

    // When: asked for twice
    cache.layout(&mut fonts, 1.0, &run, None);
    cache.layout(&mut fonts, 1.0, &run, None);

    // Then: laid out once
    assert_eq!(cache.built(), 1);
}

#[test]
fn a_start_aligned_run_at_a_width_it_fits_takes_its_max_content_layout() {
    // Given
    let run = run("00:00.01 lap ", Align::Start);

    // When
    let (kept, built, fresh) = after_max_content(&run, 400.0);

    // Then: one layout, drawn as one built at that width would be
    assert_eq!(built, 1);
    assert_eq!(drawn(&kept), drawn(&fresh));
}

#[test]
fn a_run_with_forced_breaks_at_its_widest_line_takes_its_max_content_layout() {
    // Given
    let run = run("TIME\n00:00.01", Align::Start);
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    let widest = cache.layout(&mut fonts, 1.0, &run, None).width();

    // When
    let (kept, built, fresh) = after_max_content(&run, widest);

    // Then
    assert_eq!(built, 1);
    assert_eq!(drawn(&kept), drawn(&fresh));
}

#[test]
fn a_run_narrower_than_its_max_content_is_laid_out_again() {
    // Given: wraps at 40 px
    let run = run("a few words that wrap", Align::Start);

    // When
    let (kept, built, fresh) = after_max_content(&run, 40.0);

    // Then
    assert_eq!(drawn(&kept), drawn(&fresh));
    assert_eq!(built, 2);
}

#[test]
fn a_centred_or_end_aligned_run_is_laid_out_again_at_its_width() {
    for align in [Align::Center, Align::End] {
        // Given: alignment moves the line within the width
        let run = run("TIME\n00:00.01", align);

        // When
        let (kept, built, fresh) = after_max_content(&run, 400.0);

        // Then
        assert_eq!(drawn(&kept), drawn(&fresh), "{align:?}");
        assert_eq!(built, 2, "{align:?}");
    }
}

#[test]
fn a_number_that_is_not_finite_never_fits() {
    for (content, available) in [
        (10.0, f32::NAN),
        (f32::NAN, 100.0),
        (f32::NAN, f32::NAN),
        (10.0, f32::INFINITY),
        (f32::INFINITY, f32::INFINITY),
    ] {
        assert!(!fits(content, available), "{content} in {available}");
    }
    assert!(fits(10.0, 10.0));
}

#[test]
fn a_run_with_a_nan_letter_spacing_is_laid_out_like_a_fresh_one() {
    // Given: a NaN spacing, so every glyph after the first sits at NaN
    let run = InlineRun {
        style: style(Align::Start, f32::NAN),
        ..run("a few words that wrap", Align::Start)
    };

    // When
    let (kept, built, fresh) = after_max_content(&run, 40.0);

    // Then: built again, and drawn as a fresh build is (compared as text,
    // since NaN is not equal to itself)
    assert_eq!(built, 2);
    assert_eq!(
        format!("{:?}", drawn(&kept)),
        format!("{:?}", drawn(&fresh))
    );
}

#[test]
fn a_run_with_a_negative_letter_spacing_is_laid_out_like_a_fresh_one() {
    // Given
    let run = InlineRun {
        style: style(Align::Start, -2.0),
        ..run("a few words that wrap", Align::Start)
    };

    // When
    let (kept, built, fresh) = after_max_content(&run, 400.0);

    // Then
    assert_eq!(built, 2);
    assert_eq!(drawn(&kept), drawn(&fresh));
}

#[test]
fn a_span_with_a_negative_letter_spacing_is_laid_out_like_a_fresh_one() {
    // Given
    let run = InlineRun {
        spans: vec![InlineSpan {
            node: NodeId(1),
            range: 2..6,
            visible: true,
            style: style(Align::Start, -2.0),
        }],
        ..run("a few words that wrap", Align::Start)
    };

    // When
    let (kept, built, fresh) = after_max_content(&run, 400.0);

    // Then
    assert_eq!(built, 2);
    assert_eq!(drawn(&kept), drawn(&fresh));
}

#[test]
fn reuse_at_a_fractional_scale_is_recorded() {
    // Given: max-content asked for at scale 1.25, then a width it fits
    let run = run("00:00.01 lap ", Align::Start);
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    cache.layout(&mut fonts, 1.25, &run, None);

    // When
    let kept = cache.layout(&mut fonts, 1.25, &run, Some(400.0));
    let fresh = text_layout(&mut fonts, 1.25, &run, Some(400.0));

    // Then: reused, and drawn as a fresh build is
    assert_eq!(cache.built(), 1);
    assert_eq!(drawn(&kept), drawn(&fresh));
}
