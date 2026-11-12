//! The class behind review-fable-input H1 and review-v220: for every
//! event the tree fires, a listener on each control that removes the
//! control (or its parent), then every input path that can fire it.
//! No path may panic, and the document must still update.

mod support;

use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;

use support::sweep_page::{CONTROLS, Removed, page, remove_on, the_viewport};
use support::sweep_paths::{PATHS, Path};
use vapors_elements::FIRED_EVENTS;

/// The case's panic message, or its update's refusal; `None` when it
/// ran and the document still updates.
fn failure(event: &str, control: usize, removed: Removed, path: Path) -> Option<String> {
    let run = panic::catch_unwind(AssertUnwindSafe(|| {
        IN_CASE.with(|in_case| in_case.set(true));
        let page = page(control);
        remove_on(&page, event, removed);
        path(&page);
        page.doc
            .borrow_mut()
            .update(the_viewport())
            .err()
            .map(|e| format!("update refused: {e:?}"))
    }));
    IN_CASE.with(|in_case| in_case.set(false));
    match run {
        Ok(refused) => refused,
        Err(payload) => Some(panic_message(&*payload)),
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    let text = payload.downcast_ref::<String>().map(String::as_str);
    text.or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("a panic with no message")
        .to_owned()
}

thread_local! {
    /// Set while a case runs: its panic is reported by `failure`, not
    /// printed by the default hook.
    static IN_CASE: Cell<bool> = const { Cell::new(false) };
}

/// The default hook, silent for the cases' own panics.
fn quiet_cases() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let default = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !IN_CASE.with(Cell::get) {
                default(info);
            }
        }));
    });
}

/// Every event, removal and path on the control at `control`.
fn sweep(control: usize) {
    // Given
    quiet_cases();
    let name = CONTROLS[control].0;

    // When
    let mut failures = Vec::new();
    for event in FIRED_EVENTS {
        for removed in [Removed::Target, Removed::Parent] {
            for (path_name, path) in PATHS {
                if let Some(why) = failure(event, control, removed, *path) {
                    failures.push(format!(
                        "{event} removes {removed:?} of {name}, then {path_name}: {why}"
                    ));
                }
            }
        }
    }

    // Then
    assert!(
        failures.is_empty(),
        "{} cases panicked or left the document refusing updates; re-resolve the node with `still_in` after the dispatch (cargo test -p vapors-elements --test listener_removal_sweep removes_the_{name}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn no_path_panics_when_a_listener_removes_the_button() {
    sweep(0);
}

#[test]
fn no_path_panics_when_a_listener_removes_the_checkbox() {
    sweep(1);
}

#[test]
fn no_path_panics_when_a_listener_removes_the_text() {
    sweep(2);
}

#[test]
fn no_path_panics_when_a_listener_removes_the_number() {
    sweep(3);
}

#[test]
fn no_path_panics_when_a_listener_removes_the_scrub() {
    sweep(4);
}

#[test]
fn no_path_panics_when_a_listener_removes_the_range() {
    sweep(5);
}
