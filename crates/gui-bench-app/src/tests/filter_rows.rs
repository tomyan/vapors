//! The spec's rectangles for the filter card.
#![allow(clippy::cast_precision_loss)] // indexes are small
use super::layout::{Row, at, from_x, row};
use crate::dashboard::*;

pub(super) fn rows() -> Vec<Row> {
    let (x, y) = (SIDEBAR_WIDTH + CONTENT_PADDING + CHART_WIDTH + GAP, 216.0);
    let mut all = vec![
        row(
            "filter card".into(),
            "filters",
            0,
            at(x, y, FILTERS_WIDTH, ROW2_HEIGHT),
        ),
        row("filters title".into(), "filters-title", 0, from_x(x + 20.0)),
        row(
            "compare label".into(),
            "compare-label",
            0,
            [Some(CHECK_X), Some(COMPARE_Y), None, Some(28.0)],
        ),
        row(
            "switch".into(),
            "switch",
            0,
            at(SWITCH_X, SWITCH_Y, SWITCH_WIDTH, SWITCH_HEIGHT),
        ),
        row(
            "reset".into(),
            "reset",
            0,
            at(RESET_X, BUTTONS_Y, BUTTON_WIDTH, CONTROL_HEIGHT),
        ),
        row(
            "apply".into(),
            "apply",
            0,
            at(APPLY_X, BUTTONS_Y, BUTTON_WIDTH, CONTROL_HEIGHT),
        ),
    ];
    for k in 0..Status::ALL.len() {
        let y = CHECK_TOP + CHECK_STRIDE * k as f32;
        let boxed = at(
            CHECK_X,
            y + (CHECK_STRIDE - CHECK_SIZE) / 2.0,
            CHECK_SIZE,
            CHECK_SIZE,
        );
        let label = [Some(968.0), Some(y), None, Some(CHECK_STRIDE)];
        all.push(row(format!("checkbox {k}"), "check", k, boxed));
        all.push(row(format!("checkbox label {k}"), "check-label", k, label));
    }
    all
}
