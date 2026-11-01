//! The `.events` input scripts (docs/design/11-testing-oracles.md, Q1):
//! `fixtures/<name>.events` beside `<name>.wisp`, played in Chromium over
//! the `DevTools` protocol. One directive a line, `#` comments, a note
//! after the fields, as `.pixels` has them:
//!
//! - `at WxH`: the viewport, at most once.
//! - `listen <type>…`: the event types logged, once, before any action.
//! - `capture <target>…`: elements that capture the pointer on
//!   `pointerdown` (`setPointerCapture` in Chromium), at most once.
//! - `cancel <target>…`: elements whose `pointerdown` listener prevents
//!   its default, at most once.
//! - `page <file>`: Chromium plays the script on `fixtures/<file>` in
//!   place of the fixture's page, at most once.
//! - `project <role>…`: after each step, the a11y projection of every
//!   node with one of these roles (`ax_projection`), at most once.
//! - Touch: `touch-down X Y`, `touch-move X Y`, `touch-up X Y`, one finger.
//! - Actions: `click X Y`, `double-click X Y`, `triple-click X Y`,
//!   `tap X Y`, `press X Y`, `release X Y`, `move X Y`,
//!   `wheel X Y DX DY` (CSS px, DOM signs), `key <Key>` with
//!   `+`-joined modifiers (`Shift+Tab`), `type "<text>"`, `frame`.
//! - After an action, what must hold once it is done:
//!   `expect event <type> <target>`, `expect no event <type>` (a type
//!   that is listened for), `expect focus <target>`,
//!   `expect focus-visible <target>`, `expect value <target> <text>` (a
//!   control's `value`; quoted when empty or holding spaces, `"a b"`),
//!   `expect checked <target>
//!   true|false`, `expect selection <target> <start> <end>` (a text
//!   field's selection, UTF-16 units). A target is `tag#id`, else the
//!   full child-index path from body.
//!
//! Anything else is refused, naming the file and line.

use crate::events_action::{action, exactly, is_action};
use crate::events_expect::expect;
use crate::events_fields::{at, listen, page};

/// A parsed script.
#[derive(Debug, Default)]
pub struct Script {
    /// `at`, when given.
    pub viewport: Option<(u32, u32)>,
    /// `listen`'s event types.
    pub listen: Vec<String>,
    /// `capture`'s targets: each captures the pointer on `pointerdown`.
    pub capture: Vec<String>,
    /// `cancel`'s targets: each prevents its `pointerdown`'s default.
    pub cancel: Vec<String>,
    /// `page`: the file under `fixtures/` Chromium plays the script on,
    /// in place of the fixture's own page (another page's own script,
    /// such as APG's example, as the browser's answer).
    pub page: Option<String>,
    /// `project`'s roles: after each step, the a11y projection of every
    /// node with one of them.
    pub project: Vec<String>,
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

pub use crate::events_steps::{Action, Expect};

impl Script {
    /// The controls whose `value` the script expects somewhere, sorted
    /// and each once: every step records each one's value.
    #[must_use]
    pub fn value_targets(&self) -> Vec<String> {
        self.targets(|expect| match expect {
            Expect::Value { target, .. } => Some(target),
            _ => None,
        })
    }

    /// The checkboxes whose `checked` the script expects somewhere,
    /// sorted and each once: every step records each one's state.
    #[must_use]
    pub fn checked_targets(&self) -> Vec<String> {
        self.targets(|expect| match expect {
            Expect::Checked { target, .. } => Some(target),
            _ => None,
        })
    }

    fn targets(&self, pick: fn(&Expect) -> Option<&String>) -> Vec<String> {
        let mut targets: Vec<String> = self
            .steps
            .iter()
            .flat_map(|step| step.expect.iter())
            .filter_map(|(expect, _)| pick(expect).cloned())
            .collect();
        targets.sort();
        targets.dedup();
        targets
    }
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
    checked(name, script)
}

/// A script with steps, each with an expectation: one that checks nothing
/// would pass whatever Chromium did.
fn checked(name: &str, script: Script) -> Result<Script, String> {
    if script.steps.is_empty() {
        return Err(format!(
            "{name}.events: no steps; a script plays at least one action"
        ));
    }
    if let Some(step) = script
        .steps
        .iter()
        .find(|s| s.expect.is_empty() && s.action != Action::Frame)
    {
        return Err(format!(
            "{name}.events:{}: a step with no expectation",
            step.line
        ));
    }
    Ok(script)
}

/// The fields before a note: a note follows two spaces outside quotes.
fn fields_part(line: &str) -> &str {
    let mut quoted = false;
    for (at, c) in line.char_indices() {
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted && line[at..].starts_with("  ") {
            return &line[..at];
        }
    }
    line
}

fn directive(script: &mut Script, line: &str, number: usize) -> Result<(), String> {
    let line = fields_part(line);
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    let fields: Vec<&str> = rest.split(' ').filter(|f| !f.is_empty()).collect();
    match word {
        "at" if script.viewport.is_some() => Err("a second `at`".into()),
        "at" => exactly(word, &fields, 1)
            .and_then(|()| at(rest))
            .map(|size| script.viewport = Some(size)),
        "listen" if !script.listen.is_empty() => Err("a second `listen`".into()),
        "listen" => listen(rest).map(|types| script.listen = types),
        "cancel" if !script.cancel.is_empty() => Err("a second `cancel`".into()),
        "cancel" if fields.is_empty() => Err("`cancel` takes targets".into()),
        "cancel" => {
            script.cancel = fields.iter().map(|f| (*f).to_owned()).collect();
            Ok(())
        }
        "capture" if !script.capture.is_empty() => Err("a second `capture`".into()),
        "capture" if fields.is_empty() => Err("`capture` takes targets".into()),
        "capture" => {
            script.capture = fields.iter().map(|f| (*f).to_owned()).collect();
            Ok(())
        }
        "page" if script.page.is_some() => Err("a second `page`".into()),
        "page" => exactly(word, &fields, 1)
            .and_then(|()| page(fields[0]))
            .map(|file| script.page = Some(file)),
        "project" if !script.project.is_empty() => Err("a second `project`".into()),
        "project" if fields.is_empty() => Err("`project` takes roles".into()),
        "project" => {
            script.project = fields.iter().map(|f| (*f).to_owned()).collect();
            Ok(())
        }
        "expect" => expect(script, rest, &fields, number),
        _ if script.listen.is_empty() && is_action(word) => Err("an action before `listen`".into()),
        _ => action(word, &fields, rest).map(|action| {
            script.steps.push(Step {
                action,
                expect: Vec::new(),
                line: number,
            });
        }),
    }
}

#[cfg(test)]
#[path = "events_script_capture_tests.rs"]
mod capture_tests;
#[cfg(test)]
#[path = "events_script_tests.rs"]
mod tests;
