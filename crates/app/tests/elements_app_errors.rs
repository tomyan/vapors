//! The app installs the platform's error report for listeners, so a
//! listener's `Err` is reported (`report_error`) rather than panicking
//! as it does with no report installed.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::ListenerResult;
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
