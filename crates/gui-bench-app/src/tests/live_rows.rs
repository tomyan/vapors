//! The spec's rectangles for the live switch and its spinner.
use super::layout::{Row, assert_rows, at, row};
use super::support::*;
use crate::dashboard::*;

/// The live switch's box (off), against `LIVE_*`; its knob is a sibling element, held by `switch_knob.rs`.
fn switch() -> Vec<Row> {
    vec![row(
        "live switch".into(),
        "live-switch",
        0,
        at(LIVE_X, LIVE_Y, SWITCH_WIDTH, SWITCH_HEIGHT),
    )]
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
fn turned_on_the_switch_keeps_its_box_and_the_spinner_sits_where_the_spec_puts_it() {
    // Given
    let mounted = laid_out();

    // When
    click(&mounted, [LIVE_X + 20.0, LIVE_Y + 11.0]);

    // Then: the part keeps its box and the spinner appears
    let spinner = at(
        SPINNER_X,
        TOPBAR_HEIGHT / 2.0 - SPINNER_SIZE / 2.0,
        SPINNER_SIZE,
        SPINNER_SIZE,
    );
    let rows = [
        switch().remove(0),
        row("spinner".into(), "spinner", 0, spinner),
    ];
    assert_rows(&mounted.document.borrow(), &rows);
}
