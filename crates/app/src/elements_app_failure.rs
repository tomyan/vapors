//! Why an `ElementsApp` stopped, kept for whoever ran it.

use std::cell::RefCell;
use std::rc::Rc;

/// Why the app stopped, once it has: shared with whoever ran it.
#[derive(Clone, Default)]
pub struct Failure(Rc<RefCell<Option<String>>>);

impl Failure {
    /// The report, if the app stopped on a failure.
    #[must_use]
    pub fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }

    pub(super) fn set(&self, message: &str) {
        *self.0.borrow_mut() = Some(message.to_owned());
    }
}
