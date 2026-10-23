//! `Signal<T>`: a readable, writable value whose readers re-run.

use std::fmt;
use std::marker::PhantomData;
use std::panic::Location;

use crate::access::{cell, read_cell, with_cell, write_cell};
use crate::error::{Error, Kind, NodeInfo};
use crate::node::{NodeId, State};
use crate::owner::dispose_node;
use crate::runtime::with_runtime;

/// A handle to a value in the graph. Copying the handle shares the
/// value. Reading it inside a memo or effect subscribes that reader.
pub struct Signal<T> {
    id: NodeId,
    info: NodeInfo,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Signal<T> {}

impl<T> fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.info)
    }
}

impl<T: 'static> Signal<T> {
    /// Creates a signal owned by the current owner.
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self::create(None, value, Location::caller())
    }

    /// Creates a signal that refusals name as `name`.
    #[track_caller]
    pub fn named(name: &'static str, value: T) -> Self {
        Self::create(Some(name), value, Location::caller())
    }

    fn create(
        name: Option<&'static str>,
        value: T,
        created_at: &'static Location<'static>,
    ) -> Self {
        let (id, info) = with_runtime(|runtime| {
            let id = runtime.create_node(Kind::Signal, name, created_at, State::Clean);
            let node = runtime.node_mut(id).expect("just created");
            node.value = Some(cell(value));
            (id, node.info)
        });
        Signal {
            id,
            info,
            marker: PhantomData,
        }
    }

    /// Kind, name and creation site, for messages.
    #[must_use]
    pub fn info(&self) -> NodeInfo {
        self.info
    }

    /// Reads through `f`, subscribing the running reader.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_with<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, Error> {
        let cell = read_cell(self.id, self.info, true)?;
        with_cell(&cell, self.info, f)
    }

    /// Reads through `f` without subscribing.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, Error> {
        let cell = read_cell(self.id, self.info, false)?;
        with_cell(&cell, self.info, f)
    }

    /// A clone of the value, subscribing the running reader.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_get(&self) -> Result<T, Error>
    where
        T: Clone,
    {
        self.try_with(Clone::clone)
    }

    /// A clone of the value without subscribing.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_get_untracked(&self) -> Result<T, Error>
    where
        T: Clone,
    {
        self.try_with_untracked(Clone::clone)
    }

    /// Replaces the value and notifies readers.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_set(&self, value: T) -> Result<(), Error> {
        write_cell(self.id, self.info, true, |slot| *slot = value)
    }

    /// Edits the value in place and notifies readers.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_update<R>(&self, f: impl FnOnce(&mut T) -> R) -> Result<R, Error> {
        write_cell(self.id, self.info, true, f)
    }

    /// Edits the value without notifying anyone.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_update_untracked<R>(&self, f: impl FnOnce(&mut T) -> R) -> Result<R, Error> {
        write_cell(self.id, self.info, false, f)
    }

    /// Removes the signal from the graph; later reads are refused.
    pub fn dispose(self) {
        dispose_node(self.id);
    }
}
