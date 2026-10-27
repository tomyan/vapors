//! vapors' log of an `.events` script compared with Chromium's recording
//! of it (`<name>.events.json`), step by step: the recording must be of
//! this script (each step's line and action), of a shape this reads,
//! log the same events at the same targets in the same order, and
//! agree on focus and on each named control's value.

use serde_json::Value;

use crate::events_script::Script;
use crate::events_vapors::Log;

/// Every step whose log differs from Chromium's `answers`, naming its
/// line and both logs.
///
/// # Errors
/// A recording of another script (its steps' lines or actions differ:
/// rerun the events oracle), one of a shape this does not read, naming
/// the step, or the differences, one line each.
pub fn compare(name: &str, script: &Script, ours: &Log, answers: &Value) -> Result<(), String> {
    let steps = recording_of(name, script, answers)?;
    let mut differences = Vec::new();
    for (theirs, ours) in steps.iter().zip(ours) {
        let line = &theirs["line"];
        let chromium = logged(name, theirs)?;
        if chromium != ours.events {
            differences.push(format!(
                "{name}.events:{line}: Chromium logged {chromium:?}, vapors {:?}",
                ours.events
            ));
        }
        let focus = theirs["focus"].as_str().ok_or_else(|| {
            format!("{name}.events.json: the step at line {line} has no string focus")
        })?;
        let visible = theirs["focusVisible"].as_bool().ok_or_else(|| {
            format!("{name}.events.json: the step at line {line} has no boolean focusVisible")
        })?;
        if visible != ours.focus_visible {
            differences.push(format!(
                "{name}.events:{line}: :focus-visible is {visible} in Chromium, {} in vapors",
                ours.focus_visible
            ));
        }
        if focus != ours.focus {
            differences.push(format!(
                "{name}.events:{line}: focus is on {focus} in Chromium, {} in vapors",
                ours.focus
            ));
        }
        for (target, value) in &ours.values {
            let chromium = theirs["values"][target].as_str().ok_or_else(|| {
                format!("{name}.events.json: the step at line {line} has no string value for {target}; rerun the events oracle")
            })?;
            if chromium != value {
                differences.push(format!(
                    "{name}.events:{line}: {target} has value {chromium:?} in Chromium, {value:?} in vapors"
                ));
            }
        }
    }
    if differences.is_empty() {
        Ok(())
    } else {
        Err(differences.join("\n"))
    }
}

/// `answers`' steps, when they are this script's: the same count, and
/// each step's line and action the script's.
fn recording_of<'a>(
    name: &str,
    script: &Script,
    answers: &'a Value,
) -> Result<&'a Vec<Value>, String> {
    let rerun = |why: String| {
        format!(
            "{name}.events.json: {why}; rerun the events oracle (cargo run -p vapors-elements-corpus --example events_oracle)"
        )
    };
    let steps = answers["steps"]
        .as_array()
        .ok_or_else(|| rerun("no steps".into()))?;
    if steps.len() != script.steps.len() {
        return Err(rerun(format!(
            "{} steps, the script has {}",
            steps.len(),
            script.steps.len()
        )));
    }
    for (theirs, step) in steps.iter().zip(&script.steps) {
        let action = format!("{:?}", step.action);
        if theirs["line"] != step.line || theirs["action"] != action.as_str() {
            return Err(rerun(format!(
                "a step recorded as {} at line {}, the script's is {action} at line {}",
                theirs["action"], theirs["line"], step.line
            )));
        }
    }
    Ok(steps)
}

/// A recorded step's events as (type, target); a step or an event of
/// another shape is refused, naming the step's line.
fn logged(name: &str, theirs: &Value) -> Result<Vec<(String, String)>, String> {
    let refuse = |why: &str| {
        format!(
            "{name}.events.json: the step at line {} {why}",
            theirs["line"]
        )
    };
    let events = theirs["events"]
        .as_array()
        .ok_or_else(|| refuse("has no events list"))?;
    events
        .iter()
        .map(|event| {
            let field = |key: &str| {
                event[key]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| refuse(&format!("has an event with no string {key}: {event}")))
            };
            Ok((field("type")?, field("target")?))
        })
        .collect()
}

#[cfg(test)]
#[path = "events_compare_tests.rs"]
mod tests;
