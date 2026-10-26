//! `[queue]` in `.config/factory.toml`: the repository the merge queue
//! serves (`owner/name`), what a merge waits for (`gate`: a `gate --pr`
//! pass for the head; `checks`: GitHub's checks all green), the queue's
//! own worktree, where it rebases and gates (never an agent's), its log,
//! and the worktree that publishes `/` after a merge, when there is one.
//! `targets` lists, space-separated, the roots that hold worktrees'
//! target directories: a merge empties its branch's, and a short disk
//! names the largest. A leading `~/` is the home directory.

use std::path::{Path, PathBuf};

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
    /// The roots holding worktrees' target directories
    /// (`~/.cache/touche-targets`), for emptying and measuring them.
    pub targets: Vec<PathBuf>,
    /// The home directory, which is never emptied.
    pub home: PathBuf,
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
            targets: targets(required("targets")?, home)?,
            home: PathBuf::from(home),
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

/// The space-separated roots in `text`, `~/` read as `home`.
fn targets(text: &str, home: &str) -> Result<Vec<PathBuf>, String> {
    let roots: Vec<&str> = text.split_whitespace().collect();
    if roots.is_empty() {
        return Err(
            "queue.targets names no directory; fix: list the target roots, space-separated".into(),
        );
    }
    let real_home = Path::new(home)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(home));
    roots
        .iter()
        .map(|root| root_dir(root, home, &real_home))
        .collect()
}

/// A targets root, canonical: absolute, an existing directory, and
/// neither `/` nor the home directory, which hold more than build output.
fn root_dir(text: &str, home: &str, real_home: &Path) -> Result<PathBuf, String> {
    let refused = |why: String| format!("queue.targets: `{text}` {why}");
    let path = at_home(text, home);
    if !path.is_absolute() {
        return Err(refused(
            "is not an absolute path; fix: write it from / or ~/".into(),
        ));
    }
    let meta = std::fs::metadata(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => refused(format!(
            "does not exist; fix: mkdir -p {}, or correct [queue] targets",
            path.display()
        )),
        _ => refused(format!(
            "cannot be read ({e}); fix: correct [queue] targets"
        )),
    })?;
    if !meta.is_dir() {
        return Err(refused(
            "is not a directory; fix: correct [queue] targets".into(),
        ));
    }
    let real = path.canonicalize().map_err(|e| {
        refused(format!(
            "cannot be resolved ({e}); fix: correct [queue] targets"
        ))
    })?;
    let whole = if real == Path::new("/") {
        Some("the root directory")
    } else if real == real_home {
        Some("the home directory")
    } else {
        None
    };
    match whole {
        Some(what) => Err(refused(format!(
            "is {what}, which holds more than build output; fix: name a targets root itself, such as ~/.cache/touche-targets"
        ))),
        None => Ok(real),
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

#[cfg(test)]
#[path = "queue_config_targets_tests.rs"]
mod targets_tests;
