//! Why the a11y tree, or a node in it, cannot be answered (R1).

use std::fmt;

/// A refusal from the a11y tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AxError {
    /// The document has changed since its last update, or has none.
    NotUpdated,
    /// No node has the role and name.
    Missing {
        /// The role asked for.
        role: String,
        /// The name asked for.
        name: String,
        /// Every node of that role, with its ancestors, `a "x" > b "y"`.
        candidates: Vec<String>,
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
            } => write!(
                f,
                "no node is {role} \"{name}\" (names are exact); the {role} nodes: [{}]; fix: use one of their names",
                candidates.join(", ")
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
        }
    }
}

impl std::error::Error for AxError {}
