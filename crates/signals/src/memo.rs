//! `Memo<T>`: a cached derived value, computed lazily and only when a
//! source changed.

use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

use crate::access::{cell, read_cell, refuse_cycle, with_cell};
use crate::error::{Error, Kind, NodeInfo};
use crate::node::{NodeId, State};
use crate::owner::dispose_node;
use crate::propagate::update_if_necessary;
use crate::runtime::with_runtime;

/// A handle to a derived value. Its readers re-run only when the value
/// actually changes (by `PartialEq`).
pub struct Memo<T> {
    id: NodeId,
    info: NodeInfo,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Memo<T> {}

impl<T> fmt::Debug for Memo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.info)
    }
}

impl<T: PartialEq + 'static> Memo<T> {
    /// Creates a memo owned by the current owner. `f` does not run
    /// until the memo is first read.
    #[track_caller]
    pub fn new(f: impl FnMut() -> T + 'static) -> Self {
        Self::create(None, f, Location::caller())
    }

    /// Creates a memo that refusals name as `name`.
    #[track_caller]
    pub fn named(name: &'static str, f: impl FnMut() -> T + 'static) -> Self {
        Self::create(Some(name), f, Location::caller())
    }

    fn create(
        name: Option<&'static str>,
        mut f: impl FnMut() -> T + 'static,
        created_at: &'static Location<'static>,
    ) -> Self {
        let value: Rc<RefCell<Option<T>>> = Rc::new(RefCell::new(None));
        let compute = {
            let value = Rc::clone(&value);
            move || {
                let next = f();
                let mut slot = value.borrow_mut();
                let changed = slot.as_ref() != Some(&next);
                *slot = Some(next);
                changed
            }
        };
        let (id, info) = with_runtime(|runtime| {
            let id = runtime.create_node(Kind::Memo, name, created_at, State::Dirty);
            let scope = runtime.create_owner();
            let node = runtime.node_mut(id).expect("just created");
            node.value = Some(cell(value));
            node.compute = Some(Rc::new(RefCell::new(compute)));
            node.scope = Some(scope);
            (id, node.info)
        });
        Memo {
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

    fn read<R>(&self, tracked: bool, f: impl FnOnce(&T) -> R) -> Result<R, Error> {
        with_runtime(|runtime| refuse_cycle(runtime, self.id, self.info))?;
        update_if_necessary(self.id);
        let cell = read_cell(self.id, self.info, tracked)?;
        with_cell(&cell, self.info, |value: &Rc<RefCell<Option<T>>>| {
            f(value
                .borrow()
                .as_ref()
                .expect("computed by update_if_necessary"))
        })
    }

    /// Reads through `f`, subscribing the running reader.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_with<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, Error> {
        self.read(true, f)
    }

    /// Reads through `f` without subscribing.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, Error> {
        self.read(false, f)
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
        self.read(true, Clone::clone)
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
        self.read(false, Clone::clone)
    }

    /// Like [`Self::try_with`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.read(true, f).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Like [`Self::try_with_untracked`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.read(false, f)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Like [`Self::try_get`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.try_get().unwrap_or_else(|error| panic!("{error}"))
    }

    /// Like [`Self::try_get_untracked`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    #[must_use]
    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.try_get_untracked()
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Removes the memo from the graph, running the cleanups of its
    /// last computation; later reads are refused.
    pub fn dispose(self) {
        dispose_node(self.id);
    }
}
