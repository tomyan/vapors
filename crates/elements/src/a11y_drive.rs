//! One vocabulary for driving a document by its a11y tree (R1): a
//! [`Command`] in, a [`Reply`] out, serialisable so stdio, a socket and
//! a page call carry the same JSON, and one [`Document::drive`] they
//! share. A command acts on a node by role and exact name, within an
//! ancestor's when given; every refusal is a [`Reply::Refused`] naming
//! the problem, never a panic. A command that acts updates the document
//! after its action, at the last update's viewport, and replies the tree
//! from that update; a query or find on a document changed since by
//! anything else is refused, as [`Document::ax_tree`] is.

use std::cell::RefCell;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::a11y_error::AxError;
use crate::a11y_target::AxTarget;
use crate::a11y_tree::AxElement;
use crate::document::Document;
use crate::keys::Modifiers;

/// A modifier key held with a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Held {
    /// Shift.
    Shift,
    /// Control.
    Control,
    /// Alt (Option on macOS).
    Alt,
    /// Meta: Command on macOS.
    Meta,
}

/// What a driver asks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    /// The whole tree.
    Query,
    /// One node ([`Document::ax_find`]).
    Find {
        /// The node.
        target: AxTarget,
    },
    /// [`Document::ax_press`].
    Press {
        /// The node.
        target: AxTarget,
    },
    /// [`Document::ax_focus`].
    Focus {
        /// The node.
        target: AxTarget,
    },
    /// [`Document::ax_set_value`].
    SetValue {
        /// The node.
        target: AxTarget,
        /// The value, as text.
        value: String,
    },
    /// [`Document::ax_type`].
    Type {
        /// The node.
        target: AxTarget,
        /// The text.
        text: String,
    },
    /// [`Document::ax_key`].
    Key {
        /// The node.
        target: AxTarget,
        /// The key, as the DOM's `key` (`Enter`, `ArrowUp`, `a`).
        key: String,
        /// The modifiers held.
        #[serde(default)]
        modifiers: Vec<Held>,
    },
}

/// The answer to a [`Command`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "snake_case")]
pub enum Reply {
    /// The tree's top nodes, each with its subtree.
    Tree {
        /// The nodes.
        nodes: Vec<AxElement>,
    },
    /// The node found.
    Found {
        /// The node.
        node: AxElement,
    },
    /// The action happened; the tree from the update after it, or why
    /// that tree is unavailable: the action is not to be repeated.
    Done {
        /// The tree's top nodes, each with its subtree; `None` when the
        /// tree after the action is unavailable.
        nodes: Option<Vec<AxElement>>,
        /// Why the tree is unavailable, with `nodes` `None`; its fields
        /// sit beside the reply's.
        #[serde(flatten)]
        problem: Option<AxError>,
        /// The problem in words, naming the fix.
        message: Option<String>,
    },
    /// The command was refused.
    Refused {
        /// Why; its fields sit beside the reply's.
        #[serde(flatten)]
        problem: AxError,
        /// The problem in words, naming the fix.
        message: String,
    },
}

impl Reply {
    /// `problem` refused, in words beside it.
    pub(crate) fn refused(problem: AxError) -> Self {
        Self::Refused {
            message: problem.to_string(),
            problem,
        }
    }
}

impl Document {
    /// Answers `command`; a refusal is [`Reply::Refused`].
    pub fn drive(document: &Rc<RefCell<Document>>, command: &Command) -> Reply {
        Self::answer(document, command).unwrap_or_else(Reply::refused)
    }

    fn answer(document: &Rc<RefCell<Document>>, command: &Command) -> Result<Reply, AxError> {
        let acted = match command {
            Command::Query => return Self::query(document),
            Command::Find { target } => return Self::find(document, target),
            Command::Press { target } => Self::ax_press(document, target),
            Command::Focus { target } => Self::ax_focus(document, target),
            Command::SetValue { target, value } => Self::ax_set_value(document, target, value),
            Command::Type { target, text } => Self::ax_type(document, target, text),
            Command::Key {
                target,
                key,
                modifiers,
            } => Self::ax_key(document, target, key, held(modifiers)),
        };
        acted.map(|()| Self::done(document))
    }

    /// [`Reply::Tree`]: the tree from the last update.
    fn query(document: &Rc<RefCell<Document>>) -> Result<Reply, AxError> {
        let nodes = document.borrow().ax_tree()?;
        Ok(Reply::Tree { nodes })
    }

    /// [`Reply::Found`]: the one node `target` names.
    fn find(document: &Rc<RefCell<Document>>, target: &AxTarget) -> Result<Reply, AxError> {
        let node = document
            .borrow()
            .ax_find(&target.role, &target.name, target.within())?;
        Ok(Reply::Found { node })
    }

    /// [`Reply::Done`] after an action: with the updated tree, or why
    /// it is unavailable.
    fn done(document: &Rc<RefCell<Document>>) -> Reply {
        match Self::updated_tree(document) {
            Ok(nodes) => Reply::Done {
                nodes: Some(nodes),
                problem: None,
                message: None,
            },
            Err(problem) => Reply::Done {
                nodes: None,
                message: Some(problem.to_string()),
                problem: Some(problem),
            },
        }
    }

    /// The document updated at its last update's viewport, and its tree.
    /// A second update follows when the first moved `:hover` (its pointer
    /// over a box that moved), which only the next restyle shows.
    fn updated_tree(document: &Rc<RefCell<Document>>) -> Result<Vec<AxElement>, AxError> {
        let viewport = document.borrow().kept.as_ref().map(|kept| kept.viewport);
        let viewport = viewport.ok_or(AxError::NotUpdated)?;
        for _ in 0..2 {
            document
                .borrow_mut()
                .update(viewport)
                .map_err(|errors| AxError::UpdateFailed {
                    after: "the action".to_owned(),
                    errors: errors.iter().map(ToString::to_string).collect(),
                })?;
            if let Ok(nodes) = document.borrow().ax_tree() {
                return Ok(nodes);
            }
        }
        Err(AxError::NotUpdated)
    }
}

/// `held` as the DOM's modifiers.
fn held(held: &[Held]) -> Modifiers {
    Modifiers {
        shift: held.contains(&Held::Shift),
        control: held.contains(&Held::Control),
        alt: held.contains(&Held::Alt),
        meta: held.contains(&Held::Meta),
    }
}
