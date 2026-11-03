//! Every gate step runs with `/dev/dri` hidden, as on GitHub's runners,
//! which have no DRM device. With one, headless Chromium can present a
//! WebGPU canvas that it cannot present without one, so a check passed
//! here and failed on GitHub (touche's walk, runs 41316945392 to
//! 41337806803, fixed by touche #109's --use-vulkan=swiftshader).
//! To run a step by hand outside the sandbox, drop the six arguments
//! after `bwrap` from its reproduce line.

use std::process::Command;

use crate::config::Config;

/// How the gate's steps run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    /// Through bwrap, `/dev/dri` hidden: as on GitHub.
    HideGpuDevice,
    /// As they are: `[gate] hide_gpu_device = "false"`.
    AsIs,
}

/// The key that turns the sandbox off.
pub const KEY: &str = "gate.hide_gpu_device";

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

/// How this gate runs its steps: `/dev/dri` hidden, unless this machine
/// has no device there (as GitHub's runners have none, so there is
/// nothing to hide) or `[gate] hide_gpu_device = "false"`.
///
/// # Errors
///
/// Another value for the key, or bwrap missing or not leaving
/// `/dev/dri` empty, naming the fix.
pub fn wrap(config: &Config) -> Result<Wrap, String> {
    wrap_for(config, has_gpu_device(), "bwrap")
}

/// Whether `/dev/dri` holds anything: GitHub's runner has the directory
/// but no device in it (Chromium there: "`drmGetDevices2()` has not found
/// any devices"), and no bwrap.
#[must_use]
pub fn has_gpu_device() -> bool {
    std::fs::read_dir("/dev/dri").is_ok_and(|mut entries| entries.next().is_some())
}

/// [`wrap`] on a machine with a device in `/dev/dri` or not, with
/// `program` as bwrap.
///
/// # Errors
///
/// As [`wrap`].
pub fn wrap_for(config: &Config, has_device: bool, program: &str) -> Result<Wrap, String> {
    match config.get(KEY) {
        None | Some("true") if !has_device => {
            println!(
                "gate: this machine has no device in /dev/dri, as GitHub's runners have none; steps run as they are"
            );
            Ok(Wrap::AsIs)
        }
        None | Some("true") => check_with(program).map(|()| Wrap::HideGpuDevice),
        // Relaxed guard, declared: for a machine with /dev/dri but no
        // working bwrap, steps run as they are, said on every gate; such
        // a gate can pass a check that fails on GitHub's device-less
        // runners. Retired when that machine has its own way to hide its
        // GPU from the checks.
        Some("false") => {
            println!(
                "gate: /dev/dri is not hidden ({KEY} = \"false\" in .config/factory.toml); a step can pass here that fails on GitHub"
            );
            Ok(Wrap::AsIs)
        }
        Some(other) => Err(format!(
            "[gate] hide_gpu_device is `{other}`, not true or false; fix: .config/factory.toml"
        )),
    }
}

/// [`check`] with `program` as bwrap.
///
/// # Errors
///
/// As [`check`].
pub fn check_with(program: &str) -> Result<(), String> {
    let output = Command::new(program)
        .args(HIDE_GPU_DEVICE)
        .args(["/bin/ls", "-A", "/dev/dri"])
        .output()
        .map_err(|e| {
            format!("{program} could not run ({e}); the gate hides /dev/dri with it; fix: install bubblewrap (apt install bubblewrap), or on a machine where it cannot run set [gate] hide_gpu_device = \"false\" in .config/factory.toml")
        })?;
    if !output.status.success() || !output.stdout.is_empty() {
        return Err(format!(
            "{program} did not run with /dev/dri hidden ({}; it listed `{}`); fix: allow unprivileged user namespaces, or set [gate] hide_gpu_device = \"false\" in .config/factory.toml",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "gate_sandbox_tests.rs"]
mod tests;
