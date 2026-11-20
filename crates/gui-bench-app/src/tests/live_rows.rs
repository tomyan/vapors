//! The spec's rectangles for the live switch and its spinner.
use super::layout::{Row, assert_rows, at, row};
use super::support::*;
use crate::dashboard::*;

/// The switch and its knob (off), against `LIVE_*` and the compare switch's knob inset.
fn switch() -> Vec<Row> {
    let track = at(LIVE_X, LIVE_Y, SWITCH_WIDTH, SWITCH_HEIGHT);
    let knob = SWITCH_HEIGHT - 4.0;
    vec![
        row("live switch".into(), "live-switch", 0, track),
        row(
            "live knob".into(),
            "live-knob",
            0,
            at(LIVE_X + 2.0, LIVE_Y + 2.0, knob, knob),
        ),
    ]
}

#[test]
fn the_live_switch_is_where_the_spec_puts_it_and_the_spinner_is_not_there_yet() {
    // Given
    let mounted = laid_out();

    // Then
    assert_rows(&mounted.document.borrow(), &switch());
    assert_eq!(by_class(&mounted.document.borrow(), "spinner").len(), 0);
}

#[test]
fn turned_on_the_knob_moves_right_and_the_spinner_sits_where_the_spec_puts_it() {
    // Given
    let mounted = laid_out();

    // When
    click(&mounted, [LIVE_X + 20.0, LIVE_Y + 11.0]);

    // Then
    let knob = SWITCH_HEIGHT - 4.0;
    let on_knob = at(LIVE_X + SWITCH_WIDTH - 2.0 - knob, LIVE_Y + 2.0, knob, knob);
    let spinner = at(
        SPINNER_X,
        TOPBAR_HEIGHT / 2.0 - SPINNER_SIZE / 2.0,
        SPINNER_SIZE,
        SPINNER_SIZE,
    );
    let rows = [
        row("live knob on".into(), "live-knob", 0, on_knob),
        row("spinner".into(), "spinner", 0, spinner),
    ];
    assert_rows(&mounted.document.borrow(), &rows);
}
