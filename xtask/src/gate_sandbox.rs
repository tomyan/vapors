//! Every gate step runs with `/dev/dri` hidden, as on GitHub's runners,
//! which have no DRM device. With one, headless Chromium can present a
//! WebGPU canvas that it cannot present without one, so a check passed
//! here and failed on GitHub (touche's walk, runs 41316945392 to
//! 41337806803, fixed by touche #109's --use-vulkan=swiftshader).

use std::process::Command;

/// bwrap's arguments before the step's command: the whole filesystem as
/// it is, with an empty `/dev/dri`, and the step ended with the gate.
pub const HIDE_GPU_DEVICE: [&str; 6] = [
    "--dev-bind",
    "/",
    "/",
    "--tmpfs",
    "/dev/dri",
    "--die-with-parent",
];

/// Refuse a machine where `bwrap` cannot hide `/dev/dri`.
///
/// # Errors
///
/// bwrap is missing or the sandbox did not leave `/dev/dri` empty,
/// naming the fix.
pub fn check() -> Result<(), String> {
    check_with("bwrap")
}

/// [`check`] with `program` as bwrap.
///
/// # Errors
///
/// As [`check`].
pub fn check_with(program: &str) -> Result<(), String> {
    let output = Command::new(program)
        .args(HIDE_GPU_DEVICE)
        .args(["ls", "-A", "/dev/dri"])
        .output()
        .map_err(|e| {
            format!("{program} could not run ({e}); the gate hides /dev/dri with it; fix: install bubblewrap (apt install bubblewrap)")
        })?;
    if !output.status.success() || !output.stdout.is_empty() {
        return Err(format!(
            "{program} did not run with /dev/dri hidden ({}; it listed `{}`); fix: install bubblewrap, and allow unprivileged user namespaces",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "gate_sandbox_tests.rs"]
mod tests;
