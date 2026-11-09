//! One vocabulary for driving a document by its a11y tree (R1): a
//! [`Command`] in, a [`Reply`] out, serialisable so stdio, a socket and
//! a page call carry the same JSON, and one [`Document::drive`] they
//! share. A command acts on a node by role and exact name, within an
//! ancestor's when given; every refusal is a [`Reply::Refused`] naming
//! the problem, never a panic. Commands answer from the last update, so
//! a driver updates between an action and the next command.

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
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// The whole tree.
    Query,
    /// One node ([`Document::ax_find`]).
    Find(AxTarget),
    /// [`Document::ax_press`].
    Press(AxTarget),
    /// [`Document::ax_focus`].
    Focus(AxTarget),
    /// [`Document::ax_set_value`].
    SetValue {
        /// The node.
        #[serde(flatten)]
        target: AxTarget,
        /// The value, as text.
        value: String,
    },
    /// [`Document::ax_type`].
    Type {
        /// The node.
        #[serde(flatten)]
        target: AxTarget,
        /// The text.
        text: String,
    },
    /// [`Document::ax_key`].
    Key {
        /// The node.
        #[serde(flatten)]
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
    /// The action happened.
    Done,
    /// The command was refused.
    Refused {
        /// Why.
        problem: AxError,
        /// The problem in words, naming the fix.
        message: String,
    },
}

impl Document {
    /// Answers `command`; a refusal is [`Reply::Refused`].
    pub fn drive(document: &Rc<RefCell<Document>>, command: &Command) -> Reply {
        Self::answer(document, command).unwrap_or_else(|problem| Reply::Refused {
            message: problem.to_string(),
            problem,
        })
    }

    fn answer(document: &Rc<RefCell<Document>>, command: &Command) -> Result<Reply, AxError> {
        let done = Reply::Done;
        match command {
            Command::Query => Ok(Reply::Tree {
                nodes: document.borrow().ax_tree()?,
            }),
            Command::Find(t) => Ok(Reply::Found {
                node: document.borrow().ax_find(&t.role, &t.name, t.within())?,
            }),
            Command::Press(t) => Self::ax_press(document, t).map(|()| done),
            Command::Focus(t) => Self::ax_focus(document, t).map(|()| done),
            Command::SetValue { target, value } => {
                Self::ax_set_value(document, target, value).map(|()| done)
            }
            Command::Type { target, text } => Self::ax_type(document, target, text).map(|()| done),
            Command::Key {
                target,
                key,
                modifiers,
            } => Self::ax_key(document, target, key, held(modifiers)).map(|()| done),
        }
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

/// A [`NodeId`](vapors_dom::NodeId) as its number.
pub(crate) mod node_id {
    use serde::{Deserialize, Deserializer, Serializer};
    use vapors_dom::NodeId;

    #[allow(clippy::trivially_copy_pass_by_ref)] // serde's `with` passes a reference
    pub(crate) fn serialize<S: Serializer>(node: &NodeId, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(node.0)
    }

    pub(crate) fn deserialize<'d, D: Deserializer<'d>>(d: D) -> Result<NodeId, D::Error> {
        u64::deserialize(d).map(NodeId)
    }
}
