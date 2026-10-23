//! The commands a GitHub workflow runs, read line by line (no YAML
//! library): each `run:` value, single-line or a `|` block, split into
//! commands on `&&` and on lines, with ` # ` comments dropped. The gate
//! must run the same commands as the workflow, setup aside, so a check
//! added to CI is a check run locally too.

use std::collections::BTreeSet;

/// Every command the workflow's `run:` steps run, in file order.
#[must_use]
pub fn run_commands(workflow: &str) -> Vec<String> {
    let lines: Vec<&str> = workflow.lines().collect();
    let mut commands = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(value) = line.trim_start().strip_prefix("run:") else {
            continue;
        };
        let value = value.trim();
        if value.starts_with('|') || value.starts_with('>') {
            block(&lines[index + 1..], indent(line)).for_each(|text| commands.extend(split(text)));
        } else {
            commands.extend(split(value));
        }
    }
    commands
}

/// The lines of a block scalar: those indented deeper than its key.
fn block<'a>(rest: &'a [&'a str], key_indent: usize) -> impl Iterator<Item = &'a str> {
    rest.iter()
        .take_while(move |line| line.trim().is_empty() || indent(line) > key_indent)
        .map(|line| line.trim())
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// One line's commands: split on `&&`, comments and blanks dropped.
fn split(line: &str) -> Vec<String> {
    let line = line.split(" # ").next().unwrap_or(line);
    if line.trim_start().starts_with('#') {
        return Vec::new();
    }
    line.split("&&")
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The workflow's commands less setup: those starting with a `setup` prefix.
#[must_use]
pub fn checked_commands(workflow: &str, setup: &[String]) -> Vec<String> {
    run_commands(workflow)
        .into_iter()
        .filter(|command| {
            !setup
                .iter()
                .any(|prefix| command.starts_with(prefix.as_str()))
        })
        .collect()
}

/// `command` without leading `NAME=value` environment assignments.
#[must_use]
pub fn without_env(command: &str) -> &str {
    let mut rest = command.trim_start();
    while let Some((word, tail)) = rest.split_once(' ') {
        let is_assignment = word.split_once('=').is_some_and(|(name, _)| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        });
        if !is_assignment {
            break;
        }
        rest = tail.trim_start();
    }
    rest
}

/// Whether `gate` runs exactly the workflow's checked commands.
///
/// # Errors
///
/// Every command one runs and the other does not, one per line.
pub fn agree(gate: &[String], workflow: &str, setup: &[String]) -> Result<(), String> {
    let ours: BTreeSet<&str> = gate.iter().map(|command| without_env(command)).collect();
    let checked = checked_commands(workflow, setup);
    let theirs: BTreeSet<&str> = checked.iter().map(String::as_str).collect();
    let mut differences: Vec<String> = theirs
        .difference(&ours)
        .map(|command| format!("ci.yml runs `{command}`, the gate does not"))
        .collect();
    differences.extend(
        ours.difference(&theirs)
            .map(|command| format!("the gate runs `{command}`, ci.yml does not")),
    );
    if differences.is_empty() {
        return Ok(());
    }
    Err(differences.join("\n"))
}

#[cfg(test)]
#[path = "gate_workflow_tests.rs"]
mod tests;
