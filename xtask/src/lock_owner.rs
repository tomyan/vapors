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
    ///
    /// Two-valued: a process whose start cannot be read reads as gone.
    /// The gate lock uses it only to name holders; the quiet lock, which
    /// releases on it, uses [`Owner::liveness`].
    pub fn live(&self) -> bool {
        self.liveness().unwrap_or(false)
    }

    /// Whether its process is still the one that wrote the line: false
    /// only when /proc says the pid is gone or was reused.
    ///
    /// # Errors
    ///
    /// Its start could not be read (/proc unreadable, a stat line of
    /// another shape, no boot time or tick rate), naming what.
    pub fn liveness(&self) -> Result<bool, String> {
        let start = process_start(self.pid)?;
        Ok(start.is_some_and(|start| start <= self.since + START_SLACK))
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
/// btime) plus its starttime field in clock ticks; none when it is gone.
fn process_start(pid: u32) -> Result<Option<u64>, String> {
    let path = format!("/proc/{pid}/stat");
    let stat = match std::fs::read_to_string(&path) {
        Ok(stat) => stat,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{path}: {e}")),
    };
    let ticks = start_ticks(&stat, &path)?;
    let boot = boot_time().ok_or("/proc/stat has no readable btime line")?;
    let per_second = clock_ticks().ok_or("`getconf CLK_TCK` gave no tick rate")?;
    Ok(Some(boot + ticks / per_second))
}

/// The starttime field (the 22nd) of a /proc/<pid>/stat line at `path`.
fn start_ticks(stat: &str, path: &str) -> Result<u64, String> {
    stat.rfind(')')
        .and_then(|end| {
            stat[end + 1..]
                .split_whitespace()
                .nth(19)?
                .parse::<u64>()
                .ok()
        })
        .ok_or_else(|| format!("{path} reads `{}`, with no start time", stat.trim()))
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

    #[test]
    fn a_gone_pid_is_not_live_and_says_so_rather_than_failing() {
        let gone = Owner::parse("pid 4194305 in /w since 1").expect("parsed");
        assert_eq!(gone.liveness(), Ok(false));
    }

    #[test]
    fn a_stat_line_without_a_start_time_is_refused_naming_it() {
        let refusal = start_ticks("12 (sh) S 1", "/proc/12/stat").expect_err("short");
        assert!(
            refusal.contains("/proc/12/stat reads `12 (sh) S 1`"),
            "{refusal}"
        );
        let full = format!("1 (a) {}77", "0 ".repeat(19));
        assert_eq!(start_ticks(&full, "p"), Ok(77));
    }
}
