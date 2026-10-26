//! Real git repositories for the queue's tests, each removed when its
//! guard drops.

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Repo(pub PathBuf);

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn run(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// git in `dir`, hook variables stripped; whether it succeeded.
pub fn try_run(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("git runs")
        .status
        .success()
}

pub fn commit(dir: &Path, file: &str, text: &str) -> String {
    std::fs::write(dir.join(file), text).expect("file");
    run(dir, &["add", "."]);
    run(dir, &["commit", "-q", "-m", file]);
    run(dir, &["rev-parse", "HEAD"])
}

/// main with one commit, and `feature` branched from it with two.
pub fn repo(name: &str) -> (Repo, String) {
    let dir = std::env::temp_dir().join(format!("xtask-queue-git-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    run(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "base.txt", "base");
    run(&dir, &["checkout", "-q", "-b", "feature"]);
    commit(&dir, "a.txt", "a");
    let head = commit(&dir, "b.txt", "b");
    run(&dir, &["checkout", "-q", "main"]);
    (Repo(dir), head)
}

/// main with ten lines in `f.txt`, and `feature` changing line 5.
pub fn lines_repo(name: &str) -> (Repo, String) {
    let dir = std::env::temp_dir().join(format!("xtask-queue-lines-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    run(&dir, &["init", "-q", "-b", "main"]);
    let lines: Vec<String> = (1..=10).map(|n| format!("line {n}")).collect();
    let text = lines.join("\n") + "\n";
    commit(&dir, "f.txt", &text);
    run(&dir, &["checkout", "-q", "-b", "feature"]);
    let head = commit(&dir, "f.txt", &text.replace("line 5\n", "line five\n"));
    run(&dir, &["checkout", "-q", "main"]);
    (Repo(dir), head)
}
