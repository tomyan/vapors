//! A poll on a wake: a task's `Err` is reported naming its spawn site,
//! and the time polls took is kept for the next frame's Tasks phase.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_elements::Document;
use vapors_signals::{Deadline, spawn};
use vapors_text::FontSystem;

use super::ElementsApp;

fn app() -> ElementsApp {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let app = ElementsApp::new(Rc::clone(&document));
    document.borrow_mut().take_changed();
    app
}

#[test]
fn a_tasks_err_is_reported_naming_its_spawn_site_and_owes_a_frame() {
    // Given
    let mut app = app();
    let line = line!() + 1;
    spawn(Deadline::User, async { Err::<(), _>("the fetch failed") });

    // When
    let mut reports = Vec::new();
    let owed = app.poll_tasks(&mut |failure| reports.push(failure.to_owned()));

    // Then
    assert!(owed, "a failed task owed no frame");
    let [report] = reports.as_slice() else {
        panic!("one report expected, got {reports:?}");
    };
    let site = format!("elements_app_tasks_tests.rs:{line}:");
    assert!(
        report.contains(&site) && report.contains("the fetch failed"),
        "the report names neither the site {site} nor the error: {report}"
    );
}

#[test]
fn the_time_a_poll_took_is_kept_for_the_tasks_phase() {
    // Given: a task whose poll takes 2 ms
    let mut app = app();
    spawn(Deadline::User, async {
        std::thread::sleep(Duration::from_millis(2));
        Ok::<(), String>(())
    });

    // When
    let _ = app.poll_tasks(&mut |_| {});

    // Then
    assert!(
        app.tasks_spent >= Duration::from_millis(2),
        "the poll's time was not kept: {:?}",
        app.tasks_spent
    );
}
