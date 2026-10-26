//! Who holds the gate lock, as `gate.holder` and each `ci.<pid>` marker
//! say: `pid <P> in <worktree> since <unix seconds>`. An owner is live
//! while its process exists and started no later than its stamp; a
//! process that started after it holds a reused pid. Read from /proc, so
//! Linux only (a declared limit: elsewhere every owner reads as gone).

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// This process's line, for the worktree it runs in.
#[must_use]
pub fn line(worktree: &str) -> String {
    format!("pid {} in {worktree} since {}\n", std::process::id(), now())
}

/// A parsed line: pid, worktree and stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    pub pid: u32,
    pub worktree: String,
    pub since: u64,
}

impl Owner {
    /// `text`'s owner, or none for a line in another shape.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text.trim().strip_prefix("pid ")?;
        let (pid, rest) = rest.split_once(" in ")?;
        let (worktree, since) = rest.rsplit_once(" since ")?;
        Some(Self {
            pid: pid.parse().ok()?,
            worktree: worktree.to_owned(),
            since: since.parse().ok()?,
        })
    }

    /// Whether its process is still the one that wrote the line.
    #[must_use]
    pub fn live(&self) -> bool {
        process_start(self.pid).is_some_and(|start| start <= self.since + START_SLACK)
    }

    /// How long ago it took the lock, in seconds.
    #[must_use]
    pub fn held_for(&self) -> u64 {
        now().saturating_sub(self.since)
    }
}

/// The start time from /proc is rounded to whole seconds and the stamp is
/// written after start: a little slack keeps a live owner live.
const START_SLACK: u64 = 2;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// When process `pid` started, in unix seconds: boot time (/proc/stat's
/// btime) plus its starttime field in clock ticks.
fn process_start(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = &stat[stat.rfind(')')? + 1..];
    let ticks: u64 = after.split_whitespace().nth(19)?.parse().ok()?;
    Some(boot_time()? + ticks / clock_ticks()?)
}

fn boot_time() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    stat.lines()
        .find_map(|line| line.strip_prefix("btime ")?.trim().parse().ok())
}

/// Clock ticks per second (`getconf CLK_TCK`; no libc in xtask).
fn clock_ticks() -> Option<u64> {
    static TICKS: OnceLock<Option<u64>> = OnceLock::new();
    *TICKS.get_or_init(|| {
        let out = std::process::Command::new("getconf")
            .arg("CLK_TCK")
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .ok()
            .filter(|t| *t > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_process_s_own_line_is_live_and_a_gone_pid_is_not() {
        let mine = Owner::parse(&line("/w/here")).expect("parsed");
        assert_eq!(mine.worktree, "/w/here");
        assert!(mine.live());
        assert!(
            !Owner {
                pid: 4_194_305,
                ..mine
            }
            .live()
        );
    }
}
