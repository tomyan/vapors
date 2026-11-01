//! What a listener returns: nothing, or `Result<(), E>`, whose `Err` goes
//! to the error report the app installs (`set_error_report`; the
//! elements app installs the platform's `report_error`, which the page
//! shows), so a listener that fails, such as a component whose
//! `Dispatcher::dispatch` is refused, is always seen. With no report
//! installed it panics with the error: never dropped.

use std::cell::Cell;
use std::fmt::Display;

use crate::Event;

thread_local! {
    static REPORT: Cell<Option<fn(&str)>> = const { Cell::new(None) };
}

/// Sends every listener's `Err` to `report` from now on, on this thread.
pub fn set_error_report(report: fn(&str)) {
    REPORT.with(|cell| cell.set(Some(report)));
}

/// A listener's return value, which the compiler's listener wrapper
/// hands on.
pub trait ListenerResult {
    /// Reports a failure; nothing for success.
    fn report(self);
}

impl ListenerResult for () {
    fn report(self) {}
}

impl<E: Display> ListenerResult for Result<(), E> {
    /// # Panics
    /// With no error report installed, an `Err` panics with it, so it is
    /// never dropped.
    fn report(self) {
        let Err(error) = self else { return };
        let message = format!("a listener failed: {error}");
        match REPORT.with(Cell::get) {
            Some(report) => report(&message),
            None => {
                panic!("{message}; no error report is installed (vapors_dom::set_error_report)")
            }
        }
    }
}

/// Calls a template's handler with `event` and reports what it returns:
/// the listener the compiler wraps around an `on<name>` handler.
pub fn handle<R: ListenerResult>(event: &mut Event, handler: &impl Fn(&mut Event) -> R) {
    handler(event).report();
}

/// [`handle`] for a script handler that takes no event.
pub fn handle_without_event<R: ListenerResult>(handler: &impl Fn() -> R) {
    handler().report();
}
