//! What a listener returns: `()`, or `Result<(), E>` whose `Err` goes to
//! the installed error report, or panics with it when none is installed.

use std::cell::RefCell;

use vapors_dom::{ListenerResult, set_error_report};

thread_local! {
    static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn seen(message: &str) {
    SEEN.with(|seen| seen.borrow_mut().push(message.to_owned()));
}

#[test]
fn an_err_goes_to_the_installed_report_and_ok_to_nothing() {
    // Given
    set_error_report("seen", seen);

    // When
    Ok::<(), String>(()).report();
    ().report();
    Err::<(), String>("`pick` was refused".into()).report();

    // Then
    SEEN.with(|seen| assert_eq!(*seen.borrow(), ["a listener failed: `pick` was refused"]));
}

#[test]
#[should_panic(expected = "a listener failed: lost; no error report is installed")]
fn an_err_with_no_report_installed_panics_with_it() {
    Err::<(), &str>("lost").report();
}

#[test]
fn the_same_report_installed_again_is_kept() {
    // Given
    set_error_report("seen", seen);

    // When
    set_error_report("seen", seen);
    Err::<(), &str>("twice").report();

    // Then
    SEEN.with(|seen| assert_eq!(*seen.borrow(), ["a listener failed: twice"]));
}

#[test]
#[should_panic(
    expected = "the error report `seen` is installed on this thread, and `other` would replace it"
)]
fn another_report_over_an_installed_one_panics_naming_both() {
    set_error_report("seen", seen);
    set_error_report("other", seen);
}
