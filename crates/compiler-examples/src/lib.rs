//! Worked `.wisp` components, compiled by `build.rs` with
//! `vapors_compiler::build::compile_dir("src")`. Each file under `src`
//! becomes a module here: `counter.wisp` is [`counter::Counter`].

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

/// A cleanup that panics, for `cleanup_panics.wisp`: the test that a
/// component dropped while its thread is panicking is not disposed.
///
/// # Panics
/// Always, naming the component.
pub fn panicking_cleanup() {
    panic!("the cleanup of CleanupPanics ran");
}
