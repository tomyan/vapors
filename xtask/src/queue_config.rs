//! `[queue]` in `.config/factory.toml`: the repository the merge queue
//! serves (`owner/name`), what a merge waits for (`gate`: a `gate --pr`
//! pass for the head; `checks`: GitHub's checks all green), the queue's
//! own worktree, where it rebases and gates (never an agent's), its log,
//! and the worktree that publishes `/` after a merge, when there is one.
//! A leading `~/` is the home directory.

use std::path::PathBuf;

use crate::config::Config;

/// What a merge waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeOn {
    /// A `gate: pass <head>` comment (touche).
    Gate,
    /// Every GitHub check green (vapors).
    Checks,
}

/// The queue's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueConfig {
    /// `owner/name` on GitHub.
    pub repo: String,
    /// What a merge waits for.
    pub merge_on: MergeOn,
    /// The queue's own worktree of the repository.
    pub run: PathBuf,
    /// The log file.
    pub log: PathBuf,
    /// The worktree that re-publishes `/` after a merge.
    pub serve: Option<PathBuf>,
}

impl QueueConfig {
    /// The settings in `config`, `~/` read against `home`.
    ///
    /// # Errors
    ///
    /// A missing key, or a mode other than gate or checks, naming it.
    pub fn from(config: &Config, home: &str) -> Result<Self, String> {
        let required = |key: &str| {
            config.get(&format!("queue.{key}")).ok_or_else(|| {
                format!("queue.{key} is not set in .config/factory.toml; fix: add it under [queue]")
            })
        };
        Ok(Self {
            repo: required("repo")?.to_owned(),
            merge_on: merge_on(required("merge_on")?)?,
            run: at_home(required("run")?, home),
            log: at_home(required("log")?, home),
            serve: config.get("queue.serve").map(|dir| at_home(dir, home)),
        })
    }
}

/// `text` as a path, a leading `~/` read as `home`.
fn at_home(text: &str, home: &str) -> PathBuf {
    match text.strip_prefix("~/") {
        Some(rest) => PathBuf::from(home).join(rest),
        None => PathBuf::from(text),
    }
}

fn merge_on(text: &str) -> Result<MergeOn, String> {
    match text {
        "gate" => Ok(MergeOn::Gate),
        "checks" => Ok(MergeOn::Checks),
        other => Err(format!("queue.merge_on is {other:?}; fix: gate or checks")),
    }
}

#[cfg(test)]
#[path = "queue_config_tests.rs"]
mod tests;
