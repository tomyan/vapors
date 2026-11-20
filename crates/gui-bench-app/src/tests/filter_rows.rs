//! The spec's rectangles for the filter card.
#![allow(clippy::cast_precision_loss)] // indexes are small
use super::chart_rows::CARD_Y;
use super::layout::{Row, at, from_x, row};
use crate::dashboard::*;

/// The label's left: 28 px right of the box.
const LABEL_X: f32 = CHECK_X + 28.0;

pub(super) fn rows() -> Vec<Row> {
    let x = SIDEBAR_WIDTH + CONTENT_PADDING + CHART_WIDTH + GAP;
    let card = at(x, CARD_Y, FILTERS_WIDTH, ROW2_HEIGHT);
    let label = [Some(CHECK_X), Some(COMPARE_Y), None, Some(28.0)];
    let switch = at(SWITCH_X, SWITCH_Y, SWITCH_WIDTH, SWITCH_HEIGHT);
    let reset = at(RESET_X, BUTTONS_Y, BUTTON_WIDTH, CONTROL_HEIGHT);
    let apply = at(APPLY_X, BUTTONS_Y, BUTTON_WIDTH, CONTROL_HEIGHT);
    let mut all = vec![
        row("filter card".into(), "filters", 0, card),
        row(
            "filters title".into(),
            "filters-title",
            0,
            from_x(x + CARD_PADDING),
        ),
        row("compare label".into(), "compare-label", 0, label),
        row("switch".into(), "switch", 0, switch),
        row("reset".into(), "reset", 0, reset),
        row("apply".into(), "apply", 0, apply),
    ];
    for k in 0..Status::ALL.len() {
        let y = CHECK_TOP + CHECK_STRIDE * k as f32;
        let boxed = at(
            CHECK_X,
            y + (CHECK_STRIDE - CHECK_SIZE) / 2.0,
            CHECK_SIZE,
            CHECK_SIZE,
        );
        let label = [Some(LABEL_X), Some(y), None, Some(CHECK_STRIDE)];
        all.push(row(format!("checkbox {k}"), "check", k, boxed));
        all.push(row(format!("checkbox label {k}"), "check-label", k, label));
    }
    all
}
