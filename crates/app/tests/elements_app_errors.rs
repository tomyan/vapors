//! The app installs the platform's error report for listeners, so a
//! listener's `Err` is reported (`report_error`) rather than panicking
//! as it does with no report installed; it refuses to replace a report
//! installed before it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{ListenerResult, set_error_report};
use vapors_elements::Document;
use vapors_text::FontSystem;

#[test]
fn a_listener_err_is_reported_once_an_app_exists() {
    // Given
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let _app = ElementsApp::new(doc);

    // When: with no report installed this panics
    Err::<(), &str>("`pick` was refused").report();

    // Then: it returned
}

#[test]
#[should_panic(
    expected = "the error report `the test's` is installed on this thread, and `vapors_platform::report_error` would replace it"
)]
fn an_app_over_an_installed_report_panics_naming_both() {
    // Given
    set_error_report("the test's", |_| {});
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));

    // When
    let _app = ElementsApp::new(doc);
}
