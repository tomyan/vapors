//! Actions as `DevTools` `Input` calls, and expectations checked against
//! what Chromium logged.

use serde_json::json;

use super::*;

/// The calls for one action with no button held before it.
fn calls_fresh(action: &Action) -> Vec<(&'static str, serde_json::Value)> {
    calls(action, &mut Pointer::default())
}
use crate::events_script::{Action, Expect};

#[test]
fn a_click_is_a_left_press_and_release_at_the_point() {
    let calls = calls_fresh(&Action::Click(12.0, 34.0));
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
    let calls = calls_fresh(&Action::Tap(5.0, 6.0));
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
fn a_key_with_modifiers_carries_their_bits() {
    let shift_tab = calls_fresh(&Action::Key("Shift+Tab".into()));
    assert_eq!(shift_tab[0].1["modifiers"], 8);
    assert_eq!(shift_tab[0].1["key"], "Tab");
}

#[test]
fn typing_is_a_down_and_up_per_character_each_with_its_text() {
    let typed = calls_fresh(&Action::Type("ab".into()));
    assert_eq!(typed.len(), 4);
    assert_eq!(typed[0].1["text"], "a");
}

#[test]
fn shift_with_a_letter_sends_the_capital_as_a_keyboard_does() {
    let capital = calls_fresh(&Action::Key("Shift+a".into()));
    assert_eq!(capital[0].1["key"], "A");
    assert_eq!(capital[0].1["text"], "A");
}

#[test]
fn a_key_up_carries_only_the_fields_the_key_has() {
    let comma = calls_fresh(&Action::Key(",".into()));
    assert!(comma[1].1.get("code").is_none(), "{}", comma[1].1);
    assert!(
        comma[1].1.get("windowsVirtualKeyCode").is_none(),
        "{}",
        comma[1].1
    );
}

#[test]
fn a_move_between_press_and_release_holds_the_button() {
    let mut pointer = Pointer::default();
    calls(&Action::Press(1.0, 1.0), &mut pointer);
    let dragged = calls(&Action::Move(9.0, 1.0), &mut pointer);
    calls(&Action::Release(9.0, 1.0), &mut pointer);
    let hovered = calls(&Action::Move(5.0, 5.0), &mut pointer);
    assert_eq!(
        (
            dragged[0].1["button"].clone(),
            dragged[0].1["buttons"].clone()
        ),
        (json!("left"), json!(1))
    );
    assert_eq!(
        (
            hovered[0].1["button"].clone(),
            hovered[0].1["buttons"].clone()
        ),
        (json!("none"), json!(0))
    );
}

#[test]
fn a_wheel_carries_its_deltas_in_dom_signs() {
    let calls = calls_fresh(&Action::Wheel {
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
    let space = calls_fresh(&Action::Key("Space".into()));
    assert_eq!(space[0].1["key"], " ");
    assert_eq!(space[0].1["text"], " ");
    assert_eq!(space[0].1["windowsVirtualKeyCode"], 32);
}

#[test]
fn enter_types_a_carriage_return_as_a_keyboard_does() {
    let enter = calls_fresh(&Action::Key("Enter".into()));
    assert_eq!(enter[0].1["text"], "\r");
    assert_eq!(enter[0].1["code"], "Enter");
}
