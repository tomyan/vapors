//! A text box offered a width its lines fit within the slack measures
//! that width, not the slack past it.

use taffy::{AvailableSpace, Size};
use vapors_text::{Align, FontSystem, LineHeight};

use super::*;
use crate::layout::TextCache;
use crate::style::{Face, TextStyle};

fn run(text: &str) -> InlineRun {
    let style = TextStyle {
        face: Face::Geist,
        size: 15.0,
        weight: 400.0,
        line_height: LineHeight::Normal,
        color: vapors_scene::Color::default(),
        letter_spacing: 0.0,
        align: Align::Start,
        italic: false,
    };
    InlineRun {
        text: text.to_owned(),
        style,
        visible: true,
        spans: Vec::new(),
    }
}

/// `run`'s width measured at `available` CSS pixels at `scale`.
fn measured(run: &InlineRun, scale: f32, available: AvailableSpace) -> f32 {
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    let known = Size {
        width: None,
        height: None,
    };
    let available = Size {
        width: available,
        height: AvailableSpace::MaxContent,
    };
    measure_text((&mut fonts, &mut cache), scale, run, known, available).width
}

#[test]
fn text_offered_a_width_just_under_its_own_measures_the_width_offered() {
    for scale in [1.0, 2.0, 2.625, 2.75, 2.8125, 3.0] {
        // Given: its max-content width, a rounding short
        let run = run("Clicked 17 times");
        let max_content = measured(&run, scale, AvailableSpace::MaxContent);
        let offered = max_content.next_down();

        // When
        let width = measured(&run, scale, AvailableSpace::Definite(offered));

        // Then
        assert_eq!(width.to_bits(), offered.to_bits(), "at {scale}");
    }
}

#[test]
fn text_wider_than_the_slack_past_its_width_measures_its_own_width() {
    // Given: min-content, where the widest word overflows the 0 offered
    let run = run("Clicked 17 times");

    // When
    let width = measured(&run, 1.0, AvailableSpace::MinContent);

    // Then: the widest word, not clamped to 0
    assert!(width > 10.0, "{width}");
}
