//! Why the a11y tree, or a node in it, cannot be answered (R1).

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
    /// An update the driver made failed: after scrolling a node into
    /// view, or after an action.
    UpdateFailed {
        /// What the update came after, in words.
        after: String,
        /// Each of the update's errors, in words.
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

impl std::error::Error for AxError {}
