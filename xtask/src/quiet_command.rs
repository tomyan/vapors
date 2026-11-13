//! `cargo xtask quiet`: take the quiet lock (`quiet_lock.rs`) by hand.
//!
//! - `quiet <minutes> -- <command>`: wait for running ci runs and gates
//!   to finish, run the command, release.
//! - `quiet hold [<minutes>]`: the same wait, then keep the lock after
//!   returning, until its deadline or `quiet release`.
//! - `quiet release`: release a hold taken from this worktree.
//!
//! The waits on it and the gate's hold are in `quiet_wait.rs`.

use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::quiet_lock::{self, Form, Holder, LONGEST, Quiet, WAIT};
use crate::quiet_wait::{HOLDER_ENV, worktree};

/// A hold by hand's deadline when none is given: the lead's choice
/// (2026-11-13), not a measurement; a timed run takes minutes.
pub const DEFAULT: Duration = Duration::from_mins(15);

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

/// Whole minutes from 1 to [`LONGEST`]'s; none for anything else.
fn minutes_of(text: &str) -> Option<Duration> {
    let longest = LONGEST.as_secs() / 60;
    let minutes: u64 = text.parse().ok().filter(|&m| m > 0 && m <= longest)?;
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

/// Run `command` in quiet; its exit code, or failure when the hold
/// ended before the command did, since its timings then ran beside
/// other work.
fn run_quiet(held: Duration, command: &[String]) -> Result<ExitCode, String> {
    let shown = command.join(" ");
    let quiet = take_and_drain(held, Form::Run, &shown)?;
    let status = child(command)
        .status()
        .map_err(|e| format!("`{shown}` could not start: {e}"))?;
    if !quiet.still_held() {
        return Err(format!(
            "the quiet hold of {} s ended before `{shown}` did ({status}), so the rest of it ran \
             beside other work; fix: rerun with more minutes (at most {})",
            held.as_secs(),
            LONGEST.as_secs() / 60
        ));
    }
    println!("quiet: `{shown}` ended ({status}); released");
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .map_or(ExitCode::FAILURE, ExitCode::from))
}

fn hold(held: Duration) -> Result<ExitCode, String> {
    take_and_drain(held, Form::Hold, "hold")?.keep();
    println!(
        "quiet: held for {} s from now; xtask runs from this worktree go on, others wait; \
         `cargo xtask quiet release` here ends it sooner",
        held.as_secs()
    );
    Ok(ExitCode::SUCCESS)
}

/// End a hold taken from this worktree; a gate's or a command's hold is
/// its process's to end.
fn release() -> Result<ExitCode, String> {
    let dir = crate::gate_lock::dir()?;
    let here = worktree();
    let Some(holder) = quiet_lock::current(&dir)? else {
        println!("quiet: not held");
        return Ok(ExitCode::SUCCESS);
    };
    if holder.form != Form::Hold || holder.owner.worktree != here {
        return Err(format!(
            "held by {}, not a hold from {here}; fix: release a hold from the worktree that took it, \
             or wait for it to end",
            holder.describe(quiet_lock::now_ms())
        ));
    }
    std::fs::remove_file(dir.join(quiet_lock::FILE)).map_err(|e| e.to_string())?;
    println!("quiet: released");
    Ok(ExitCode::SUCCESS)
}

/// Take the lock for `held`, then wait for running ci runs and gates to
/// finish, by taking the gate lock once and letting it go.
fn take_and_drain(held: Duration, form: Form, what: &str) -> Result<Quiet, String> {
    let dir = crate::gate_lock::dir()?;
    let quiet = quiet_lock::take(&dir, Holder::me(&worktree(), held, form, what), WAIT)?;
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

#[cfg(test)]
#[path = "quiet_command_tests.rs"]
mod tests;
