//! What a listener returns: nothing, or `Result<(), E>`, whose `Err` goes
//! to the error report the app installs (`set_error_report`; the
//! elements app installs the platform's `report_error`, which the page
//! shows), so a listener that fails, such as a component whose
//! `Dispatcher::dispatch` is refused, is always seen. With no report
//! installed it panics with the error: never dropped.

use std::cell::Cell;
use std::fmt::Display;

use crate::Event;

/// An installed error report: its name, and where errors go.
type Report = (&'static str, fn(&str));

thread_local! {
    static REPORT: Cell<Option<Report>> = const { Cell::new(None) };
}

/// Sends every listener's `Err` to `report`, named `name`, from now on,
/// on this thread. Installing the same name again keeps it.
///
/// # Panics
/// When a report of another name is installed on this thread: one
/// would silently take the other's errors, so neither is replaced
/// quietly. Install one report, before the app (`ElementsApp::new`
/// installs the platform's).
pub fn set_error_report(name: &'static str, report: fn(&str)) {
    REPORT.with(|cell| match cell.get() {
        None => cell.set(Some((name, report))),
        Some((installed, _)) if installed == name => {}
        Some((installed, _)) => panic!(
            "the error report `{installed}` is installed on this thread, and `{name}` would replace it: install one report, once"
        ),
    });
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
            Some((_, report)) => report(&message),
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
