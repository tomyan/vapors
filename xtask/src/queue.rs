//! `cargo xtask queue [--dry-run]`: one pass of the merge queue, then
//! exit; a systemd user timer runs it (`xtask/systemd/`), so no agent need
//! be awake. It reads `[queue]` (`queue_config.rs`), rotates its log,
//! takes its pass lock (`queue_lock.rs`), and runs the pass
//! (`queue_pass.rs`) over the real effects (`queue_real.rs`), with what
//! it remembers between passes (`queue_state.rs`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::config::Config;
use crate::queue_config::QueueConfig;
use crate::queue_log::{Log, rotate, utc};
use crate::queue_real::Real;
use crate::queue_state::State;

/// Run one pass; with `dry_run`, judge and log only.
pub fn run(dry_run: bool) -> ExitCode {
    let config = match load() {
        Ok(config) => config,
        Err(why) => {
            println!("FAIL queue: {why}");
            return ExitCode::FAILURE;
        }
    };
    let log = Log {
        path: config.log.clone(),
    };
    match locked_pass(&config, &log, dry_run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            let _ = log.line(&format!("FAILED pass: {why}"));
            ExitCode::FAILURE
        }
    }
}

fn load() -> Result<QueueConfig, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_owned())?;
    let config = Config::load().map_err(|e| e.to_string())?;
    QueueConfig::from(&config, &home)
}

/// The pass under its lock, its log rotated, its state kept.
fn locked_pass(config: &QueueConfig, log: &Log, dry_run: bool) -> Result<(), String> {
    if let Some(dir) = config.log.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    rotate(&config.log, &today())?;
    let _held = crate::queue_lock::take(&beside(&config.log, "lock"))?;
    worktree(&config.run)?;
    let state_file = beside(&config.log, "state.json");
    let mut state = State::load(&state_file)?;
    let passed = crate::queue_pass::pass(&Real { config }, config, log, &mut state, dry_run);
    state.save(&state_file)?;
    passed
}

/// The queue's run worktree exists.
fn worktree(run: &Path) -> Result<(), String> {
    if run.join(".git").exists() {
        return Ok(());
    }
    Err(format!(
        "{} is not a worktree; fix: git worktree add --detach {} origin/main",
        run.display(),
        run.display()
    ))
}

/// `<log>.<extension>`, beside the log.
fn beside(log: &Path, extension: &str) -> PathBuf {
    let name = log
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    log.with_file_name(format!("{name}.{extension}"))
}

/// Today in UTC, `YYYY-MM-DD`.
fn today() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    utc(seconds)[..10].to_owned()
}
