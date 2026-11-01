//! vapors' log of an `.events` script compared with Chromium's recording
//! of it (`<name>.events.json`), step by step: the recording must be of
//! this script (each step's line and action), of a shape this reads,
//! log the same events at the same targets in the same order, and
//! agree on focus, on each named control's value, on each named
//! checkbox's state and on each named text field's selection.

use serde_json::Value;

use crate::events_recording::{logged, recording_of};
use crate::events_script::Script;
use crate::events_vapors::{Log, Logged};

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
        let step = Step {
            name,
            line: theirs["line"].to_string(),
            theirs,
        };
        differences.extend(step.events(ours)?);
        differences.extend(step.focus(ours)?);
        differences.extend(step.controls(script, ours)?);
        differences.extend(step.projection(ours));
    }
    if differences.is_empty() {
        Ok(())
    } else {
        Err(differences.join("\n"))
    }
}

/// One recorded step, with what every comparison names.
struct Step<'a> {
    name: &'a str,
    line: String,
    theirs: &'a Value,
}

impl Step<'_> {
    /// The events, when Chromium's differ from vapors'.
    fn events(&self, ours: &Logged) -> Result<Option<String>, String> {
        let chromium = logged(self.name, self.theirs)?;
        Ok((chromium != ours.events).then(|| {
            format!(
                "{}.events:{}: Chromium logged {chromium:?}, vapors {:?}",
                self.name, self.line, ours.events
            )
        }))
    }

    /// The focus and `:focus-visible`, where they differ.
    fn focus(&self, ours: &Logged) -> Result<Vec<String>, String> {
        let focus = self.theirs["focus"].as_str().ok_or_else(|| {
            format!(
                "{}.events.json: the step at line {} has no string focus",
                self.name, self.line
            )
        })?;
        let visible = self.theirs["focusVisible"].as_bool().ok_or_else(|| {
            format!(
                "{}.events.json: the step at line {} has no boolean focusVisible",
                self.name, self.line
            )
        })?;
        let mut out = Vec::new();
        if visible != ours.focus_visible {
            out.push(format!(
                "{}.events:{}: :focus-visible is {visible} in Chromium, {} in vapors",
                self.name, self.line, ours.focus_visible
            ));
        }
        if focus != ours.focus {
            out.push(format!(
                "{}.events:{}: focus is on {focus} in Chromium, {} in vapors",
                self.name, self.line, ours.focus
            ));
        }
        Ok(out)
    }

    /// What Chromium recorded under `field` for `target`, read by
    /// `read`; a recording without it asks for a rerun.
    fn recorded<'a, T>(
        &'a self,
        field: &str,
        target: &str,
        read: impl FnOnce(&'a Value) -> Option<T>,
        shape: &str,
    ) -> Result<T, String> {
        read(&self.theirs[field][target]).ok_or_else(|| {
            format!(
                "{}.events.json: the step at line {} has no {shape} for {target}; rerun the events oracle",
                self.name, self.line
            )
        })
    }

    /// What vapors logged for `target`; a log without it is refused.
    fn ours<T: Clone>(
        &self,
        logged: &[(String, T)],
        target: &str,
        what: &str,
    ) -> Result<T, String> {
        logged
            .iter()
            .find(|(t, _)| t == target)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                format!(
                    "{}.events:{}: vapors' log has no {what} for {target}",
                    self.name, self.line
                )
            })
    }

    /// The difference line when `chromium` and `vapors` differ.
    fn differs(&self, target: &str, verb: &str, chromium: &str, vapors: &str) -> Option<String> {
        (chromium != vapors).then(|| {
            format!(
                "{}.events:{}: {target} {verb} {chromium} in Chromium, {vapors} in vapors",
                self.name, self.line
            )
        })
    }
}

#[path = "events_compare_controls.rs"]
mod controls;

#[cfg(test)]
#[path = "events_compare_tests.rs"]
mod tests;
