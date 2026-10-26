//! `cargo xtask queue`: one pass of the merge queue, then exit; a
//! systemd user timer runs it, so no agent need be awake. It reads
//! `[queue]` (`queue_config.rs`) and judges each open PR oldest first
//! (`queue_decide.rs`, from its comments and git), logging every
//! decision. Then it acts on the first PR that is ready: it merges it,
//! or rebases it in the queue's own worktree and gates it. One action a
//! pass: after a merge main has moved, and a gate takes the machine's
//! one gate slot. A PR that cannot be judged (git or gh refusing) is
//! logged and skipped, never acted on.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crate::config::Config;
use crate::queue_comments::{cleared, gate, held};
use crate::queue_config::{MergeOn, QueueConfig};
use crate::queue_decide::{Clear, Decision, Facts, decide};
use crate::queue_log::Log;
use crate::queue_remote::{self, Listed};

const MAIN: &str = "origin/main";

/// Run one pass; with `dry_run`, judge and log only.
pub fn run(dry_run: bool) -> ExitCode {
    let config = std::env::var("HOME")
        .map_err(|_| "HOME is not set".to_owned())
        .and_then(|home| {
            let config = Config::load().map_err(|e| e.to_string())?;
            QueueConfig::from(&config, &home)
        });
    let config = match config {
        Ok(config) => config,
        Err(why) => return refuse(&why),
    };
    let log = Log {
        path: config.log.clone(),
    };
    match pass(&config, &log, dry_run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            let _ = log.line(&format!("FAILED pass: {why}"));
            ExitCode::FAILURE
        }
    }
}

fn refuse(why: &str) -> ExitCode {
    println!("FAIL queue: {why}");
    ExitCode::FAILURE
}

fn pass(config: &QueueConfig, log: &Log, dry_run: bool) -> Result<(), String> {
    clean_worktree(&config.run)?;
    queue_remote::fetch(&config.run, &["main"])?;
    let prs = queue_remote::open_prs(&config.repo)?;
    for pr in &prs {
        let judged = judge(config, pr);
        if dry_run {
            log.line(&dry(pr.number, &judged))?;
            continue;
        }
        match judged {
            Ok((Decision::Merge | Decision::Regate, _)) if !current(config, log, pr)? => {
                return Ok(());
            }
            Ok((Decision::Merge, _)) => return crate::queue_act::merge(config, log, pr, &prs),
            Ok((Decision::Regate, behind)) => {
                return crate::queue_act::regate(config, log, pr, behind);
            }
            Ok((Decision::Skip(why), _)) => log.line(&why)?,
            Err(why) => log.line(&format!("#{} could not be judged: {why}", pr.number))?,
        }
    }
    log.line(&format!("pass over {} open PRs: nothing to do", prs.len()))
}

/// How long to wait before reading a PR's head again when GitHub's
/// answers disagree. Seen 2026-10-26: after a push, `gh pr view` gave the
/// old head for one or two 3 s retries, about 6 s; 10 s is over that.
const HEAD_RECHECK: Duration = Duration::from_secs(10);

/// Whether `pr`'s head, read again just before acting, is the one judged.
/// A mismatch is read once more after `HEAD_RECHECK`; both are logged,
/// and a head that still differs ends the pass (the next one relists).
fn current(config: &QueueConfig, log: &Log, pr: &Listed) -> Result<bool, String> {
    let now = crate::gate_pr_gh::find_in(&config.repo, pr.number)?.head;
    if now == pr.head {
        return Ok(true);
    }
    log.line(&format!(
        "#{} was listed at {} but reads {now}; reading again in {HEAD_RECHECK:?}",
        pr.number, pr.head
    ))?;
    std::thread::sleep(HEAD_RECHECK);
    let again = crate::gate_pr_gh::find_in(&config.repo, pr.number)?.head;
    let same = again == pr.head;
    let said = if same {
        "matches the judged head"
    } else {
        "still differs; the next pass judges it again"
    };
    log.line(&format!("#{} reads {again}: {said}", pr.number))?;
    Ok(same)
}

/// What a dry run logs for PR `number`.
fn dry(number: u64, judged: &Result<(Decision, bool), String>) -> String {
    match judged {
        Ok((Decision::Merge, _)) => format!("dry run: would merge #{number}"),
        Ok((Decision::Regate, behind)) => {
            format!("dry run: would gate #{number} (behind main: {behind})")
        }
        Ok((Decision::Skip(why), _)) => format!("dry run: {why}"),
        Err(why) => format!("dry run: #{number} could not be judged: {why}"),
    }
}

/// The queue's worktree exists and has no changes (a rebase left half
/// done is aborted first).
fn clean_worktree(run: &Path) -> Result<(), String> {
    if !run.join(".git").exists() {
        return Err(format!(
            "{} is not a worktree; fix: git worktree add --detach {} origin/main",
            run.display(),
            run.display()
        ));
    }
    let _ = crate::queue_git::git(run, &["rebase", "--abort"]);
    match crate::queue_git::git(run, &["status", "--porcelain"])?.as_str() {
        "" => Ok(()),
        changes => Err(format!(
            "{} has changes ({changes}); fix: clean it by hand",
            run.display()
        )),
    }
}

/// The decision for `pr`, and whether it is behind main.
fn judge(config: &QueueConfig, pr: &Listed) -> Result<(Decision, bool), String> {
    let mut facts = Facts {
        number: pr.number,
        head: pr.head.clone(),
        base_is_main: pr.base == "main",
        draft: pr.draft,
        held: held(&pr.comments),
        clear: Clear::Missing,
        contains_main: true,
        gate: crate::queue_decide::Gate::None,
        rebase_clean: true,
    };
    if let Decision::Skip(why) = decide(&facts_before_git(&facts)) {
        return Ok((Decision::Skip(why), false));
    }
    queue_remote::fetch(&config.run, &[&pr.head])?;
    facts.clear = clear(config, pr)?;
    if facts.clear != Clear::Holds {
        return Ok((decide(&facts), false));
    }
    facts.contains_main = crate::queue_git::contains(&config.run, MAIN, &pr.head)?;
    facts.gate = match config.merge_on {
        MergeOn::Gate => gate(&pr.comments, &pr.head),
        MergeOn::Checks => crate::queue_checks::checks(&config.repo, pr.number)?,
    };
    if !facts.contains_main && config.merge_on == MergeOn::Gate {
        facts.rebase_clean = crate::queue_git::rebase_clean(&config.run, MAIN, &pr.head)?.is_some();
    }
    Ok((decide(&facts), !facts.contains_main))
}

/// The facts as they stand before any git call: a clear assumed to hold,
/// so only what the comments and gh say can make it wait.
fn facts_before_git(facts: &Facts) -> Facts {
    let clear = if facts.clear == Clear::Missing {
        Clear::Holds
    } else {
        facts.clear
    };
    Facts {
        clear,
        gate: crate::queue_decide::Gate::Passed,
        ..facts.clone()
    }
}

/// The PR's review clear, judged by patch ids against the cleared sha.
fn clear(config: &QueueConfig, pr: &Listed) -> Result<Clear, String> {
    let Some(sha) = cleared(&pr.comments) else {
        return Ok(Clear::Missing);
    };
    queue_remote::fetch(&config.run, &[&sha])?;
    crate::queue_git::clear_holds(&config.run, MAIN, &pr.head, &sha)
}
