//! The panicking conveniences of `Signal<T>`: the same operations as
//! the `try_` forms, with the refusal's message as the panic.

use crate::signal::Signal;

impl<T: 'static> Signal<T> {
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

    /// Like [`Self::try_set`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    pub fn set(&self, value: T) {
        self.try_set(value)
            .unwrap_or_else(|error| panic!("{error}"));
    }

    /// Like [`Self::try_update`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.try_update(f).unwrap_or_else(|error| panic!("{error}"));
    }

    /// Like [`Self::try_update_untracked`]; panics with the refusal's message.
    ///
    /// # Panics
    /// With the same message the `try_` form returns as an error.
    pub fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.try_update_untracked(f)
            .unwrap_or_else(|error| panic!("{error}"));
    }
}
