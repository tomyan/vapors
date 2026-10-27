//! An `.events` expectation checked against what Chromium logged after
//! its step: the events, the focus, `:focus-visible`, and each named
//! control's value.

use serde_json::Value;

use crate::events_script::Expect;

/// Why `expect` does not hold for a step's `answer`, or `None` when it does.
#[must_use]
pub fn unmet(expect: &Expect, answer: &Value) -> Option<String> {
    let events = answer["events"].as_array().cloned().unwrap_or_default();
    let focus = answer["focus"].as_str().unwrap_or("(nothing)");
    match expect {
        Expect::Event { kind, target } => {
            let found = events
                .iter()
                .any(|e| e["type"] == kind.as_str() && e["target"] == target.as_str());
            (!found).then(|| format!("no {kind} at {target}; logged: [{}]", logged(&events)))
        }
        Expect::NoEvent(kind) => {
            let found = events.iter().any(|e| e["type"] == kind.as_str());
            found.then(|| format!("a {kind} was logged: [{}]", logged(&events)))
        }
        Expect::Focus(target) => {
            (focus != target).then(|| format!("focus is on {focus}, not {target}"))
        }
        Expect::FocusVisible(target) => focus_visible(target, focus, &answer["focusVisible"]),
        Expect::Value { target, value } => {
            let found = answer["values"][target]
                .as_str()
                .unwrap_or("(not recorded)");
            (found != value).then(|| format!("{target} has value {found:?}, not {value:?}"))
        }
    }
}

fn focus_visible(target: &str, focus: &str, visible: &Value) -> Option<String> {
    let shown = focus == target && *visible == true;
    (!shown).then(|| {
        format!(
            "{target} does not show focus-visible; focus is on {focus}, focus-visible {visible}"
        )
    })
}

fn logged(events: &[Value]) -> String {
    let each = events.iter().map(|e| {
        format!(
            "{} at {}",
            e["type"].as_str().unwrap_or("?"),
            e["target"].as_str().unwrap_or("?")
        )
    });
    each.collect::<Vec<_>>().join(", ")
}
