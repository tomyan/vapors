//! Signals, memos, effects, batches and owner scopes.
//!
//! The semantics are Leptos's (`reactive_graph`): a signal write marks
//! its readers dirty and their readers "check"; a memo recomputes only
//! when read and only when a source actually changed, so a diamond runs
//! once; effects run synchronously after the write, or after the
//! outermost batch, in creation order.
//!
//! The graph lives in one thread-local runtime with no locks, because
//! the UI thread owns it: components mount, signals change and effects
//! patch the element tree on that thread alone. The automation API
//! posts closures to the UI thread rather than touching the graph from
//! another one. The same code runs on native and on wasm32, where there
//! is one thread anyway.
//!
//! Refusals are values, not panics, wherever a value can be returned:
//! [`Signal::try_get`] on a disposed signal and a read that closes a
//! memo cycle both return an [`Error`] naming the node and where it was
//! created. The panicking conveniences (`get`, `set`) carry the same
//! message.

mod access;
mod effect;
mod error;
mod flush;
mod guard;
mod memo;
mod node;
mod owner;
mod propagate;
mod runtime;
mod signal;
mod signal_convenience;
mod track;
mod trigger;

pub use effect::Effect;
pub use error::{Error, Kind, NodeInfo};
pub use flush::batch;
pub use memo::Memo;
pub use owner::{Owner, on_cleanup};
pub use signal::Signal;
pub use track::untrack;
pub use trigger::Trigger;
