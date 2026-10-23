//! Refusals, each naming the node and where it was created.

use std::fmt;
use std::panic::Location;

/// What a node in the graph is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A readable, writable value.
    Signal,
    /// A cached derived value.
    Memo,
    /// A side effect re-run when what it read changes.
    Effect,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Signal => "signal",
            Kind::Memo => "memo",
            Kind::Effect => "effect",
        })
    }
}

/// Identifies a node in messages: its kind, its name if it was given
/// one, and the source location that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeInfo {
    /// Signal, memo or effect.
    pub kind: Kind,
    /// The name passed to `named`, if any.
    pub name: Option<&'static str>,
    /// Where `new` or `named` was called.
    pub created_at: &'static Location<'static>,
}

impl fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(name) => write!(f, "{} `{}` created at {}", self.kind, name, self.created_at),
            None => write!(f, "{} created at {}", self.kind, self.created_at),
        }
    }
}

/// Why a read or write was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The node's owner was disposed, or the node itself was.
    Disposed {
        /// The node that was asked for.
        node: NodeInfo,
    },
    /// Reading the node would re-enter a computation already running.
    /// The path lists the nodes being computed, outermost first, ending
    /// with the node whose read closed the loop.
    Cycle {
        /// The nodes on the loop.
        path: Vec<NodeInfo>,
    },
    /// The value is borrowed by a `with` on the same node higher on
    /// the stack, so it cannot be written now.
    Busy {
        /// The node whose value is borrowed.
        node: NodeInfo,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Disposed { node } => {
                write!(
                    f,
                    "{node} was disposed; dispose the owner after its last use"
                )
            }
            Error::Cycle { path } => {
                let names: Vec<String> = path.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "reading it would form a cycle: {}; break the loop with untrack or a plain value",
                    names.join(" -> ")
                )
            }
            Error::Busy { node } => write!(
                f,
                "{node} is borrowed by a `with` on the same node; write after the `with` returns"
            ),
        }
    }
}

impl std::error::Error for Error {}
