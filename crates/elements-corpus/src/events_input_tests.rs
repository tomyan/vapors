//! Actions as `DevTools` `Input` calls, and expectations checked against
//! what Chromium logged.

use serde_json::json;

use super::*;
use crate::events_script::{Action, Expect};

#[test]
fn a_click_is_a_left_press_and_release_at_the_point() {
    let calls = calls(&Action::Click(12.0, 34.0));
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0],
        (
            "Input.dispatchMouseEvent",
            json!({ "type": "mousePressed", "x": 12.0, "y": 34.0, "button": "left", "clickCount": 1 })
        )
    );
    assert_eq!(calls[1].1["type"], "mouseReleased");
}

#[test]
fn a_tap_is_a_touch_start_and_end() {
    let calls = calls(&Action::Tap(5.0, 6.0));
    assert_eq!(
        calls[0],
        (
            "Input.dispatchTouchEvent",
            json!({ "type": "touchStart", "touchPoints": [{ "x": 5.0, "y": 6.0 }] })
        )
    );
    assert_eq!(
        calls[1],
        (
            "Input.dispatchTouchEvent",
            json!({ "type": "touchEnd", "touchPoints": [] })
        )
    );
}

#[test]
fn a_key_with_modifiers_carries_their_bits_and_a_printable_key_its_text() {
    let shift_tab = calls(&Action::Key("Shift+Tab".into()));
    assert_eq!(shift_tab[0].1["modifiers"], 8);
    assert_eq!(shift_tab[0].1["key"], "Tab");
    let typed = calls(&Action::Type("ab".into()));
    assert_eq!(typed.len(), 4, "down and up for each character");
    assert_eq!(typed[0].1["text"], "a");
}

#[test]
fn a_wheel_carries_its_deltas_in_dom_signs() {
    let calls = calls(&Action::Wheel {
        x: 1.0,
        y: 2.0,
        dx: 0.0,
        dy: 120.0,
    });
    assert_eq!(
        calls,
        [(
            "Input.dispatchMouseEvent",
            json!({ "type": "mouseWheel", "x": 1.0, "y": 2.0, "deltaX": 0.0, "deltaY": 120.0 })
        )]
    );
}

#[test]
fn each_expectation_is_checked_against_the_steps_log_and_focus() {
    // Given
    let answer = json!({
        "events": [{ "type": "click", "target": "button#edit", "phase": 1 }],
        "focus": "button#edit",
        "focusVisible": false
    });

    // When / Then
    assert_eq!(
        unmet(
            &Expect::Event {
                kind: "click".into(),
                target: "button#edit".into()
            },
            &answer
        ),
        None
    );
    assert_eq!(unmet(&Expect::Focus("button#edit".into()), &answer), None);
    assert!(unmet(&Expect::NoEvent("click".into()), &answer).is_some());
    assert!(unmet(&Expect::FocusVisible("button#edit".into()), &answer).is_some());
    let wrong = unmet(
        &Expect::Event {
            kind: "click".into(),
            target: "input#title".into(),
        },
        &answer,
    );
    assert!(
        wrong.is_some_and(|w| w.contains("button#edit")),
        "names what was logged"
    );
}

#[test]
fn space_is_named_space_in_a_script_and_sent_as_a_space_with_its_text() {
    let space = calls(&Action::Key("Space".into()));
    assert_eq!(space[0].1["key"], " ");
    assert_eq!(space[0].1["text"], " ");
    assert_eq!(space[0].1["windowsVirtualKeyCode"], 32);
}

#[test]
fn enter_types_a_carriage_return_as_a_keyboard_does() {
    let enter = calls(&Action::Key("Enter".into()));
    assert_eq!(enter[0].1["text"], "\r");
    assert_eq!(enter[0].1["code"], "Enter");
}
