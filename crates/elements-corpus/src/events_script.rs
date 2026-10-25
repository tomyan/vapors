//! The `.events` input scripts (docs/design/11-testing-oracles.md, Q1):
//! `fixtures/<name>.events` beside `<name>.wisp`, played in Chromium over
//! the `DevTools` protocol. One directive a line, `#` comments, a note
//! after the fields, as `.pixels` has them:
//!
//! - `at WxH`: the viewport, at most once.
//! - `listen <type>…`: the event types logged, once, before any action.
//! - Actions: `click X Y`, `tap X Y`, `press X Y`, `release X Y`,
//!   `move X Y`, `wheel X Y DX DY` (CSS px, DOM signs), `key <Key>` with
//!   `+`-joined modifiers (`Shift+Tab`), `type "<text>"`, `frame`.
//! - After an action, what must hold once it is done:
//!   `expect event <type> <target>`, `expect no event <type>` (a type
//!   that is listened for), `expect focus <target>`,
//!   `expect focus-visible <target>`. A target is `tag#id`, else the
//!   full child-index path from body.
//!
//! Anything else is refused, naming the file and line.

use crate::events_action::{action, is_action};

/// A parsed script.
#[derive(Debug, Default)]
pub struct Script {
    /// `at`, when given.
    pub viewport: Option<(u32, u32)>,
    /// `listen`'s event types.
    pub listen: Vec<String>,
    /// The actions, each with the expectations after it.
    pub steps: Vec<Step>,
}

/// One action and what must hold after it.
#[derive(Debug)]
pub struct Step {
    /// What is done.
    pub action: Action,
    /// What must hold once it is, each with its line.
    pub expect: Vec<(Expect, usize)>,
    /// Its line, from 1.
    pub line: usize,
}

/// An input, in CSS px.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Press and release the left button.
    Click(f64, f64),
    /// A touch's start and end.
    Tap(f64, f64),
    /// Press the left button.
    Press(f64, f64),
    /// Release the left button.
    Release(f64, f64),
    /// Move the pointer.
    Move(f64, f64),
    /// A wheel turn at x, y by dx, dy.
    Wheel {
        /// Where, across.
        x: f64,
        /// Where, down.
        y: f64,
        /// How far across, positive to the right.
        dx: f64,
        /// How far down, positive down.
        dy: f64,
    },
    /// A key, with `+`-joined modifiers.
    Key(String),
    /// Text, one key per character.
    Type(String),
    /// Wait for the next painted frame.
    Frame,
}

/// What must hold after an action.
#[derive(Debug, PartialEq)]
pub enum Expect {
    /// An event of `kind` at `target` was logged.
    Event {
        /// The event type.
        kind: String,
        /// Where it was dispatched.
        target: String,
    },
    /// No event of this type was logged.
    NoEvent(String),
    /// This element has focus.
    Focus(String),
    /// This element has focus and matches `:focus-visible`.
    FocusVisible(String),
}

/// The script `text` of fixture `name`.
///
/// # Errors
/// The first line of another shape, as `<name>.events:<line>: <why>`.
pub fn parse(name: &str, text: &str) -> Result<Script, String> {
    let mut script = Script::default();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        directive(&mut script, line, index + 1)
            .map_err(|why| format!("{name}.events:{}: {why}: {line}", index + 1))?;
    }
    Ok(script)
}

fn directive(script: &mut Script, line: &str, number: usize) -> Result<(), String> {
    let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    match word {
        "at" if script.viewport.is_some() => Err("a second `at`".into()),
        "at" => at(rest).map(|size| script.viewport = Some(size)),
        "listen" if !script.listen.is_empty() => Err("a second `listen`".into()),
        "listen" => listen(rest).map(|types| script.listen = types),
        "expect" => expect(script, rest, number),
        _ if script.listen.is_empty() && is_action(word) => Err("an action before `listen`".into()),
        _ => action(word, rest).map(|action| {
            script.steps.push(Step {
                action,
                expect: Vec::new(),
                line: number,
            });
        }),
    }
}

fn at(rest: &str) -> Result<(u32, u32), String> {
    let size = rest.split_whitespace().next().unwrap_or_default();
    let parsed = size
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
    parsed.ok_or_else(|| "`at` takes WxH".to_owned())
}

fn listen(rest: &str) -> Result<Vec<String>, String> {
    let types: Vec<String> = rest.split_whitespace().map(str::to_owned).collect();
    if types.is_empty() {
        return Err("`listen` takes event types".into());
    }
    Ok(types)
}

fn expect(script: &mut Script, rest: &str, number: usize) -> Result<(), String> {
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let expectation = match fields.as_slice() {
        ["event", kind, target, ..] => Expect::Event { kind: (*kind).into(), target: (*target).into() },
        ["no", "event", kind, ..] if !script.listen.iter().any(|t| t == kind) => {
            return Err(format!("`{kind}` is not listened for, so expecting none of it proves nothing"));
        }
        ["no", "event", kind, ..] => Expect::NoEvent((*kind).into()),
        ["focus", target, ..] => Expect::Focus((*target).into()),
        ["focus-visible", target, ..] => Expect::FocusVisible((*target).into()),
        _ => return Err("`expect` takes event <type> <target>, no event <type>, focus <target> or focus-visible <target>".into()),
    };
    let step = script
        .steps
        .last_mut()
        .ok_or("an expectation before any action")?;
    step.expect.push((expectation, number));
    Ok(())
}

#[cfg(test)]
#[path = "events_script_tests.rs"]
mod tests;
