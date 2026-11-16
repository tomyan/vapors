//! A top-level component dropped while its thread is panicking is not
//! disposed (the generated `Drop`, #353): its cleanups may panic, and a
//! panic during unwinding aborts the process, losing the first panic's
//! message. `src/cleanup_panics.wisp` registers a cleanup that panics.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use vapors_compiler_examples::cleanup_panics::{CleanupPanics, CleanupPanicsProps};
use vapors_dom::recording::RecordingDom;

#[test]
fn a_component_dropped_by_unwinding_keeps_the_first_panic_s_message() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When: the component is dropped by the unwinding of a panic
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let _page = CleanupPanics::mount(&dom, root, CleanupPanicsProps::default());
        panic!("the first panic");
    }));

    // Then: the first panic arrives, the cleanup did not run during it
    let payload = unwound.expect_err("the closure panicked");
    let message = payload.downcast_ref::<&str>().copied();
    assert_eq!(message, Some("the first panic"));
}

#[test]
fn a_component_dropped_otherwise_runs_its_cleanups() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let page = CleanupPanics::mount(&dom, root, CleanupPanicsProps::default());

    // When
    let dropped = catch_unwind(AssertUnwindSafe(|| drop(page)));

    // Then: its cleanup ran (and panicked, as it is written to)
    let payload = dropped.expect_err("the cleanup panics");
    let message = payload.downcast_ref::<&str>().copied();
    assert_eq!(message, Some("the cleanup of CleanupPanics ran"));
}
