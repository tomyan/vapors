//! What must not compile, one trybuild run for every fixture in
//! `tests/compile_fail/`, each with its expected error beside it: one
//! run builds the fixtures' dependencies once, where a run per crate
//! would wait on the others' build-directory lock (a cold gate's build
//! of this one took 175 s, #638).
//! - `dropped_mount.rs`: a generated component is `#[must_use]`, so a
//!   mount whose result is dropped at once (its effects disposed with
//!   it, #353) does not compile under `deny(unused_must_use)`.
//! - `clip_literal.rs`: `vapors_scene::Clip`'s fields are private, so a
//!   literal cannot carry a radius `Clip::rounded` would refuse.

#[test]
fn what_must_not_compile_fails_with_its_expected_error() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compile_fail/dropped_mount.rs");
    cases.compile_fail("tests/compile_fail/clip_literal.rs");
}
