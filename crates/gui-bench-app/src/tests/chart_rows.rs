//! The spec's rectangles for the chart card: title, segments, grid, bars.
#![allow(clippy::cast_precision_loss)] // indexes are small
use super::layout::{Row, at, from_x, row};
use crate::dashboard::*;

/// The chart card's left and top.
pub(super) const CARD_X: f32 = SIDEBAR_WIDTH + CONTENT_PADDING;
pub(super) const CARD_Y: f32 = TOPBAR_HEIGHT + CONTENT_PADDING + KPI_HEIGHT + GAP;
/// The segment track: 2 px of padding round three segments, 16 px down.
const TRACK_WIDTH: f32 = SEGMENT_WIDTH * 3.0 + 4.0;
const TRACK_X: f32 = CARD_X + CHART_WIDTH - CARD_PADDING - TRACK_WIDTH;
const TRACK_Y: f32 = CARD_Y + 16.0;

pub(super) fn rows() -> Vec<Row> {
    let card = at(CARD_X, CARD_Y, CHART_WIDTH, ROW2_HEIGHT);
    let track = at(TRACK_X, TRACK_Y, TRACK_WIDTH, SEGMENT_HEIGHT + 4.0);
    let plot = at(
        PLOT_X,
        PLOT_TOP,
        CHART_SLOT * CHART_BARS as f32,
        PLOT_BOTTOM - PLOT_TOP,
    );
    let mut all = vec![
        row("chart card".into(), "chart", 0, card),
        row("chart title".into(), "chart-title", 0, from_x(PLOT_X)),
        row("segments".into(), "segments", 0, track),
        row("plot".into(), "plot", 0, plot),
    ];
    for k in 0..SEGMENTS.len() {
        let x = TRACK_X + 2.0 + k as f32 * SEGMENT_WIDTH;
        let want = at(x, TRACK_Y + 2.0, SEGMENT_WIDTH, SEGMENT_HEIGHT);
        all.push(row(format!("segment {k}"), "segment", k, want));
    }
    for (k, y) in GRID_LINES.iter().enumerate() {
        let want = at(PLOT_X, *y, CHART_SLOT * CHART_BARS as f32, 1.0);
        all.push(row(format!("grid line {k}"), "grid", k, want));
    }
    all
}

/// Bar `i` of the current period at series `series`; with `compare` on
/// the bar is the right 8 px of its slot.
pub(super) fn current(series: usize, i: usize, compare: bool) -> Row {
    let (offset, width) = if compare { (13.0, 8.0) } else { (4.0, 17.0) };
    let h = bar_height(series, i);
    let x = PLOT_X + CHART_SLOT * i as f32 + offset;
    let want = at(x, PLOT_BOTTOM - h, width, h);
    row(format!("series {series} bar {i}"), "cur", i, want)
}

/// Bar `i` of the previous period at series `series`, shown with compare on.
pub(super) fn previous(series: usize, i: usize) -> Row {
    let h = previous_height(series, i);
    let x = PLOT_X + CHART_SLOT * i as f32 + 3.0;
    let want = at(x, PLOT_BOTTOM - h, 8.0, h);
    row(format!("series {series} previous {i}"), "prev", i, want)
}

/// Every bar of series `series`, with the previous period when `compare`.
pub(super) fn bars(series: usize, compare: bool) -> Vec<Row> {
    let mut all: Vec<Row> = (0..CHART_BARS)
        .map(|i| current(series, i, compare))
        .collect();
    if compare {
        all.extend((0..CHART_BARS).map(|i| previous(series, i)));
    }
    all
}
