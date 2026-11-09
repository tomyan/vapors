//! Why the a11y tree, or a node in it, cannot be answered (R1).

use std::fmt;

/// A refusal from the a11y tree.
#[derive(Clone, Debug, PartialEq)]
pub enum AxError {
    /// The document has changed since its last update, or has none.
    NotUpdated,
    /// No node has the role and name.
    Missing {
        /// The role asked for.
        role: String,
        /// The name asked for.
        name: String,
        /// Every node of that role (inside the ancestor asked for, when
        /// one was), with its ancestors, `a "x" > b "y"`.
        candidates: Vec<String>,
        /// The nodes with the role and name outside the ancestor asked
        /// for; empty when none was.
        outside: Vec<String>,
    },
    /// More than one node has the role and name.
    Ambiguous {
        /// The role asked for.
        role: String,
        /// The name asked for.
        name: String,
        /// Each match with its ancestors in the tree, `a "x" > b "y"`.
        matches: Vec<String>,
    },
    /// No drawn node has the role and name, and one that is not drawn
    /// (`display: none`, `visibility: hidden`, `aria-hidden`) does.
    Hidden {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
    },
    /// The node is disabled, so a person could not act on it.
    Disabled {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
    },
    /// A press at the node's centre would reach another element, or
    /// nothing (`at`, CSS px); `by` is the element's tag.
    Covered {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
        /// The point pressed.
        at: [f32; 2],
        /// The element there, if any.
        by: Option<String>,
    },
    /// A press at the node's centre (`at`, CSS px) falls outside the
    /// viewport even after its scroll containers brought it into view.
    OutOfView {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
        /// The point a press would take.
        at: [f32; 2],
    },
    /// The node has `pointer-events: none`, so no pointer reaches it.
    NoPointer {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
    },
    /// Focus left the node before every key was sent: `sent` of `of`
    /// went to it; `to` names the node that took focus, `None` when none
    /// did.
    FocusLost {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
        /// The node that has focus, `role "name"` or `<tag>`.
        to: Option<String>,
        /// The keys sent to the node.
        sent: usize,
        /// The keys asked for.
        of: usize,
    },
    /// The update after scrolling the node into view failed.
    UpdateFailed {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
        /// The update's errors.
        errors: Vec<String>,
    },
    /// Focus was asked of a node that takes none.
    NotFocusable {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
    },
    /// A value was set on a node that is no slider or spinbutton.
    NotSettable {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
    },
    /// The value set is not a valid floating-point number as HTML
    /// parses one.
    NotANumber {
        /// The node's role.
        role: String,
        /// The node's name.
        name: String,
        /// The value sent.
        value: String,
    },
}

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
            Self::UpdateFailed { role, name, errors } => write!(
                f,
                "the update after scrolling {role} \"{name}\" into view failed: [{}]; fix: the errors name it",
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

impl std::error::Error for AxError {}
