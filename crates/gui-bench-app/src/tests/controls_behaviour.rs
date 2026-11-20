//! What the chart's segments, the compare switch and the buttons do,
//! through vapors' test drivers.
use super::behaviour_top::{centre_of, texts};
use super::chart_rows::bars;
use super::layout::assert_rows;
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;
use vapors_signals::Signal;

fn paint(mounted: &Mounted, class: &str, nth: usize, property: &str) -> String {
    let doc = mounted.document.borrow();
    let node = by_class(&doc, class)[nth];
    let style = doc.computed_style(node).expect("styled");
    style.get_by_name(property).expect("a property").to_css()
}

/// A colour as the style engine writes it.
fn css(color: u32) -> String {
    let [_, r, g, b] = color.to_be_bytes();
    format!("rgb({r} {g} {b})")
}

/// The left edge of the compare switch's knob.
fn knob_x(mounted: &Mounted) -> f32 {
    let doc = mounted.document.borrow();
    rect(&doc, by_class(&doc, "knob")[0])[0]
}

fn prev_count(mounted: &Mounted) -> usize {
    by_class(&mounted.document.borrow(), "prev").len()
}

#[test]
fn a_click_on_a_segment_selects_it_and_shows_its_series() {
    // Given
    let mounted = laid_out();
    assert_eq!(texts(&mounted, "segment"), SEGMENTS);
    let selected = |k: usize| paint(&mounted, "segment", k, "background-color") == css(CARD_BG);
    assert_eq!(
        [selected(0), selected(1), selected(2)],
        [true, false, false]
    );

    // When
    for series in [1, 2, 0] {
        click(&mounted, centre_of(&mounted, "segment", series));

        // Then
        let picked: Vec<bool> = (0..SEGMENTS.len()).map(selected).collect();
        assert_eq!(
            picked,
            (0..SEGMENTS.len()).map(|k| k == series).collect::<Vec<_>>()
        );
        assert_rows(&mounted.document.borrow(), &bars(series, false));
    }
}

#[test]
fn every_series_shows_every_bar_and_previous_bar_at_the_specs_height() {
    // Given
    let mounted = laid_out();
    click(&mounted, centre_of(&mounted, "switch", 0));

    // When
    for series in 0..SEGMENTS.len() {
        click(&mounted, centre_of(&mounted, "segment", series));

        // Then: with compare on, then off
        assert_rows(&mounted.document.borrow(), &bars(series, true));
        assert_eq!(prev_count(&mounted), CHART_BARS);
        click(&mounted, centre_of(&mounted, "switch", 0));
        assert_rows(&mounted.document.borrow(), &bars(series, false));
        assert_eq!(prev_count(&mounted), 0);
        click(&mounted, centre_of(&mounted, "switch", 0));
    }
}

#[test]
fn the_compare_switch_adds_the_previous_period_and_narrows_the_bars() {
    // Given
    let mounted = laid_out();
    let off = paint(&mounted, "switch", 0, "background-color");
    assert_eq!(off, css(SWITCH_OFF));
    assert_eq!(prev_count(&mounted), 0);
    assert_eq!(knob_x(&mounted), SWITCH_X + 2.0);

    // When
    click(&mounted, centre_of(&mounted, "switch", 0));

    // Then
    assert_eq!(
        paint(&mounted, "switch", 0, "background-color"),
        css(PRIMARY)
    );
    assert_rows(&mounted.document.borrow(), &bars(0, true));
    assert_eq!(knob_x(&mounted), SWITCH_X + SWITCH_WIDTH - 2.0 - 18.0);

    // When: a second click
    click(&mounted, centre_of(&mounted, "switch", 0));

    // Then
    assert_rows(&mounted.document.borrow(), &bars(0, false));
    assert_eq!(prev_count(&mounted), 0);
    assert_eq!(knob_x(&mounted), SWITCH_X + 2.0);
    assert_eq!(paint(&mounted, "switch", 0, "background-color"), off);
}

#[test]
fn reset_checks_every_status_turns_compare_off_picks_the_first_segment_and_clears_the_search() {
    // Given
    let mounted = laid_out();
    let controls = mounted.controls;
    click(&mounted, centre_of(&mounted, "switch", 0));
    click(&mounted, centre_of(&mounted, "segment", 1));
    for k in 0..controls.shown.len() {
        click(&mounted, centre_of(&mounted, "check", k));
    }
    controls.search.set("ada".into());
    refresh(&mounted);
    assert!(controls.compare.get_untracked());
    assert_eq!(controls.series.get_untracked(), 1);
    assert!(!controls.shown.iter().any(Signal::get_untracked));
    assert_eq!(controls.search.get_untracked(), "ada");

    // When
    click(&mounted, centre_of(&mounted, "reset", 0));

    // Then
    assert!(!controls.compare.get_untracked());
    assert_eq!(controls.series.get_untracked(), 0);
    assert!(controls.shown.iter().all(Signal::get_untracked));
    assert_eq!(controls.search.get_untracked(), "");
    assert_rows(&mounted.document.borrow(), &bars(0, false));
}

#[test]
fn apply_changes_nothing() {
    // Given: compare off, a segment picked, a status hidden
    let mounted = laid_out();
    click(&mounted, centre_of(&mounted, "segment", 1));
    click(&mounted, centre_of(&mounted, "check", 3));
    mounted.controls.search.set("ada".into());

    // When
    click(&mounted, centre_of(&mounted, "apply", 0));

    // Then
    let controls = mounted.controls;
    assert_rows(&mounted.document.borrow(), &bars(1, false));
    assert!(!controls.compare.get_untracked() && !controls.shown[3].get_untracked());
    assert_eq!(controls.search.get_untracked(), "ada");
}
