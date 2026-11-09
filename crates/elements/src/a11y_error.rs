//! Why the a11y tree, or a node in it, cannot be answered (R1).

use std::fmt;

use serde::{Deserialize, Serialize};

/// A refusal from the a11y tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "problem", rename_all = "snake_case")]
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
                "a press at the centre of {role} \"{name}\" ({}, {}) reaches {}; fix: scroll it into view or close what covers it",
                at[0],
                at[1],
                by.as_deref()
                    .map_or("nothing".to_owned(), |tag| format!("<{tag}>"))
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
