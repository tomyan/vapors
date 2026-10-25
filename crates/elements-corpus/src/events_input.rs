//! An `.events` action as `DevTools` `Input` calls, and an expectation
//! checked against what Chromium logged after it. `frame` makes no Input
//! call: the player waits for two animation frames instead.

use serde_json::{Value, json};

use crate::events_script::{Action, Expect};

/// `Input` modifier bits, by the names `key` takes.
const MODIFIER_BITS: [(&str, u8); 4] = [("Alt", 1), ("Control", 2), ("Meta", 4), ("Shift", 8)];

/// Named keys and their Windows virtual key codes, which Chromium needs
/// to act on them (Tab to move focus, Enter to activate).
const NAMED_KEYS: [(&str, u8); 13] = [
    ("Backspace", 8),
    ("Tab", 9),
    ("Enter", 13),
    ("Escape", 27),
    (" ", 32),
    ("Space", 32),
    ("End", 35),
    ("Home", 36),
    ("ArrowLeft", 37),
    ("ArrowUp", 38),
    ("ArrowRight", 39),
    ("ArrowDown", 40),
    ("Delete", 46),
];

/// The `Input` calls that perform `action`, in order.
#[must_use]
pub fn calls(action: &Action) -> Vec<(&'static str, Value)> {
    let mouse = |kind: &str, x: f64, y: f64| {
        (
            "Input.dispatchMouseEvent",
            json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 }),
        )
    };
    match action {
        Action::Click(x, y) => vec![
            mouse("mousePressed", *x, *y),
            mouse("mouseReleased", *x, *y),
        ],
        Action::Press(x, y) => vec![mouse("mousePressed", *x, *y)],
        Action::Release(x, y) => vec![mouse("mouseReleased", *x, *y)],
        Action::Move(x, y) => vec![(
            "Input.dispatchMouseEvent",
            json!({ "type": "mouseMoved", "x": x, "y": y }),
        )],
        Action::Tap(x, y) => vec![
            (
                "Input.dispatchTouchEvent",
                json!({ "type": "touchStart", "touchPoints": [{ "x": x, "y": y }] }),
            ),
            (
                "Input.dispatchTouchEvent",
                json!({ "type": "touchEnd", "touchPoints": [] }),
            ),
        ],
        Action::Wheel { x, y, dx, dy } => {
            vec![(
                "Input.dispatchMouseEvent",
                json!({ "type": "mouseWheel", "x": x, "y": y, "deltaX": dx, "deltaY": dy }),
            )]
        }
        Action::Key(key) => key_calls(key),
        Action::Type(text) => text
            .chars()
            .flat_map(|c| key_calls(&c.to_string()))
            .collect(),
        Action::Frame => Vec::new(),
    }
}

/// A key down and up; `+`-joined modifiers set their bits.
fn key_calls(combination: &str) -> Vec<(&'static str, Value)> {
    let parts: Vec<&str> = combination.split('+').collect();
    let (modifiers, key) = parts.split_at(parts.len().saturating_sub(1));
    // `Space` names the key the script cannot write bare; Chromium calls it " ".
    let key = match key.first().copied().unwrap_or_default() {
        "Space" => " ",
        key => key,
    };
    let bits: u8 = MODIFIER_BITS
        .iter()
        .filter(|(name, _)| modifiers.contains(name))
        .map(|(_, bit)| bit)
        .sum();
    let mut down = json!({ "type": "keyDown", "key": key, "modifiers": bits });
    if let Some(code) = virtual_code(key) {
        down["windowsVirtualKeyCode"] = json!(code);
    }
    if let Some(code) = code_of(key) {
        down["code"] = json!(code);
    }
    if key.chars().count() == 1 && bits & !8 == 0 {
        down["text"] = json!(key);
    }
    // Enter types a carriage return, which is what activates a button.
    if key == "Enter" && bits == 0 {
        down["text"] = json!("\r");
    }
    let up = json!({ "type": "keyUp", "key": key, "modifiers": bits, "windowsVirtualKeyCode": down["windowsVirtualKeyCode"], "code": down["code"] });
    vec![
        ("Input.dispatchKeyEvent", down),
        ("Input.dispatchKeyEvent", up),
    ]
}

/// The physical key's `code`: Chromium activates a button on Space's keyup
/// only when the key event names it.
fn code_of(key: &str) -> Option<String> {
    let named = [
        (" ", "Space"),
        ("Enter", "Enter"),
        ("Tab", "Tab"),
        ("Escape", "Escape"),
        ("Backspace", "Backspace"),
    ];
    if let Some((_, code)) = named.iter().find(|(name, _)| *name == key) {
        return Some((*code).to_owned());
    }
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => {
            Some(format!("Key{}", c.to_ascii_uppercase()))
        }
        (Some(c), None) if c.is_ascii_digit() => Some(format!("Digit{c}")),
        _ => None,
    }
}

fn virtual_code(key: &str) -> Option<u8> {
    if let Some((_, code)) = NAMED_KEYS.iter().find(|(name, _)| *name == key) {
        return Some(*code);
    }
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => u8::try_from(c.to_ascii_uppercase()).ok(),
        _ => None,
    }
}

/// Why `expect` does not hold for a step's `answer`, or `None` when it does.
#[must_use]
pub fn unmet(expect: &Expect, answer: &Value) -> Option<String> {
    let events = answer["events"].as_array().cloned().unwrap_or_default();
    let logged = || {
        events
            .iter()
            .map(|e| {
                format!(
                    "{} at {}",
                    e["type"].as_str().unwrap_or("?"),
                    e["target"].as_str().unwrap_or("?")
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let focus = answer["focus"].as_str().unwrap_or("(nothing)");
    match expect {
        Expect::Event { kind, target } => {
            let found = events
                .iter()
                .any(|e| e["type"] == kind.as_str() && e["target"] == target.as_str());
            (!found).then(|| format!("no {kind} at {target}; logged: [{}]", logged()))
        }
        Expect::NoEvent(kind) => events
            .iter()
            .any(|e| e["type"] == kind.as_str())
            .then(|| format!("a {kind} was logged: [{}]", logged())),
        Expect::Focus(target) => {
            (focus != target).then(|| format!("focus is on {focus}, not {target}"))
        }
        Expect::FocusVisible(target) => {
            let visible = focus == target && answer["focusVisible"] == true;
            (!visible).then(|| {
                format!(
                    "{target} does not show focus-visible; focus is on {focus}, focus-visible {}",
                    answer["focusVisible"]
                )
            })
        }
    }
}

#[cfg(test)]
#[path = "events_input_tests.rs"]
mod tests;
