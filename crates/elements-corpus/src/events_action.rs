//! One action line of a `.events` script (`events_script`): its fields,
//! exactly as many as the action takes, numbers finite, keys named.

use crate::events_script::Action;

const MODIFIERS: [&str; 4] = ["Shift", "Control", "Alt", "Meta"];

/// Keys a script may name besides a single printable character.
pub const KEY_NAMES: [&str; 14] = [
    "Backspace",
    "Tab",
    "Enter",
    "Escape",
    "Space",
    "PageUp",
    "PageDown",
    "End",
    "Home",
    "ArrowLeft",
    "ArrowUp",
    "ArrowRight",
    "ArrowDown",
    "Delete",
];

pub fn is_action(word: &str) -> bool {
    [
        "click",
        "tap",
        "press",
        "release",
        "move",
        "wheel",
        "key",
        "type",
        "frame",
        "touch-down",
        "touch-move",
        "touch-up",
    ]
    .contains(&word)
}

/// The action `word` with its `fields`.
pub fn action(word: &str, fields: &[&str], rest: &str) -> Result<Action, String> {
    match word {
        "click" => point(word, fields).map(|(x, y)| Action::Click(x, y)),
        "tap" => point(word, fields).map(|(x, y)| Action::Tap(x, y)),
        "press" => point(word, fields).map(|(x, y)| Action::Press(x, y)),
        "release" => point(word, fields).map(|(x, y)| Action::Release(x, y)),
        "move" => point(word, fields).map(|(x, y)| Action::Move(x, y)),
        "touch-down" => point(word, fields).map(|(x, y)| Action::TouchDown(x, y)),
        "touch-move" => point(word, fields).map(|(x, y)| Action::TouchMove(x, y)),
        "touch-up" => point(word, fields).map(|(x, y)| Action::TouchUp(x, y)),
        "wheel" => wheel(fields),
        "key" => exactly(word, fields, 1).and_then(|()| key(fields[0])),
        "type" => quoted(rest).map(Action::Type),
        "frame" => exactly(word, fields, 0).map(|()| Action::Frame),
        _ => Err(format!("no directive `{word}`")),
    }
}

/// `count` fields, or a refusal: too few is a misuse, more are extra fields.
pub fn exactly(word: &str, fields: &[&str], count: usize) -> Result<(), String> {
    if fields.len() > count {
        return Err(format!(
            "extra fields after `{word}`; a note follows two spaces"
        ));
    }
    Ok(())
}

/// `count` finite numbers.
fn numbers(word: &str, fields: &[&str], count: usize) -> Option<Vec<f64>> {
    exactly(word, fields, count).ok()?;
    let values: Vec<f64> = fields
        .iter()
        .map(|f| f.parse::<f64>())
        .collect::<Result<_, _>>()
        .ok()?;
    (values.len() == count && values.iter().all(|v| v.is_finite())).then_some(values)
}

fn point(word: &str, fields: &[&str]) -> Result<(f64, f64), String> {
    exactly(word, fields, 2)?;
    numbers(word, fields, 2)
        .map(|v| (v[0], v[1]))
        .ok_or_else(|| format!("`{word}` takes X Y"))
}

fn wheel(fields: &[&str]) -> Result<Action, String> {
    exactly("wheel", fields, 4)?;
    let v = numbers("wheel", fields, 4).ok_or("`wheel` takes X Y DX DY")?;
    Ok(Action::Wheel {
        x: v[0],
        y: v[1],
        dx: v[2],
        dy: v[3],
    })
}

/// A key, `+`-joined modifiers first; the key named or one character.
fn key(combination: &str) -> Result<Action, String> {
    let parts: Vec<&str> = combination.split('+').collect();
    let (modifiers, key) = parts.split_at(parts.len() - 1);
    if let Some(unknown) = modifiers.iter().find(|m| !MODIFIERS.contains(m)) {
        return Err(format!(
            "no modifier `{unknown}`; use {}",
            MODIFIERS.join(", ")
        ));
    }
    let key = key.first().copied().unwrap_or_default();
    if !KEY_NAMES.contains(&key) && key.chars().count() != 1 {
        return Err(format!(
            "no key `{key}`; use one character or {}",
            KEY_NAMES.join(", ")
        ));
    }
    Ok(Action::Key(combination.to_owned()))
}

/// A quoted, non-empty text.
fn quoted(rest: &str) -> Result<String, String> {
    let refused = || "`type` takes a quoted text".to_owned();
    let (text, after) = rest
        .strip_prefix('"')
        .and_then(|r| r.split_once('"'))
        .ok_or_else(refused)?;
    if text.is_empty() {
        return Err(refused());
    }
    if !after.is_empty() && !after.starts_with("  ") {
        return Err("extra fields after `type`; a note follows two spaces".into());
    }
    Ok(text.to_owned())
}
