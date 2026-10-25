//! One action line of a `.events` script (`events_script`).

use crate::events_script::Action;

const MODIFIERS: [&str; 4] = ["Shift", "Control", "Alt", "Meta"];

pub fn is_action(word: &str) -> bool {
    [
        "click", "tap", "press", "release", "move", "wheel", "key", "type", "frame",
    ]
    .contains(&word)
}

pub fn action(word: &str, rest: &str) -> Result<Action, String> {
    match word {
        "click" => point(word, rest).map(|(x, y)| Action::Click(x, y)),
        "tap" => point(word, rest).map(|(x, y)| Action::Tap(x, y)),
        "press" => point(word, rest).map(|(x, y)| Action::Press(x, y)),
        "release" => point(word, rest).map(|(x, y)| Action::Release(x, y)),
        "move" => point(word, rest).map(|(x, y)| Action::Move(x, y)),
        "wheel" => wheel(rest),
        "key" => key(rest),
        "type" => quoted(rest).map(Action::Type),
        "frame" => Ok(Action::Frame),
        _ => Err(format!("no directive `{word}`")),
    }
}

fn numbers(rest: &str, count: usize) -> Option<Vec<f64>> {
    let values: Vec<f64> = rest
        .split_whitespace()
        .take(count)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (values.len() == count).then_some(values)
}

fn point(word: &str, rest: &str) -> Result<(f64, f64), String> {
    numbers(rest, 2)
        .map(|v| (v[0], v[1]))
        .ok_or_else(|| format!("`{word}` takes X Y"))
}

fn wheel(rest: &str) -> Result<Action, String> {
    let v = numbers(rest, 4).ok_or("`wheel` takes X Y DX DY")?;
    Ok(Action::Wheel {
        x: v[0],
        y: v[1],
        dx: v[2],
        dy: v[3],
    })
}

fn key(rest: &str) -> Result<Action, String> {
    let key = rest.split_whitespace().next().ok_or("`key` takes a key")?;
    let parts: Vec<&str> = key.split('+').collect();
    let (modifiers, _key) = parts.split_at(parts.len() - 1);
    if let Some(unknown) = modifiers.iter().find(|m| !MODIFIERS.contains(m)) {
        return Err(format!(
            "no modifier `{unknown}`; use {}",
            MODIFIERS.join(", ")
        ));
    }
    Ok(Action::Key(key.to_owned()))
}

fn quoted(rest: &str) -> Result<String, String> {
    let inner = rest
        .strip_prefix('"')
        .and_then(|r| r.split_once('"'))
        .map(|(text, _)| text);
    inner
        .map(str::to_owned)
        .ok_or_else(|| "`type` takes a quoted text".to_owned())
}
