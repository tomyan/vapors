//! The cargo-binstall action's `version` input against the tag its SHA
//! is commented with. Without `version` the action installs binstall's
//! latest release whatever the SHA; Dependabot moves the SHA and its
//! comment but not the input, so this fails such a bump until both agree.

use std::path::Path;

use super::WORKFLOW;

/// Matched anywhere in a line, so a quoted `uses: '…'` is found too.
const ACTION: &str = "cargo-bins/cargo-binstall@";

/// The command that needs binstall: it installs each pinned tool with
/// `cargo binstall` (`setup_install.rs`), so the workflow needs one binstall
/// step per line that runs it. Counting both sides, instead of a constant,
/// keeps one answer to "which jobs install binstall".
const SETUP: &str = "cargo xtask setup --ci";

/// Each use of the action: its line number, the tag in its `# vX.Y.Z`
/// comment, and its `version:` input, if any.
fn binstall_uses(workflow: &str) -> Vec<(usize, Option<String>, Option<String>)> {
    let lines: Vec<&str> = workflow.lines().collect();
    let mut uses = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some(rest) = line.split_once(ACTION).map(|(_, rest)| rest) else {
            continue;
        };
        let tag = rest
            .split_once("# v")
            .map(|(_, tag)| tag.trim().to_string());
        uses.push((at + 1, tag, version_input(&lines[at + 1..])));
    }
    uses
}

/// The `version:` under the step's `with:`, read until the next step.
fn version_input(after: &[&str]) -> Option<String> {
    after
        .iter()
        .take_while(|line| !line.trim_start().starts_with("- "))
        .find_map(|line| line.trim().strip_prefix("version: ").map(str::to_string))
}

/// `file` runs `SETUP` at least once, has one binstall step per run, and
/// each installs the release its SHA is commented with; else the first
/// disagreement, naming the file (and line) and the fix.
fn check(file: &str, workflow: &str) -> Result<(), String> {
    let setups = workflow.lines().filter(|line| line.contains(SETUP)).count();
    if setups == 0 {
        return Err(format!(
            "{file}: no line runs `{SETUP}`, so there is nothing to hold the binstall steps to; \
             fix: point the check at the workflow that installs the pinned tools"
        ));
    }
    let uses = binstall_uses(workflow);
    if uses.len() != setups {
        return Err(format!(
            "{file}: {} steps use {ACTION} but {setups} lines run `{SETUP}`; fix: give each job \
             that runs it a pinned binstall step, or find the step the match missed",
            uses.len()
        ));
    }
    for (line, tag, version) in uses {
        let tag = tag.ok_or(format!("{file}:{line}: the SHA has no `# vX.Y.Z` comment"))?;
        if version.as_deref() != Some(tag.as_str()) {
            let installs = version.unwrap_or("binstall's latest release (no version input)".into());
            return Err(format!(
                "{file}:{line}: the action is commented v{tag} but installs {installs}; \
                 fix: set `with: version: {tag}`"
            ));
        }
    }
    Ok(())
}

#[test]
fn every_binstall_action_installs_the_release_its_sha_is_commented_with() {
    // Given
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let workflow = std::fs::read_to_string(root.join(WORKFLOW)).expect("ci.yml");

    // When
    let checked = check(WORKFLOW, &workflow);

    // Then
    assert_eq!(checked, Ok(()));
}

/// A job that installs binstall pinned to 1.28.0 and runs setup.
const PINNED_JOB: &str = "      - uses: cargo-bins/cargo-binstall@abc # v1.28.0\n        with:\n          version: 1.28.0\n      - name: install pinned tools\n        run: cargo xtask setup --ci\n";

/// A job that runs setup with no binstall step of its own.
const UNINSTALLED_JOB: &str =
    "      - name: install pinned tools\n        run: cargo xtask setup --ci\n";

#[test]
fn a_workflow_that_runs_no_setup_is_refused_naming_the_file() {
    // Given
    let empty = "";

    // When
    let refused = check("empty.yml", empty).expect_err("nothing to check");

    // Then
    assert!(
        refused.starts_with("empty.yml: no line runs `cargo xtask setup --ci`"),
        "{refused}"
    );
}

#[test]
fn a_fifth_setup_job_without_a_binstall_step_is_refused_naming_both_counts() {
    // Given: four pinned jobs and a fifth that runs setup with no binstall
    let workflow = PINNED_JOB.repeat(4) + UNINSTALLED_JOB;

    // When
    let refused = check("ci.yml", &workflow).expect_err("a job is missing its step");

    // Then
    assert!(
        refused.starts_with("ci.yml: 4 steps use cargo-bins/cargo-binstall@ but 5 lines run"),
        "{refused}"
    );
}

#[test]
fn as_many_pinned_steps_as_setup_jobs_pass() {
    // Given
    let workflow = PINNED_JOB.repeat(5);

    // When
    let checked = check("ci.yml", &workflow);

    // Then
    assert_eq!(checked, Ok(()));
}

#[test]
fn a_quoted_use_with_no_version_or_another_version_is_read_as_such() {
    // Given
    let workflow = "      - uses: 'cargo-bins/cargo-binstall@abc' # v1.28.0\n      - name: next\n      - uses: cargo-bins/cargo-binstall@def # v1.28.0\n        with:\n          version: 1.26.2\n      - name: after\n";

    // When
    let uses = binstall_uses(workflow);

    // Then
    assert_eq!(
        uses,
        vec![
            (1, Some("1.28.0".to_string()), None),
            (3, Some("1.28.0".to_string()), Some("1.26.2".to_string())),
        ]
    );
}
