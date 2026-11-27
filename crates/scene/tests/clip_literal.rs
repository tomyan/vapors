//! A `Clip` is made only by its constructors: a literal outside
//! vapors-scene does not compile, so no clip carries a radius
//! `Clip::rounded` would refuse, or an `outer` `Clip::cut` did not set.
//! The fixture is `tests/compile_fail/clip_literal.rs`, its expected
//! error beside it.

#[test]
fn a_clip_literal_does_not_compile() {
    trybuild::TestCases::new().compile_fail("tests/compile_fail/clip_literal.rs");
}
