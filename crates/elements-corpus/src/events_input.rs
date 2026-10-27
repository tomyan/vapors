//! An `.events` action as `DevTools` `Input` calls (an expectation is
//! checked against what Chromium logged by `events_unmet`). `frame`
//! makes no Input call: the player waits for two animation frames
//! instead.

use serde_json::{Value, json};

use crate::events_script::Action;

/// `Input` modifier bits, by the names `key` takes.
const MODIFIER_BITS: [(&str, u8); 4] = [("Alt", 1), ("Control", 2), ("Meta", 4), ("Shift", 8)];

/// Named keys: what Chromium calls each, its `code`, and its Windows
/// virtual key code, which Chromium needs to act on it (Tab to move
/// focus); Enter types a carriage return, which activates a button.
const NAMED_KEYS: [(&str, &str, &str, u8); 14] = [
    ("Backspace", "Backspace", "Backspace", 8),
    ("Tab", "Tab", "Tab", 9),
    ("Enter", "Enter", "Enter", 13),
    ("Escape", "Escape", "Escape", 27),
    ("Space", " ", "Space", 32),
    ("PageUp", "PageUp", "PageUp", 33),
    ("PageDown", "PageDown", "PageDown", 34),
    ("End", "End", "End", 35),
    ("Home", "Home", "Home", 36),
    ("ArrowLeft", "ArrowLeft", "ArrowLeft", 37),
    ("ArrowUp", "ArrowUp", "ArrowUp", 38),
    ("ArrowRight", "ArrowRight", "ArrowRight", 39),
    ("ArrowDown", "ArrowDown", "ArrowDown", 40),
    ("Delete", "Delete", "Delete", 46),
];

/// Whether the left button is held, between a `press` and a `release`.
#[derive(Default)]
pub struct Pointer {
    held: bool,
}

/// The `Input` calls that perform `action`, in order.
pub fn calls(action: &Action, pointer: &mut Pointer) -> Vec<(&'static str, Value)> {
    match action {
        Action::Click(x, y) => vec![
            mouse("mousePressed", *x, *y),
            mouse("mouseReleased", *x, *y),
        ],
        Action::Press(x, y) => {
            pointer.held = true;
            vec![mouse("mousePressed", *x, *y)]
        }
        Action::Release(x, y) => {
            pointer.held = false;
            vec![mouse("mouseReleased", *x, *y)]
        }
        Action::Move(x, y) => vec![moved(*x, *y, pointer.held)],
        Action::Tap(x, y) => tap(*x, *y),
        Action::TouchDown(x, y) => vec![touch("touchStart", Some((*x, *y)))],
        Action::TouchMove(x, y) => vec![touch("touchMove", Some((*x, *y)))],
        Action::TouchUp(..) => vec![touch("touchEnd", None)],
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

fn mouse(kind: &str, x: f64, y: f64) -> (&'static str, Value) {
    (
        "Input.dispatchMouseEvent",
        json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 }),
    )
}

fn moved(x: f64, y: f64, held: bool) -> (&'static str, Value) {
    let (button, buttons) = if held { ("left", 1) } else { ("none", 0) };
    (
        "Input.dispatchMouseEvent",
        json!({ "type": "mouseMoved", "x": x, "y": y, "button": button, "buttons": buttons }),
    )
}

/// One finger's touch event: where it is, or none once it lifted.
fn touch(kind: &str, at: Option<(f64, f64)>) -> (&'static str, Value) {
    let points: Vec<Value> = at
        .map(|(x, y)| json!({ "x": x, "y": y }))
        .into_iter()
        .collect();
    (
        "Input.dispatchTouchEvent",
        json!({ "type": kind, "touchPoints": points }),
    )
}

fn tap(x: f64, y: f64) -> Vec<(&'static str, Value)> {
    vec![
        (
            "Input.dispatchTouchEvent",
            json!({ "type": "touchStart", "touchPoints": [{ "x": x, "y": y }] }),
        ),
        (
            "Input.dispatchTouchEvent",
            json!({ "type": "touchEnd", "touchPoints": [] }),
        ),
    ]
}

/// A key down and up; `+`-joined modifiers set their bits.
fn key_calls(combination: &str) -> Vec<(&'static str, Value)> {
    let parts: Vec<&str> = combination.split('+').collect();
    let (modifiers, key) = parts.split_at(parts.len().saturating_sub(1));
    let bits: u8 = MODIFIER_BITS
        .iter()
        .filter(|(name, _)| modifiers.contains(name))
        .map(|(_, bit)| bit)
        .sum();
    let mut down = described(key.first().copied().unwrap_or_default(), bits);
    down["type"] = json!("keyDown");
    let mut up = down.clone();
    up["type"] = json!("keyUp");
    up.as_object_mut().map(|o| o.remove("text"));
    vec![
        ("Input.dispatchKeyEvent", down),
        ("Input.dispatchKeyEvent", up),
    ]
}

/// The key's fields, only those it has: `key`, `code`, the virtual key
/// code, and the text a printable key types (a capital with Shift).
fn described(name: &str, bits: u8) -> Value {
    if let Some((_, key, code, virtual_code)) = NAMED_KEYS.iter().find(|(n, ..)| *n == name) {
        let mut fields = json!({ "key": key, "code": code, "windowsVirtualKeyCode": virtual_code, "modifiers": bits });
        let typed = match name {
            "Space" => Some(" "),
            "Enter" => Some("\r"),
            _ => None,
        };
        if let (Some(text), 0) = (typed, bits) {
            fields["text"] = json!(text);
        }
        return fields;
    }
    character(name, bits)
}

fn character(name: &str, bits: u8) -> Value {
    let shifted = bits & 8 != 0;
    let key: String = if shifted {
        name.to_uppercase()
    } else {
        name.to_owned()
    };
    let mut fields = json!({ "key": key, "modifiers": bits });
    let c = name.chars().next().unwrap_or_default();
    if c.is_ascii_alphanumeric() {
        fields["windowsVirtualKeyCode"] = json!(c.to_ascii_uppercase() as u32);
        let code = if c.is_ascii_digit() {
            format!("Digit{c}")
        } else {
            format!("Key{}", c.to_ascii_uppercase())
        };
        fields["code"] = json!(code);
    }
    if bits & !8 == 0 {
        fields["text"] = json!(key);
    }
    fields
}

pub use crate::events_unmet::unmet;

#[cfg(test)]
#[path = "events_input_tests.rs"]
mod tests;
