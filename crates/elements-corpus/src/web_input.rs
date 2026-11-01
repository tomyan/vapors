//! The web page's text input checked against Chromium's own `<input>`
//! (E6): the same `DevTools` input calls (typed text, an IME's
//! composition and its commit, keys) played on a plain `<input>` and on
//! the built page's `field` demo, and the text and selection after each
//! compared. Chromium's are the `<input>`'s own; vapors' are what the
//! page's hidden textarea last took from the app's field
//! (`data-vapors-text`, `data-vapors-selection`), so an edit the app
//! did not take shows.

use std::time::Duration;

use serde_json::{Value, json};
use vapors_devtools::Session;

use crate::events_input::{Pointer, calls};
use crate::events_script::Action;

/// One call's deadline: vapors' `COLD_START`, 145 s, about 2.5x the
/// slowest first launch on a runner; a warm browser answers in ms.
pub const DEADLINE: Duration = Duration::from_secs(145);

/// The field's state after a step: text, selection start and end.
pub type State = (String, u64, u64);

/// What each step is called, and its calls.
#[must_use]
pub fn steps() -> Vec<(&'static str, Vec<(&'static str, Value)>)> {
    let compose = |text: &str| {
        let at = text.encode_utf16().count();
        vec![(
            "Input.imeSetComposition",
            json!({ "text": text, "selectionStart": at, "selectionEnd": at }),
        )]
    };
    let insert = |text: &str| vec![("Input.insertText", json!({ "text": text }))];
    let key = |name: &str| calls(&Action::Key(name.to_owned()), &mut Pointer::default());
    vec![
        ("typed th", insert("th")),
        ("composing c", compose("c")),
        ("composing co", compose("co")),
        ("composition committed as cool", insert("cool")),
        ("Backspace", key("Backspace")),
        ("ArrowLeft", key("ArrowLeft")),
        ("composing x in the middle", compose("x")),
        ("committed as x", insert("x")),
        ("Shift+Home", key("Shift+Home")),
        ("typed over the selection", insert("A")),
    ]
}

/// Plays every step on the page open in `session`, reading the field
/// with `read` (an expression for `[text, start, end]`) after each.
///
/// # Errors
/// A call that fails, or a read of another shape.
pub fn play(session: &mut Session, read: &str) -> Result<Vec<(String, State)>, String> {
    let mut states = Vec::new();
    for (name, step) in steps() {
        for (method, params) in step {
            session.call(method, &params, DEADLINE)?;
        }
        settle(session, read)?;
        states.push((name.to_owned(), state(session, read)?));
    }
    Ok(states)
}

/// The field's state now.
///
/// # Errors
/// A read that throws or is not `[text, start, end]`.
pub fn state(session: &mut Session, read: &str) -> Result<State, String> {
    let value = evaluate(session, read)?;
    let shape = || format!("{read} gave {value}, not [text, start, end]");
    let text = value[0].as_str().ok_or_else(shape)?.to_owned();
    let start = value[1].as_u64().ok_or_else(shape)?;
    let end = value[2].as_u64().ok_or_else(shape)?;
    Ok((text, start, end))
}

/// Waits until the field's state (`read`) has held for [`QUIET_MS`],
/// polling every 10 ms, for at most 5 s (over 30x the slowest): the page's loop takes an input
/// and mirrors the field after the call returns.
fn settle(session: &mut Session, read: &str) -> Result<(), String> {
    let wait = format!(
        "new Promise((done, fail) => {{ const read = () => JSON.stringify({read}); let last = read(); let since = performance.now(); const start = since; const poll = () => {{ const now = read(); const at = performance.now(); if (now !== last) {{ last = now; since = at; }} if (at - since >= {QUIET_MS}) {{ done(true); }} else if (at - start > 5000) {{ fail(new Error('the field did not settle in 5 s')); }} else {{ setTimeout(poll, 10); }} }}; poll(); }})"
    );
    evaluate(session, &wait).map(drop)
}

/// How long the state must hold: 2x the slowest change after a step's
/// calls began, 147.5 ms (Shift+Home, three runs on the dev box with
/// a warm Chrome, 2026-11-01; most steps changed within 25 ms).
const QUIET_MS: u32 = 300;

/// `expression`'s value, awaited; a throw is refused with its text.
///
/// # Errors
/// The call fails or the page throws.
pub fn evaluate(session: &mut Session, expression: &str) -> Result<Value, String> {
    let params = json!({ "expression": expression, "awaitPromise": true, "returnByValue": true });
    let mut result = session.call("Runtime.evaluate", &params, DEADLINE)?;
    if let Some(exception) = result.get("exceptionDetails") {
        return Err(format!("{expression}: the page threw {exception}"));
    }
    Ok(result["result"]["value"].take())
}

/// Each step whose state differs, as a line naming both.
#[must_use]
pub fn differences(chromium: &[(String, State)], vapors: &[(String, State)]) -> Vec<String> {
    chromium
        .iter()
        .zip(vapors)
        .filter(|(theirs, ours)| theirs != ours)
        .map(|((step, theirs), (_, ours))| {
            format!("after {step}: Chromium's <input> holds {theirs:?}, vapors' field {ours:?}")
        })
        .collect()
}
