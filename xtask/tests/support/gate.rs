//! A scratch repository the gate can run in: a workflow of one job whose
//! one step, `ci: check`, says so and fails, doing no work (a `cargo`
//! step would find the workspace above the scratch directory), classified in `[gate.shared]` or `[gate.exclusive]`, and
//! a `.gitignore` that keeps everything else out of the tree, so the
//! gate's clean-tree check passes with HOME, TMPDIR and target/ inside.

use std::path::Path;
use std::process::Command;

pub const WORKFLOW: &str = r"defaults:
  run:
    shell: bash
jobs:
  ci:
    runs-on: ubuntu-24.04
    timeout-minutes: 1
    steps:
      - uses: actions/checkout@v7
      - name: check
        run: echo the step ran && exit 1
      - name: report a timeout
        if: cancelled()
        timeout-minutes: 1
        env:
          GH_TOKEN: ${{ github.token }}
          TIMEOUT_MINUTES: 1
        run: .github/scripts/report-timeout.sh
";

/// `dir` as a committed repository with `settings` and the workflow,
/// its one step held as `hold` (`shared` or `exclusive`).
pub fn workspace(dir: &Path, settings: &str, hold: &str) {
    workspace_with(dir, settings, hold, ("", WORKFLOW));
}

/// [`workspace`], with `gate_keys` added under `[gate]` and `workflow`
/// in place of the one-step workflow.
pub fn workspace_with(dir: &Path, settings: &str, hold: &str, (gate_keys, workflow): (&str, &str)) {
    std::fs::create_dir_all(dir.join(".github/workflows")).expect("workflow dir");
    std::fs::write(dir.join(".github/workflows/ci.yml"), workflow).expect("workflow");
    let gate = format!(
        "[gate]\nhide_gpu_device = \"false\"\n{gate_keys}\n[gate.actions]\n\"actions/checkout\" = \"a checkout\"\n\n\
         [gate.{hold}]\n\"ci: check\" = \"the one step\"\n"
    );
    std::fs::write(
        dir.join(".config/factory.toml"),
        format!("{settings}\n{gate}"),
    )
    .expect("settings");
    std::fs::write(
        dir.join(".gitignore"),
        "/*\n!/.config\n!/.github\n!/.gitignore\n",
    )
    .expect("ignore");
    for args in [
        &["init", "-q"][..],
        &["add", "-A"],
        &["commit", "-q", "-m", "scratch"],
    ] {
        git(dir, args);
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=scratch",
            "-c",
            "user.email=scratch@invalid",
        ])
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} in {}", dir.display());
}
