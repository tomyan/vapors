//! Chromium's recording of an `.events` script (`<name>.events.json`)
//! as the comparison reads it: of this script, and of a shape it knows.

use serde_json::Value;

use crate::events_script::Script;

/// `answers`' steps, when they are this script's: the same count, and
/// each step's line and action the script's.
pub(crate) fn recording_of<'a>(
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
pub(crate) fn logged(name: &str, theirs: &Value) -> Result<Vec<(String, String)>, String> {
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
            Ok((with_modifiers(&field("type")?, event), field("target")?))
        })
        .collect()
}

/// `kind` with each modifier the event recorded as held: `click+alt`.
fn with_modifiers(kind: &str, event: &Value) -> String {
    MODIFIER_KEYS
        .iter()
        .filter(|(key, _)| event[key] == true)
        .fold(kind.to_owned(), |logged, (_, name)| {
            format!("{logged}+{name}")
        })
}

/// The DOM's modifier fields and the names a logged type adds them by,
/// in the order both sides add them (`events_vapors::logged_type`).
pub(crate) const MODIFIER_KEYS: [(&str, &str); 4] = [
    ("altKey", "alt"),
    ("ctrlKey", "ctrl"),
    ("metaKey", "meta"),
    ("shiftKey", "shift"),
];

/// Whether Chromium's event of type `name` is a `MouseEvent` (a
/// `PointerEvent` or `WheelEvent` is one), whose modifiers the recording
/// logs (`events_play.js`); decided by type, as the recorder decides by
/// class, so a keyboard-activated `click` counts.
pub(crate) fn is_mouse_event(name: &str) -> bool {
    const TYPES: [&str; 5] = ["click", "dblclick", "auxclick", "contextmenu", "wheel"];
    TYPES.contains(&name)
        || name.starts_with("mouse")
        || name.starts_with("pointer")
        || name.ends_with("pointercapture")
}

#[cfg(test)]
mod tests {
    use super::is_mouse_event;

    #[test]
    fn mouse_event_types_are_decided_by_name() {
        // Given
        let mouse = [
            "click",
            "dblclick",
            "pointerdown",
            "pointermove",
            "gotpointercapture",
            "lostpointercapture",
            "wheel",
        ];
        let other = [
            "keydown",
            "keyup",
            "focus",
            "blur",
            "input",
            "change",
            "selectionchange",
            "paste",
        ];

        // When, Then
        assert!(mouse.iter().all(|name| is_mouse_event(name)), "{mouse:?}");
        assert!(!other.iter().any(|name| is_mouse_event(name)), "{other:?}");
    }
}
