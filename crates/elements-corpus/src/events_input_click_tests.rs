//! Clicks in a row: `double-click` and `triple-click` read as presses
//! counting up, sent to Chromium with each press's `clickCount`.

use super::*;
use crate::events_script::{Action, parse};

#[test]
fn double_and_triple_click_read_as_runs_of_two_and_three() {
    // When
    let script = parse(
        "w",
        "listen click\ndouble-click 60 12\nexpect event click input#a\ntriple-click 5 6\nexpect event click input#a\n",
    )
    .expect("parses");
    let refused = parse("w", "listen click\ndouble-click 60\n").expect_err("refused");

    // Then
    assert_eq!(script.steps[0].action, Action::Clicks(60.0, 12.0, 2));
    assert_eq!(script.steps[1].action, Action::Clicks(5.0, 6.0, 3));
    assert!(
        refused.contains("w.events:2:") && refused.contains("X Y"),
        "{refused}"
    );
}

#[test]
fn a_run_of_clicks_counts_each_press_and_release_up() {
    // When
    let calls = calls(&Action::Clicks(12.0, 34.0, 2), &mut Pointer::default());

    // Then
    let sent: Vec<(&str, i64, f64)> = calls
        .iter()
        .map(|(_, p)| {
            (
                p["type"].as_str().expect("type"),
                p["clickCount"].as_i64().expect("count"),
                p["x"].as_f64().expect("x"),
            )
        })
        .collect();
    assert_eq!(
        sent,
        [
            ("mousePressed", 1, 12.0),
            ("mouseReleased", 1, 12.0),
            ("mousePressed", 2, 12.0),
            ("mouseReleased", 2, 12.0),
        ]
    );
}
