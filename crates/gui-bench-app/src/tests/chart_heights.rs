//! The chart's bar heights are bound, not written out as rules, and equal
//! Chromium's boxes for the same CSS (`chart_oracle.rs`, live) for every
//! series, compare off and on, at mount and after each update (a segment
//! click, the compare switch).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // indexes are small
use super::behaviour_top::centre_of;
use super::chart_oracle::{Bar, REPRODUCE, bar, chromium_bars};
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;

/// Chromium's layout unit, 1/64 px.
const UNIT: f32 = 1.0 / 64.0;
/// A slot's width in px (`.slot` in `ui/chart_plot.wisp`).
const SLOT_W: f32 = 25.0;

/// A bar's slot and its [x, y, w, h] relative to the plot.
type Rel = (usize, [f32; 4]);

/// Chromium's boxes of `class` in state (`series`, `compare`).
fn chromium(all: &[Bar], series: usize, compare: bool, class: &str) -> Vec<Rel> {
    let same = |b: &&Bar| b.series == series && b.compare == compare && b.class == class;
    all.iter().filter(same).map(|b| (b.slot, b.rect)).collect()
}

/// The app's boxes of class `class` relative to its plot; a bar's slot is
/// the slot column its box starts in.
fn app(mounted: &Mounted, class: &str) -> Vec<Rel> {
    let doc = mounted.document.borrow();
    let [px, py, ..] = rect(&doc, by_class(&doc, "plot")[0]);
    let rel = |n| {
        let [x, y, w, h] = rect(&doc, n);
        (
            (((x - px) / SLOT_W).floor()) as usize,
            [x - px, y - py, w, h],
        )
    };
    by_class(&doc, class).into_iter().map(rel).collect()
}

fn assert_same(got: &[Rel], want: &[Rel], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: bar count; run {REPRODUCE}");
    for (i, ((gs, g), (ws, w))) in got.iter().zip(want).enumerate() {
        assert_eq!(
            gs, ws,
            "{what} bar {i}: vapors slot {gs}, Chromium slot {ws}"
        );
        for k in 0..4 {
            let near = (g[k] - w[k]).abs() <= UNIT;
            assert!(near, "{what} bar {i}: vapors {g:?}, Chromium {w:?}");
        }
    }
}

fn check(all: &[Bar], mounted: &Mounted, series: usize, compare: bool) {
    let what = format!("series {series} compare {compare}");
    for class in ["cur", "prev"] {
        let want = chromium(all, series, compare, class);
        assert_same(&app(mounted, class), &want, &format!("{what} {class}"));
    }
}

/// A guard on the source text, not proof: the Chromium tests are the proof.
#[test]
fn the_sheet_has_no_per_bar_height_rules_and_binds_each_height() {
    let source = include_str!("../../ui/chart_plot.wisp");
    assert!(
        !source.contains("nth-child"),
        "per-bar rules remain in ui/chart_plot.wisp"
    );
    assert_eq!(
        source.matches("style:height=").count(),
        2,
        "cur and prev bind style:height"
    );
}

#[test]
fn chromium_lays_out_every_state_of_the_oracle_page() {
    let all = chromium_bars();
    for series in 0..SEGMENTS.len() {
        assert_eq!(chromium(all, series, false, "cur").len(), CHART_BARS);
        assert_eq!(chromium(all, series, false, "prev").len(), 0);
        assert_eq!(chromium(all, series, true, "prev").len(), CHART_BARS);
    }
}

#[test]
fn every_series_and_compare_state_equals_chromium_after_each_update() {
    let all = chromium_bars();
    let mounted = laid_out();
    check(all, &mounted, 0, false);
    for series in [1, 2, 0] {
        click(&mounted, centre_of(&mounted, "segment", series));
        check(all, &mounted, series, false);
        click(&mounted, centre_of(&mounted, "switch", 0));
        check(all, &mounted, series, true);
        click(&mounted, centre_of(&mounted, "segment", (series + 1) % 3));
        check(all, &mounted, (series + 1) % 3, true);
        click(&mounted, centre_of(&mounted, "switch", 0));
        click(&mounted, centre_of(&mounted, "segment", series));
        check(all, &mounted, series, false);
    }
}

#[test]
#[should_panic(expected = "mismatch bar 3: vapors")]
fn a_deliberate_mismatch_fails() {
    let mut cur = chromium(chromium_bars(), 0, false, "cur");
    cur[3].1[3] += 1.0;
    assert_same(&app(&laid_out(), "cur"), &cur, "mismatch");
}

#[test]
#[should_panic(expected = "mismatch bar 2: vapors slot")]
fn a_deliberate_slot_mismatch_fails() {
    let mut cur = chromium(chromium_bars(), 0, false, "cur");
    cur[2].0 += 1;
    assert_same(&app(&laid_out(), "cur"), &cur, "mismatch");
}

#[test]
#[should_panic(expected = "outside 3 series")]
fn a_series_past_the_spec_is_refused() {
    crate::chart_bars::current_height(SEGMENTS.len(), 0);
}

#[test]
#[should_panic(expected = "outside 3 series")]
fn a_slot_past_the_spec_is_refused() {
    crate::chart_bars::previous_height(0, CHART_BARS);
}

#[test]
#[should_panic(expected = "compare \"maybe\" is not true or false")]
fn a_malformed_compare_field_is_refused() {
    bar("0 maybe cur 1 0 0 17 40");
}
