//! What a sandbox may take of the host (review of #473-#475): its memory
//! and its tasks, enforced by a systemd user scope around bwrap, so a
//! session that leaks or forks without end stops at its own limit rather
//! than at the host's, which every builder shares. The tmpfs mounts have
//! sizes of their own (`api_sandbox.rs`).

use std::path::Path;
use std::process::{Command, ExitCode};

/// The most memory a sandbox may use, page cache and tmpfs included.
/// Measured 2026-11-17: `cargo xtask` lint, wasm, unit and gpu inside a
/// fresh sandbox peaked at 24.4 GB under a 40 GB ceiling (memory.peak,
/// mostly the cache of what the build wrote), with no OOM event; 32 GB is
/// that and a third.
pub const MEMORY_MAX: &str = "32G";

/// The most tasks (processes and threads) a sandbox may run. Measured in
/// the same run: 1,344 at peak (pids.peak); 4,096 is about three times
/// that, far below the host's own limit, so a fork bomb stops here.
pub const TASKS_MAX: &str = "4096";

/// [`MEMORY_MAX`] in bytes, as the cgroup's `memory.max` shows it.
pub const MEMORY_MAX_BYTES: u64 = 32 * 1024 * 1024 * 1024;

/// systemd-run, ready for the command it scopes: a transient user scope
/// with the limits, no swap to stretch them, and nothing printed; the
/// trusted `xtask` checks the limits are in force (`api-scoped`) before
/// it runs the command.
pub fn scope(xtask: &Path) -> Command {
    let mut scope = Command::new("systemd-run");
    scope.args(["--user", "--scope", "--quiet", "--collect"]);
    for property in [
        format!("MemoryMax={MEMORY_MAX}"),
        "MemorySwapMax=0".to_owned(),
        format!("TasksMax={TASKS_MAX}"),
    ] {
        scope.args(["-p", &property]);
    }
    scope.arg("--").arg(xtask).args(["api-scoped", "--"]);
    scope
}

/// `xtask api-scoped -- <command>`: inside the scope, reads the scope's
/// cgroup and runs `command` only if its memory and task limits are the
/// ones asked for. A user manager without the memory or pids controller
/// accepts the properties and enforces nothing, saying nothing (review of
/// #476-#478, M5); this says so and stops.
pub fn scoped(command: &[String]) -> ExitCode {
    let Some((program, args)) = command.split_first() else {
        eprintln!("FAIL api-scoped: no command to run");
        return ExitCode::FAILURE;
    };
    let cgroup = std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    if let Err(why) = in_force(&cgroup, Path::new("/sys/fs/cgroup")) {
        eprintln!("FAIL api-scoped: {why}");
        return ExitCode::FAILURE;
    }
    match Command::new(program).args(args).status() {
        Ok(status) => ExitCode::from(
            status
                .code()
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1),
        ),
        Err(error) => {
            eprintln!("FAIL api-scoped: the command did not start: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The limits of the cgroup `/proc/self/cgroup` names (`cgroup`), read
/// under `root`.
///
/// # Errors
///
/// A cgroup line of another shape, or a limit absent or not the one asked
/// for, each named with the fix.
pub fn in_force(cgroup: &str, root: &Path) -> Result<(), String> {
    let path = cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .ok_or_else(|| format!("/proc/self/cgroup has no cgroup v2 line: {cgroup:?}"))?;
    let dir = root.join(path.trim_start_matches('/'));
    for (file, expected) in [
        ("memory.max", MEMORY_MAX_BYTES.to_string()),
        ("pids.max", TASKS_MAX.to_owned()),
    ] {
        let found = std::fs::read_to_string(dir.join(file)).map_err(|e| {
            format!("{} is not there ({e}): the scope's limits are not enforced; fix: delegate the memory and pids controllers to the user manager", dir.join(file).display())
        })?;
        if found.trim() != expected {
            return Err(format!(
                "{} is {}, not {expected}: the scope's limit is not in force",
                dir.join(file).display(),
                found.trim()
            ));
        }
    }
    Ok(())
}

/// No core dump from this process or anything it starts while it holds
/// the key (`prlimit --core=0` on itself; children inherit it), read back
/// from `/proc/self/limits`.
///
/// # Errors
///
/// prlimit failing, or the limit not 0 after it.
pub fn no_core_dumps() -> Result<(), String> {
    let pid = std::process::id().to_string();
    let set = Command::new("prlimit")
        .args(["--pid", &pid, "--core=0:0"])
        .status();
    if !set.is_ok_and(|status| status.success()) {
        return Err(
            "prlimit could not set this process's core limit to 0; fix: install util-linux".into(),
        );
    }
    let limits = std::fs::read_to_string("/proc/self/limits")
        .map_err(|e| format!("/proc/self/limits: {e}"))?;
    let core = limits
        .lines()
        .find(|line| line.starts_with("Max core file size"))
        .unwrap_or_default();
    if core.split_whitespace().skip(4).take(2).eq(["0", "0"]) {
        return Ok(());
    }
    Err(format!("the core limit is not 0 after prlimit: {core:?}"))
}

#[cfg(test)]
#[path = "api_limits_tests.rs"]
mod tests;
