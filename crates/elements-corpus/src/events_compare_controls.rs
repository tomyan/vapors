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
        for target in &script.selection_targets() {
            differences.extend(self.selection(target, ours)?);
        }
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

/// A recorded `[start, end]` as a pair.
fn selection_pair(value: &Value) -> Option<(usize, usize)> {
    let pair = value.as_array()?;
    let at = |i: usize| pair.get(i)?.as_u64().and_then(|n| usize::try_from(n).ok());
    Some((at(0)?, at(1)?))
}

#[cfg(test)]
#[path = "events_compare_controls_tests.rs"]
mod tests;
