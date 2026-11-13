//! `cargo xtask quiet`: take the quiet lock (`quiet_lock.rs`) by hand.
//!
//! - `quiet <minutes> -- <command>`: wait for running ci runs and gates
//!   to finish, run the command, release.
//! - `quiet hold [<minutes>]`: the same wait, then keep the lock after
//!   returning, until its deadline or `quiet release`.
//! - `quiet release`: release a hold taken from this worktree.
//!
//! Also the wait every `ci`, `test` and `gate` run makes before it
//! starts, and the gate's own hold.

use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::quiet_lock::{self, Holder, LONGEST, Quiet, WAIT};

/// A hold by hand's deadline when none is given: the lead's choice
/// (2026-11-13), not a measurement; a timed run takes minutes.
pub const DEFAULT: Duration = Duration::from_mins(15);

/// Set on the command `quiet` runs to the holder's pid, so the xtask
/// runs it starts do not wait on their own holder.
pub const HOLDER_ENV: &str = "XTASK_QUIET_HOLDER";

/// What `cargo xtask quiet` was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Run(Duration, Vec<String>),
    Hold(Duration),
    Release,
}

const FORMS: &str = "fix: `cargo xtask quiet <minutes> -- <command>`, \
                     `cargo xtask quiet hold [<minutes>]` or `cargo xtask quiet release`";

impl Action {
    /// The action `args` name.
    ///
    /// # Errors
    ///
    /// Any other form, naming the three.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let refused = || {
            format!(
                "`cargo xtask quiet {}` is not a form it takes; {FORMS}",
                words.join(" ")
            )
        };
        match words.as_slice() {
            ["release"] => Ok(Self::Release),
            ["hold"] => Ok(Self::Hold(DEFAULT)),
            ["hold", minutes] => minutes_of(minutes).map(Self::Hold).ok_or_else(refused),
            [minutes, "--", _, ..] => minutes_of(minutes)
                .map(|held| Self::Run(held, args[2..].to_vec()))
                .ok_or_else(refused),
            _ => Err(refused()),
        }
    }
}

fn minutes_of(text: &str) -> Option<Duration> {
    let minutes: u64 = text.parse().ok().filter(|&m| m > 0)?;
    Some(Duration::from_mins(minutes))
}

/// `cargo xtask quiet <args>`.
pub fn run(args: &[String]) -> ExitCode {
    let done = Action::parse(args).and_then(|action| match action {
        Action::Run(held, command) => run_quiet(held, &command),
        Action::Hold(held) => hold(held),
        Action::Release => release(),
    });
    match done {
        Ok(code) => code,
        Err(why) => crate::fail("quiet", &why),
    }
}

fn run_quiet(held: Duration, command: &[String]) -> Result<ExitCode, String> {
    let _quiet = take_and_drain(held, &command.join(" "))?;
    let status = child(command)
        .status()
        .map_err(|e| format!("`{}` could not start: {e}", command.join(" ")))?;
    println!("quiet: `{}` ended ({status}); released", command.join(" "));
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn hold(held: Duration) -> Result<ExitCode, String> {
    take_and_drain(held, "hold")?.keep();
    println!(
        "quiet: held for {} s from now; `cargo xtask quiet release` here ends it sooner",
        held.as_secs()
    );
    Ok(ExitCode::SUCCESS)
}

fn release() -> Result<ExitCode, String> {
    let dir = crate::gate_lock::dir()?;
    let here = worktree();
    match quiet_lock::read(&dir)? {
        None => println!("quiet: not held"),
        Some(holder) if holder.worktree == here => {
            std::fs::remove_file(dir.join(quiet_lock::FILE)).map_err(|e| e.to_string())?;
            println!("quiet: released");
        }
        Some(holder) => {
            return Err(format!(
                "held by {}, not from {here}; fix: release it from {}, or wait for it to expire",
                holder.describe(quiet_lock::now_ms()),
                holder.worktree
            ));
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Take the lock for `held`, then wait for running ci runs and gates to
/// finish, by taking the gate lock once and letting it go.
fn take_and_drain(held: Duration, what: &str) -> Result<Quiet, String> {
    let dir = crate::gate_lock::dir()?;
    let quiet = quiet_lock::take(&dir, Holder::me(&worktree(), held, what), WAIT)?;
    println!("quiet: taken; waiting for running ci runs and gates to finish");
    drop(crate::gate_lock::acquire(
        &dir,
        &worktree(),
        crate::gate_lock::WAIT,
    )?);
    Ok(quiet)
}

/// The command `quiet` runs, told who holds the lock.
fn child(command: &[String]) -> Command {
    let mut child = Command::new(&command[0]);
    child
        .args(&command[1..])
        .env(HOLDER_ENV, std::process::id().to_string());
    child
}

/// Before `who` (`ci`, `test`) starts: wait while someone else holds the
/// lock. Not in a gate's step, whose gate holds it.
///
/// # Errors
///
/// As [`quiet_lock::wait_clear`].
pub fn before(who: &str) -> Result<(), String> {
    let gate_env = std::env::var(crate::priority::GATE_ENV).ok();
    if crate::priority::in_gate(false, gate_env.as_deref()) {
        return Ok(());
    }
    let me = std::env::var(HOLDER_ENV)
        .ok()
        .and_then(|pid| pid.parse().ok());
    let dir = crate::gate_lock::dir()?;
    quiet_lock::wait_clear(&dir, (who, me), WAIT, &mut |line| println!("{line}"))
}

/// The gate's hold, for [`LONGEST`]; none when `quiet` started the gate
/// and holds it already.
///
/// # Errors
///
/// As [`quiet_lock::take`].
pub fn for_gate(worktree: &str) -> Result<Option<Quiet>, String> {
    if std::env::var(HOLDER_ENV).is_ok() {
        return Ok(None);
    }
    let dir = crate::gate_lock::dir()?;
    quiet_lock::take(&dir, Holder::me(worktree, LONGEST, "gate"), WAIT).map(Some)
}

fn worktree() -> String {
    std::env::current_dir().map_or_else(|_| "?".to_owned(), |d| d.display().to_string())
}

#[cfg(test)]
#[path = "quiet_command_tests.rs"]
mod tests;
