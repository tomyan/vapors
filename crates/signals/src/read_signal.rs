//! `ReadSignal<T>`: the reading half of a signal or a memo.

use std::fmt;

use crate::error::{Error, NodeInfo};
use crate::memo::Memo;
use crate::signal::Signal;

/// A read-only, `Copy` view of a [`Signal`] or a [`Memo`]. It reads and
/// tracks as the node it wraps does, and has no way to write.
pub enum ReadSignal<T> {
    /// A view of a signal.
    Signal(Signal<T>),
    /// A view of a memo.
    Memo(Memo<T>),
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ReadSignal<T> {}

impl<T> fmt::Debug for ReadSignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadSignal::Signal(signal) => signal.fmt(f),
            ReadSignal::Memo(memo) => memo.fmt(f),
        }
    }
}

impl<T> From<Signal<T>> for ReadSignal<T> {
    fn from(signal: Signal<T>) -> Self {
        ReadSignal::Signal(signal)
    }
}

impl<T> From<Memo<T>> for ReadSignal<T> {
    fn from(memo: Memo<T>) -> Self {
        ReadSignal::Memo(memo)
    }
}

impl<T: 'static> Signal<T> {
    /// A read-only view of this signal: hand it where the receiver
    /// may read but not write.
    #[must_use]
    pub fn read_only(self) -> ReadSignal<T> {
        self.into()
    }
}

impl<T: 'static> ReadSignal<T> {
    /// Kind, name and creation site, for messages.
    #[must_use]
    pub fn info(&self) -> NodeInfo {
        match self {
            ReadSignal::Signal(signal) => signal.info(),
            ReadSignal::Memo(memo) => memo.info(),
        }
    }

    /// Reads through `f`, subscribing the running reader.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_with<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, Error> {
        match self {
            ReadSignal::Signal(signal) => signal.try_with(f),
            ReadSignal::Memo(memo) => memo.try_with(f),
        }
    }

    /// Reads through `f` without subscribing.
    ///
    /// # Errors
    /// Refused by name when the node was disposed, when the read would
    /// close a cycle, or when its value is borrowed by an enclosing `with`.
    pub fn try_with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, Error> {
        match self {
            ReadSignal::Signal(signal) => signal.try_with_untracked(f),
            ReadSignal::Memo(memo) => memo.try_with_untracked(f),
        }
    }

    /// A clone of the value, subscribing the running reader.
    ///
    /// # Errors
    /// As [`Self::try_with`].
    pub fn try_get(&self) -> Result<T, Error>
    where
        T: Clone,
    {
        self.try_with(Clone::clone)
    }

    /// A clone of the value without subscribing.
    ///
    /// # Errors
    /// As [`Self::try_with_untracked`].
    pub fn try_get_untracked(&self) -> Result<T, Error>
    where
        T: Clone,
    {
        self.try_with_untracked(Clone::clone)
    }

    /// Like [`Self::try_with`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.try_with(f).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Like [`Self::try_with_untracked`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.try_with_untracked(f)
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
        self.with(Clone::clone)
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
        self.with_untracked(Clone::clone)
    }
}
