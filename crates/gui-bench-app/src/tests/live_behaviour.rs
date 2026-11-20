//! The live switch: the tick and the spinner follow the frame clock, which
//! the tests advance themselves (a fake clock: no frame is drawn).
use super::behaviour_top::texts;
use super::support::*;
use crate::dashboard::*;
use crate::live::spinner_path;
use std::time::Duration;
use vapors_elements::{advance_frame_clock, frame_clock_wanted};

const SWITCH: [f32; 2] = [LIVE_X + 20.0, LIVE_Y + 11.0];

fn clock(mounted: &crate::app::Mounted, millis: u64) {
    advance_frame_clock(Duration::from_millis(millis));
    refresh(mounted);
}

fn values(tick: u64) -> Vec<String> {
    (0..KPI_COUNT).map(|k| kpi_value(k, tick)).collect()
}

/// The first point of the spinner's path.
fn first_point(path: &str) -> (f32, f32) {
    let mut numbers = path[1..]
        .split([' ', 'L'])
        .map(|n| n.parse::<f32>().expect("a number"));
    (numbers.next().unwrap(), numbers.next().unwrap())
}

#[test]
fn the_metrics_follow_the_tick_every_interval_while_on_and_return_when_off() {
    // Given: the switch turned on, and the clock at its first frame
    let mounted = laid_out();
    assert_eq!(texts(&mounted, "kpi-value"), values(0));
    click(&mounted, SWITCH);
    clock(&mounted, 5_000);

    // When / Then: one tick per interval of the clock (the second frame is the count's start)
    let start = 5_000 + 16;
    clock(&mounted, start);
    assert_eq!(texts(&mounted, "kpi-value"), values(0));
    for tick in [1, 2, 3, 10] {
        clock(&mounted, start + tick * LIVE_INTERVAL_MS - 1);
        assert_eq!(
            texts(&mounted, "kpi-value"),
            values(tick - 1),
            "just before tick {tick}"
        );
        clock(&mounted, start + tick * LIVE_INTERVAL_MS);
        assert_eq!(texts(&mounted, "kpi-value"), values(tick), "tick {tick}");
    }
    assert!(frame_clock_wanted(), "the loop draws while it is on");

    // When: turned off
    click(&mounted, SWITCH);

    // Then: the startup values, no spinner, and no frame asked for
    assert_eq!(texts(&mounted, "kpi-value"), values(0));
    assert_eq!(by_class(&mounted.document.borrow(), "spinner").len(), 0);
    assert!(!frame_clock_wanted(), "the loop idles when it is off");
}

#[test]
fn turned_on_after_a_wait_it_counts_from_the_frame_of_the_click() {
    // Given: on, then off, the clock far behind
    let mounted = laid_out();
    click(&mounted, SWITCH);
    clock(&mounted, 1_000);
    clock(&mounted, 1_016);
    click(&mounted, SWITCH);

    // When: on again, whose first frame comes a minute later
    click(&mounted, SWITCH);
    clock(&mounted, 61_000);
    clock(&mounted, 61_016);

    // Then: no ticks for the wait
    assert_eq!(texts(&mounted, "kpi-value"), values(0));
    clock(&mounted, 61_016 + LIVE_INTERVAL_MS);
    assert_eq!(texts(&mounted, "kpi-value"), values(1));
}

#[test]
fn the_spinner_turns_at_a_constant_speed_once_per_period() {
    // Given: points of the path every eighth of a period
    let step = SPINNER_PERIOD_MS / 8;
    let at = |ms: u64| first_point(&spinner_path(Duration::from_millis(ms)));
    let angle = |ms: u64| {
        let (x, y) = at(ms);
        (y - SPINNER_SIZE / 2.0).atan2(x - SPINNER_SIZE / 2.0)
    };

    // Then: each step turns the same angle (a clockwise eighth), and a period is a full turn
    for k in 0..16 {
        let turned = (angle((k + 1) * step) - angle(k * step)).rem_euclid(std::f32::consts::TAU);
        assert!(
            (turned - std::f32::consts::FRAC_PI_4).abs() < 1e-3,
            "step {k}: {turned}"
        );
    }
    assert_eq!(
        spinner_path(Duration::ZERO),
        spinner_path(Duration::from_millis(SPINNER_PERIOD_MS))
    );
}

#[test]
fn the_spinner_in_the_document_turns_with_the_clock() {
    // Given
    let mounted = laid_out();
    click(&mounted, SWITCH);
    clock(&mounted, 100);

    // When
    let path = |mounted: &crate::app::Mounted| {
        let doc = mounted.document.borrow();
        let paths: Vec<_> = doc
            .elements()
            .into_iter()
            .filter(|n| doc.tag(*n) == Some("path"))
            .collect();
        doc.attribute(paths[0], "d").expect("a path").to_owned()
    };
    clock(&mounted, 200);
    let before = path(&mounted);
    clock(&mounted, 300);

    // Then
    assert_ne!(path(&mounted), before);
    assert_eq!(path(&mounted), spinner_path(Duration::from_millis(200)));
}
