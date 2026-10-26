//! Playing an `.events` script in a page Chromium has loaded: each action
//! through `Input`, then the events logged since the previous step, the
//! focused element and whether it matches `:focus-visible`. Each step's
//! answer is recorded; each expectation is checked against it, and an
//! expectation whose target names more or fewer than one element is
//! refused. Events that arrive after the last step are refused: one that
//! fires late into a step would make that step's log wrong.

use std::time::Duration;

use serde_json::{Value, json};
use vapors_devtools::Session;

use crate::events_input::{Pointer, calls, unmet};
use crate::events_script::{Expect, Script, Step};

const PAGE_SIDE: &str = include_str!("events_play.js");

/// Two animation frames: what a step waits for before its answer, so the
/// page has handled what the action caused.
const FRAMES: &str =
    "new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => done(true))))";

/// Half a second after the last step, for timers and transitions a step
/// set off. It first sets `window.__eventsSettling`, so a page can act
/// after the last step's log is taken and be sure of the order.
const SETTLE: &str =
    "(window.__eventsSettling = true, new Promise((done) => setTimeout(() => done(true), 500)))";

/// Two frames took 14.7 to 49.6 ms (median 33.4) over 30 waits in
/// headless Chromium on the dev machine at load 25 (2026-10-25); 5 s is
/// a stalled page, not a slow one.
const FRAME_DEADLINE: Duration = Duration::from_secs(5);

/// A script being played: the page's session, the call deadline, and
/// the pointer's held button.
struct Player<'a> {
    session: &'a mut Session,
    deadline: Duration,
    pointer: Pointer,
}

/// Each step's answers, or every expectation that did not hold, as
/// `<name>.events:<line>: <why>`.
///
/// # Errors
/// A call that fails, events after the last step, or expectations that
/// do not hold, each naming its line.
pub fn play(
    session: &mut Session,
    name: &str,
    script: &Script,
    deadline: Duration,
) -> Result<Value, String> {
    let mut player = Player {
        session,
        deadline,
        pointer: Pointer::default(),
    };
    let types = serde_json::to_string(&script.listen).map_err(|e| e.to_string())?;
    player
        .evaluate(&PAGE_SIDE.replace("__TYPES__", &types), deadline)
        .map_err(|e| format!("{name}.events: {e}"))?;
    let (answers, unmet_lines) = player.steps(name, script)?;
    player
        .nothing_late()
        .map_err(|e| format!("{name}.events: {e}"))?;
    if unmet_lines.is_empty() {
        Ok(json!({ "steps": answers }))
    } else {
        Err(unmet_lines.join("\n"))
    }
}

impl Player<'_> {
    /// Every step's answer, and every expectation that did not hold.
    fn steps(&mut self, name: &str, script: &Script) -> Result<(Vec<Value>, Vec<String>), String> {
        let (mut answers, mut unmet_lines) = (Vec::new(), Vec::new());
        for step in &script.steps {
            let answer = self
                .act(step)
                .map_err(|e| format!("{name}.events:{}: {e}", step.line))?;
            for (expect, line) in &step.expect {
                let unmet = self
                    .check(expect, &answer)
                    .map_err(|e| format!("{name}.events:{line}: {e}"))?;
                unmet_lines.extend(unmet.map(|why| format!("{name}.events:{line}: {why}")));
            }
            answers.push(answer);
        }
        Ok((answers, unmet_lines))
    }

    /// Perform one step and read its answer.
    fn act(&mut self, step: &Step) -> Result<Value, String> {
        for (method, params) in calls(&step.action, &mut self.pointer) {
            self.session.call(method, &params, self.deadline)?;
        }
        self.evaluate(FRAMES, FRAME_DEADLINE)?;
        let events = self.evaluate("window.__eventsDrain()", self.deadline)?;
        let focus = self.evaluate("window.__eventsFocus()", self.deadline)?;
        Ok(
            json!({ "line": step.line, "action": format!("{:?}", step.action), "events": events, "focus": focus["focus"], "focusVisible": focus["focusVisible"] }),
        )
    }

    /// After the last step, [`SETTLE`] and two more frames; any event
    /// logged then was caused by a step but missed its log. Declared
    /// limit: an event later than that is not seen.
    fn nothing_late(&mut self) -> Result<(), String> {
        self.evaluate(SETTLE, FRAME_DEADLINE)?;
        self.evaluate(FRAMES, FRAME_DEADLINE)?;
        let late = self.evaluate("window.__eventsDrain()", self.deadline)?;
        match late.as_array() {
            Some(late) if late.is_empty() => Ok(()),
            _ => Err(format!(
                "events arrived after the last step, missed by its log: {late}; fix: end with `frame` and expect them there"
            )),
        }
    }

    /// Why `expect` does not hold, refusing a target that is not one element.
    fn check(&mut self, expect: &Expect, answer: &Value) -> Result<Option<String>, String> {
        if let Expect::Event { target, .. } | Expect::Focus(target) | Expect::FocusVisible(target) =
            expect
        {
            let selector = serde_json::to_string(target).map_err(|e| e.to_string())?;
            let count =
                self.evaluate(&format!("window.__eventsCount({selector})"), self.deadline)?;
            if count != 1 {
                return Ok(Some(format!("{target} names {count} elements, not one")));
            }
        }
        Ok(unmet(expect, answer))
    }

    /// `expression`'s value, awaited; an exception is refused with its text.
    fn evaluate(&mut self, expression: &str, deadline: Duration) -> Result<Value, String> {
        let params =
            json!({ "expression": expression, "awaitPromise": true, "returnByValue": true });
        let mut result = self.session.call("Runtime.evaluate", &params, deadline)?;
        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception["exception"]["description"]
                .as_str()
                .map_or_else(|| exception.to_string(), str::to_owned);
            return Err(format!("the page threw: {text}"));
        }
        Ok(result["result"]["value"].take())
    }
}
