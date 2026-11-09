//! Each a11y refusal in words, naming the fix (R1).

use std::fmt;

use crate::a11y_error::AxError;

impl fmt::Display for AxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUpdated => write!(
                f,
                "the a11y tree is read from the last update, and the document has changed since or has none; fix: update the document first"
            ),
            Self::Missing {
                role,
                name,
                candidates,
                outside,
            } if outside.is_empty() => write!(
                f,
                "no node is {role} \"{name}\" (names are exact); the {role} nodes: [{}]; fix: use one of their names",
                candidates.join(", ")
            ),
            Self::Missing {
                role,
                name,
                candidates,
                outside,
            } => write!(
                f,
                "no node inside the ancestor asked for is {role} \"{name}\"; the {role} nodes inside it: [{}]; outside it: [{}]; fix: name the ancestor of one of those, or one of the names inside",
                candidates.join(", "),
                outside.join(", ")
            ),
            Self::Ambiguous {
                role,
                name,
                matches,
            } => write!(
                f,
                "{} nodes are {role} \"{name}\": [{}]; fix: name an ancestor that holds only one",
                matches.len(),
                matches.join("; ")
            ),
            Self::Hidden { role, name } => write!(
                f,
                "{role} \"{name}\" is not drawn (display: none, visibility: hidden or aria-hidden); fix: show it first"
            ),
            Self::Disabled { role, name } => {
                write!(f, "{role} \"{name}\" is disabled; fix: enable it first")
            }
            Self::Covered { role, name, at, by } => write!(
                f,
                "a press at the centre of {role} \"{name}\" ({}, {}) reaches {}; fix: close what covers it",
                at[0],
                at[1],
                by.as_deref()
                    .map_or("nothing".to_owned(), |tag| format!("<{tag}>"))
            ),
            Self::OutOfView { role, name, at } => write!(
                f,
                "the centre of {role} \"{name}\" ({}, {}) is outside the viewport, and no scroll container brings it in; fix: make the viewport larger or move the node into it",
                at[0], at[1]
            ),
            Self::NoPointer { role, name } => write!(
                f,
                "{role} \"{name}\" has pointer-events: none, so a press cannot reach it; fix: give it pointer-events: auto, or act on it by key"
            ),
            Self::FocusLost {
                role,
                name,
                to,
                sent,
                of,
            } => write!(
                f,
                "focus left {role} \"{name}\" after {sent} of {of} keys, to {}; fix: find the listener that moves focus, or send the rest to the node that has it",
                to.as_deref().unwrap_or("nothing")
            ),
            Self::UpdateFailed { after, errors } => write!(
                f,
                "the update after {after} failed: [{}]; fix: the page's styles or layout, as the errors name",
                errors.join("; ")
            ),
            Self::NotFocusable { role, name } => {
                write!(f, "{role} \"{name}\" takes no focus; fix: act on a control")
            }
            Self::NotSettable { role, name } => write!(
                f,
                "{role} \"{name}\" has no value to set (only a slider or spinbutton has); fix: type into a textbox, press others"
            ),
            Self::NotANumber { role, name, value } => write!(
                f,
                "\"{value}\" set on {role} \"{name}\" is not a number; fix: send a number such as 12.5"
            ),
        }
    }
}
