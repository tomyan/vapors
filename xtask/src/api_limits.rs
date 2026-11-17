//! What a sandbox may take of the host (review of #473-#475): its memory
//! and its tasks, enforced by a systemd user scope around bwrap, so a
//! session that leaks or forks without end stops at its own limit rather
//! than at the host's, which every builder shares. The tmpfs mounts have
//! sizes of their own (`api_sandbox.rs`).

use std::process::Command;

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

/// systemd-run, ready for the command it scopes: a transient user scope
/// with the limits, no swap to stretch them, and nothing printed.
pub fn scope() -> Command {
    let mut scope = Command::new("systemd-run");
    scope.args(["--user", "--scope", "--quiet", "--collect"]);
    for property in [
        format!("MemoryMax={MEMORY_MAX}"),
        "MemorySwapMax=0".to_owned(),
        format!("TasksMax={TASKS_MAX}"),
    ] {
        scope.args(["-p", &property]);
    }
    scope.arg("--");
    scope
}

#[cfg(test)]
#[path = "api_limits_tests.rs"]
mod tests;
