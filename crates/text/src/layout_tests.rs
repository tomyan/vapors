//! Shaping and layout against the bundled fonts, with no GPU.

use super::*;
use crate::{Family, FontSystem, LayoutOptions, Style};

fn system() -> FontSystem {
    FontSystem::bundle_only()
}

fn advances(layout: &Layout) -> Vec<f32> {
    layout.glyphs().map(|g| g.advance).collect()
}

fn single_line(system: &mut FontSystem, text: &str, style: &Style<'_>) -> Layout {
    Layout::new(system, text, style, &LayoutOptions::default())
}

#[test]
fn kerning_changes_advances_and_can_be_turned_off() {
    // Given
    let mut system = system();
    let kerned = Style::new(Family::Geist, 16.0);
    let unkerned = Style {
        kerning: false,
        ..Style::new(Family::Geist, 16.0)
    };

    // When
    let with = single_line(&mut system, "AVAVAV", &kerned);
    let without = single_line(&mut system, "AVAVAV", &unkerned);

    // Then
    assert_ne!(advances(&with), advances(&without));
    assert!(
        with.width() < without.width(),
        "kerning pulls A and V together"
    );
}

#[test]
fn tabular_figures_give_every_digit_the_same_advance() {
    // Given
    let mut system = system();
    let tabular = Style {
        tabular_figures: true,
        ..Style::new(Family::Geist, 16.0)
    };
    let proportional = Style::new(Family::Geist, 16.0);

    // When
    let tabular_advances = advances(&single_line(&mut system, "0123456789", &tabular));
    let proportional_advances = advances(&single_line(&mut system, "0123456789", &proportional));

    // Then
    assert_eq!(tabular_advances.len(), 10);
    let first = tabular_advances[0];
    assert!(
        tabular_advances.iter().all(|a| (a - first).abs() < 1e-4),
        "tnum advances {tabular_advances:?}"
    );
    assert!(
        proportional_advances
            .iter()
            .any(|a| (a - proportional_advances[0]).abs() > 0.1),
        "proportional advances {proportional_advances:?} should differ"
    );
}

#[test]
fn a_line_wraps_before_the_word_that_does_not_fit() {
    // Given
    let mut system = system();
    let style = Style::new(Family::Geist, 16.0);
    let text = "the quick brown fox";
    let one_line = single_line(&mut system, "the quick brown", &style);
    let width = one_line.width() - 1.0;

    // When
    let layout = Layout::new(
        &mut system,
        text,
        &style,
        &LayoutOptions {
            width: Some(width),
            ..LayoutOptions::default()
        },
    );
    let lines: Vec<_> = layout.lines().collect();

    // Then
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(&text[lines[0].text_range.clone()], "the quick ");
    assert_eq!(&text[lines[1].text_range.clone()], "brown fox");
}

#[test]
fn a_hebrew_run_in_a_latin_line_is_laid_out_right_to_left() {
    // Given
    let mut system = system();
    let style = Style::new(Family::Geist, 16.0);
    let text = "abc אבג def";
    let aleph = text.find('א').expect("aleph");
    let gimel = text.find('ג').expect("gimel");

    // When
    let layout = single_line(&mut system, text, &style);
    let (aleph_x, gimel_x) = (layout.cluster_x(aleph), layout.cluster_x(gimel));
    let rtl_runs = layout
        .inner()
        .lines()
        .flat_map(|l| l.runs())
        .filter(parley::Run::is_rtl)
        .count();

    // Then
    assert_eq!(rtl_runs, 1);
    assert!(
        gimel_x < aleph_x,
        "gimel at {gimel_x} should be left of aleph at {aleph_x}"
    );
}

#[test]
fn optical_size_is_accepted_and_ignored_by_a_font_without_the_axis() {
    // Given
    let mut system = system();
    let plain = Style::new(Family::Geist, 16.0);
    let opsz = Style {
        optical_size: Some(12.0),
        ..Style::new(Family::Geist, 16.0)
    };

    // When
    let a = advances(&single_line(&mut system, "Hamburg", &plain));
    let b = advances(&single_line(&mut system, "Hamburg", &opsz));

    // Then
    assert_eq!(a, b);
}

#[test]
fn weight_changes_advances_in_the_variable_font() {
    // Given
    let mut system = system();
    let regular = Style::new(Family::Geist, 16.0);
    let bold = Style {
        weight: 700.0,
        ..Style::new(Family::Geist, 16.0)
    };

    // When
    let a = single_line(&mut system, "Hamburg", &regular);
    let b = single_line(&mut system, "Hamburg", &bold);

    // Then
    assert!(b.width() > a.width());
    assert!(
        a.glyphs()
            .all(|g| g.coords.is_empty() || g.coords.iter().all(|c| c.to_bits() == 0))
    );
    assert!(
        b.glyphs()
            .all(|g| g.coords.iter().any(|c| c.to_bits() != 0))
    );
}

#[test]
fn geist_mono_glyphs_share_one_advance() {
    // Given
    let mut system = system();
    let style = Style::new(Family::GeistMono, 13.0);

    // When
    let advances = advances(&single_line(&mut system, "iW.0", &style));

    // Then
    assert!(
        advances.iter().all(|a| (a - advances[0]).abs() < 1e-4),
        "{advances:?}"
    );
}

#[test]
fn glyphs_sit_on_the_first_baseline_at_the_requested_scale() {
    // Given
    let mut system = system();
    let style = Style::new(Family::Geist, 10.0);

    // When
    let layout = Layout::new(
        &mut system,
        "x",
        &style,
        &LayoutOptions {
            scale: 2.0,
            ..LayoutOptions::default()
        },
    );
    let glyph = layout.glyphs().next().expect("one glyph");

    // Then
    assert_eq!(glyph.size, 20.0);
    assert_eq!(glyph.y, layout.lines().next().expect("line").baseline);
}
