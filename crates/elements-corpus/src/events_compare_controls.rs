//! The controls a step names, compared: each value, each text field's
//! selection and each checkbox's state, Chromium's against vapors'.

use serde_json::Value;

use super::Step;
use crate::events_script::Script;
use crate::events_vapors::Logged;

impl Step<'_> {
    /// How each named control's value, selection and checked state
    /// differ from Chromium's.
    pub(super) fn controls(&self, script: &Script, ours: &Logged) -> Result<Vec<String>, String> {
        let mut differences = Vec::new();
        for target in &script.value_targets() {
            differences.extend(self.value(target, ours)?);
        }
        differences.extend(self.selections(ours)?);
        for target in &script.checked_targets() {
            differences.extend(self.checked(target, ours)?);
        }
        Ok(differences)
    }

    fn value(&self, target: &str, ours: &Logged) -> Result<Option<String>, String> {
        let chromium = self.recorded("values", target, Value::as_str, "string value")?;
        let vapors = self.ours(&ours.values, target, "value")?;
        Ok(self.differs(
            target,
            "has value",
            &format!("{chromium:?}"),
            &format!("{vapors:?}"),
        ))
    }

    /// Every text field's selection: both logs must name the same
    /// fields, and each field's must agree.
    fn selections(&self, ours: &Logged) -> Result<Vec<String>, String> {
        let recorded = self.theirs["selections"].as_object().ok_or_else(|| {
            format!(
                "{}.events.json: the step at line {} has no selections; rerun the events oracle (cargo run -p vapors-elements-corpus --example events_oracle)",
                self.name, self.line
            )
        })?;
        let chromium: Vec<&String> = recorded.keys().collect();
        let vapors: Vec<&String> = ours.selections.iter().map(|(field, _)| field).collect();
        if chromium != vapors {
            return Ok(vec![format!(
                "{}.events:{}: the text fields are {chromium:?} in Chromium, {vapors:?} in vapors",
                self.name, self.line
            )]);
        }
        let mut differences = Vec::new();
        for field in chromium {
            differences.extend(self.selection(field, ours)?);
        }
        Ok(differences)
    }

    fn selection(&self, target: &str, ours: &Logged) -> Result<Option<String>, String> {
        let chromium = self.recorded("selections", target, selection_pair, "selection pair")?;
        let vapors = self.ours(&ours.selections, target, "selection")?;
        let range = |(start, end): (usize, usize)| format!("{start}..{end}");
        Ok(self.differs(target, "has selection", &range(chromium), &range(vapors)))
    }

    fn checked(&self, target: &str, ours: &Logged) -> Result<Option<String>, String> {
        let chromium = self.recorded("checked", target, Value::as_bool, "boolean checked")?;
        let vapors = self.ours(&ours.checked, target, "checked state")?;
        Ok(self.differs(
            target,
            "is checked",
            &chromium.to_string(),
            &vapors.to_string(),
        ))
    }
}

/// A recorded `[start, end]` as a pair: exactly two whole numbers,
/// start at most end.
fn selection_pair(value: &Value) -> Option<(usize, usize)> {
    let [start, end] = value.as_array()?.as_slice() else {
        return None;
    };
    let at = |n: &Value| n.as_u64().and_then(|n| usize::try_from(n).ok());
    let (start, end) = (at(start)?, at(end)?);
    (start <= end).then_some((start, end))
}

#[cfg(test)]
#[path = "events_compare_controls_tests.rs"]
mod tests;
