//! Playing an `.events` script in a page Chromium has loaded: each action
//! through `Input`, then the events logged since the previous step, the
//! focused element and whether it matches `:focus-visible`. Each step's
//! answer is recorded; each expectation is checked against it, and an
//! expectation whose target names more or fewer than one element is
//! refused.

use std::time::Duration;

use serde_json::{Value, json};
use vapors_devtools::Session;

use crate::events_input::{calls, unmet};
use crate::events_script::{Expect, Script, Step};

const PAGE_SIDE: &str = include_str!("events_play.js");

/// Two animation frames: what a step waits for before its answer, so the
/// page has handled what the action caused.
const FRAMES: &str =
    "new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => done(true))))";

/// Each step's answers, or every expectation that did not hold, as
/// `<name>.events:<line>: <why>`.
///
/// # Errors
/// A call that fails, or expectations that do not hold.
pub fn play(
    session: &mut Session,
    name: &str,
    script: &Script,
    deadline: Duration,
) -> Result<Value, String> {
    let types = serde_json::to_string(&script.listen).map_err(|e| e.to_string())?;
    evaluate(session, &PAGE_SIDE.replace("__TYPES__", &types), deadline)?;
    let mut answers = Vec::new();
    let mut unmet_lines = Vec::new();
    for step in &script.steps {
        let answer = act(session, step, deadline)?;
        for (expect, line) in &step.expect {
            if let Some(why) = check(session, expect, &answer, deadline)? {
                unmet_lines.push(format!("{name}.events:{line}: {why}"));
            }
        }
        answers.push(answer);
    }
    if !unmet_lines.is_empty() {
        return Err(unmet_lines.join("\n"));
    }
    Ok(json!({ "steps": answers }))
}

/// Perform one step and read its answer.
fn act(session: &mut Session, step: &Step, deadline: Duration) -> Result<Value, String> {
    for (method, params) in calls(&step.action) {
        session.call(method, &params, deadline)?;
    }
    evaluate(session, FRAMES, deadline)?;
    let events = evaluate(session, "window.__eventsDrain()", deadline)?;
    let focus = evaluate(session, "window.__eventsFocus()", deadline)?;
    Ok(json!({
        "line": step.line,
        "action": format!("{:?}", step.action),
        "events": events,
        "focus": focus["focus"],
        "focusVisible": focus["focusVisible"],
    }))
}

/// Why `expect` does not hold, refusing a target that is not one element.
fn check(
    session: &mut Session,
    expect: &Expect,
    answer: &Value,
    deadline: Duration,
) -> Result<Option<String>, String> {
    let target = match expect {
        Expect::Event { target, .. } | Expect::Focus(target) | Expect::FocusVisible(target) => {
            Some(target)
        }
        Expect::NoEvent(_) => None,
    };
    if let Some(target) = target {
        let selector = serde_json::to_string(target).map_err(|e| e.to_string())?;
        let count = evaluate(
            session,
            &format!("window.__eventsCount({selector})"),
            deadline,
        )?;
        if count != 1 {
            return Ok(Some(format!("{target} names {count} elements, not one")));
        }
    }
    Ok(unmet(expect, answer))
}

/// `expression`'s value, awaited; an exception is refused with its text.
fn evaluate(session: &mut Session, expression: &str, deadline: Duration) -> Result<Value, String> {
    let params = json!({ "expression": expression, "awaitPromise": true, "returnByValue": true });
    let mut result = session.call("Runtime.evaluate", &params, deadline)?;
    if let Some(exception) = result.get("exceptionDetails") {
        return Err(format!(
            "the page threw: {}",
            exception["exception"]["description"]
                .as_str()
                .unwrap_or(&exception.to_string())
        ));
    }
    Ok(result["result"]["value"].take())
}
