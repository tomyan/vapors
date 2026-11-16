//! The cargo profile this harness was built with, as its directory under
//! `target` names it, and the one judged runs need: `judged`, release
//! with one codegen unit (the workspace `Cargo.toml` says why). A judged
//! scaling run built any other way is refused, since its numbers move
//! with how the compiler split the code.

/// The profile directory this binary was built into (`build.rs`).
pub const BUILT: &str = env!("VAPORS_PERF_PROFILE_DIR");
/// The profile judged runs are built with.
pub const JUDGED: &str = "judged";

/// The profile, as each report's header states it.
#[must_use]
pub fn header() -> String {
    if BUILT == JUDGED {
        format!("{BUILT} (release, codegen-units 1)")
    } else {
        format!("{BUILT} (not the judged profile)")
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
