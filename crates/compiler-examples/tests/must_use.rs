//! A generated component is `#[must_use]`: a mount whose result is
//! dropped at once (its effects disposed with it, #353) does not
//! compile under `deny(unused_must_use)`. The fixture is
//! `tests/compile_fail/dropped_mount.rs`, its expected error beside it.

#[test]
fn a_mount_whose_component_is_dropped_at_once_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/compile_fail/dropped_mount.rs");
}
