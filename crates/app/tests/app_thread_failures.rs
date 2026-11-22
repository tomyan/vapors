//! The application thread's failures are loud: a task's `Err` reaches
//! the report naming its spawn site, and a panic, while the app side is
//! built or in a handler, is returned rather than lost.

use std::sync::Mutex;

use vapors_app::{AppThread, AppThreadError};
use vapors_signals::{Deadline, spawn};

use super::app_thread::{ignore, wait_for};

static REPORTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn keep(report: &str) {
    REPORTS.lock().unwrap().push(report.to_owned());
}

#[test]
fn a_tasks_err_reaches_the_report_naming_its_spawn_site() {
    // Given
    let line = line!() + 4;
    let app = AppThread::start(
        || {
            |(): ()| {
                spawn(Deadline::User, async { Err::<(), _>("the read failed") });
            }
        },
        keep,
    )
    .expect("the thread starts");

    // When
    app.send(()).expect("the thread runs");

    // Then
    wait_for("the report", || !REPORTS.lock().unwrap().is_empty());
    let report = REPORTS.lock().unwrap()[0].clone();
    let site = format!("app_thread_failures.rs:{line}:");
    assert!(
        report.contains(&site) && report.contains("the read failed"),
        "the report names neither {site} nor the error: {report}"
    );
    app.stop().expect("the thread stops");
}

#[test]
fn a_panicking_handler_stops_the_thread_and_stop_returns_why() {
    // Given
    let app = AppThread::start(|| |(): ()| panic!("the handler broke"), ignore)
        .expect("the thread starts");

    // When
    app.send(()).expect("the thread runs");

    // Then: later sends are refused, and stop names the panic
    wait_for("the refusal", || {
        app.send(()) == Err(AppThreadError::Stopped)
    });
    let stopped = app
        .stop()
        .expect_err("a panicked thread stops with its panic");
    assert!(
        stopped.to_string().contains("the handler broke"),
        "{stopped}"
    );
}

#[test]
fn a_panic_while_building_the_app_side_is_returned_by_start() {
    // When
    let started = AppThread::<()>::start(
        || -> fn(()) { panic!("the app side could not be built") },
        ignore,
    );

    // Then
    let refused = started.err().expect("start refuses");
    assert!(
        refused.to_string().contains("could not be built"),
        "{refused}"
    );
}
