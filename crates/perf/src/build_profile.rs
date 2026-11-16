//! The cargo profile this harness was built with, as its directory under
//! `target` names it, and the one judged runs need: `judged`, release
//! with one codegen unit (the workspace `Cargo.toml` says why). A judged
//! scaling run built any other way is refused, since its numbers move
//! with how the compiler split the code. The build script fails a
//! judged build that any override it can see would change: a
//! `CARGO_PROFILE_JUDGED_*` variable, codegen units in the rustflags, or
//! a `[profile.judged]` codegen-units in a cargo config on the path or in
//! cargo's home (`build_judged.rs`). It cannot see `cargo --config`
//! overrides or a config's inline tables; `build_judged.rs` names both.

/// The profile directory this binary was built into (`build.rs`).
pub const BUILT: &str = env!("VAPORS_PERF_PROFILE_DIR");
/// The profile judged runs are built with.
pub const JUDGED: &str = "judged";

/// The profile, as each report's header states it.
#[must_use]
pub fn header() -> String {
    if BUILT == JUDGED {
        format!("{BUILT} (release, codegen-units 1, no override the build could see)")
    } else {
        format!("{BUILT} (not the judged profile)")
    }
}

/// The profile a scene run states: scene runs are held to release, the
/// shipped profile (`scene_run.rs`), so `built` is named with whether it
/// is that one.
#[must_use]
pub fn scene_header(built: &str) -> String {
    if built == "release" {
        "release (the shipped profile, which scene runs are judged against)".to_owned()
    } else {
        format!("{built} (not release, the shipped profile scene runs are judged against)")
    }
}

/// The profile a Mount run states. Its times are property 7's, the first
/// frame a user waits for, so they are taken on release, the shipped
/// profile, as scene runs are; `scripts/first-frame-table.sh` refuses a
/// run whose header says otherwise.
#[must_use]
pub fn shipped_header(built: &str) -> String {
    if built == "release" {
        "release (the shipped profile)".to_owned()
    } else {
        format!("{built} (not release, the shipped profile)")
    }
}

/// Refuse a judged run unless built with [`JUDGED`].
///
/// # Errors
/// Built with another profile, naming it and the build to use.
pub fn judged(built: &str) -> Result<(), String> {
    if built == JUDGED {
        return Ok(());
    }
    Err(format!(
        "a judged --scaling run built with the {built} profile; its numbers move with codegen units, \
         so it is not judged; fix: cargo build --profile {JUDGED} -p vapors-perf, then run \
         target/{JUDGED}/vapors-perf"
    ))
}

#[cfg(test)]
#[path = "build_profile_tests.rs"]
mod tests;
