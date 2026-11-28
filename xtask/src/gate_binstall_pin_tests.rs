//! The cargo-binstall action's `version` input against the tag its SHA
//! is commented with. Without `version` the action installs binstall's
//! latest release whatever the SHA; Dependabot moves the SHA and its
//! comment but not the input, so this fails such a bump until both agree.

use std::path::Path;

use super::WORKFLOW;

/// Matched anywhere in a line, so a quoted `uses: '…'` is found too.
const ACTION: &str = "cargo-bins/cargo-binstall@";

/// The steps that install binstall: one in each job that runs
/// `cargo xtask setup --ci` (ci, web, gpu, window). A count that changes
/// fails here, so a moved workflow or a match that stops finding them is
/// never read as "every step agrees".
const EXPECTED_STEPS: usize = 4;

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

/// `file`'s binstall steps each install the release their SHA is
/// commented with, and there are [`EXPECTED_STEPS`] of them; else the
/// first disagreement, naming the file and line and the fix.
fn check(file: &str, workflow: &str) -> Result<(), String> {
    let uses = binstall_uses(workflow);
    if uses.len() != EXPECTED_STEPS {
        return Err(format!(
            "{file}: {} steps use {ACTION}, expected {EXPECTED_STEPS}; fix: find the missing \
             steps, or change EXPECTED_STEPS in gate_binstall_pin_tests.rs if a job was added or removed",
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

#[test]
fn a_workflow_with_no_binstall_step_is_refused_naming_the_file() {
    // When
    let refused = check("empty.yml", "").expect_err("nothing to check");

    // Then
    assert!(refused.starts_with("empty.yml: 0 steps use"), "{refused}");
}

#[test]
fn fewer_binstall_steps_than_expected_are_refused() {
    // Given: three of the four, each pinned
    let step = "      - uses: cargo-bins/cargo-binstall@abc # v1.28.0\n        with:\n          version: 1.28.0\n";

    // When
    let refused = check("ci.yml", &step.repeat(3)).expect_err("one is missing");

    // Then
    assert!(refused.contains("3 steps use"), "{refused}");
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
