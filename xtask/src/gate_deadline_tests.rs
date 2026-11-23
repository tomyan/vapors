//! The gate's deadline holds to the workflow and the tables it is
//! derived from.

use std::collections::BTreeMap;

use super::*;
use crate::config::Config;
use crate::gate_hold::Hold;
use crate::gate_plan::PlannedStep;

/// The repository's settings and the plan of its workflow.
pub fn repository_plan() -> (Config, Vec<PlannedStep>) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let settings = Config::load_from(&root).expect("settings");
    let read = |path: &str| std::fs::read_to_string(root.join(path)).map_err(|e| e.to_string());
    let workflow = read(crate::gate::WORKFLOW).expect("workflow");
    let setup = crate::gate::setup_lines(&settings);
    let actions = crate::gate::actions(&settings).expect("actions");
    let sources = crate::gate_plan::Sources {
        setup: &setup,
        actions: &actions,
        read: &read,
    };
    let planned = crate::gate_plan::plan(&workflow, &sources).expect("plan");
    (settings, planned)
}

#[test]
fn the_workflow_takes_as_many_holds_as_the_deadline_counts() {
    // Given
    let (settings, planned) = repository_plan();
    let table = crate::gate_hold_table::holds(&settings, &planned).expect("classified");

    // When
    let holds: Vec<Hold> = planned.iter().map(|step| table[&step.name]).collect();
    let runs = 1 + holds.windows(2).filter(|pair| pair[0] != pair[1]).count();

    // Then
    assert_eq!(runs, usize::try_from(HOLDS).expect("small"));
}

#[test]
fn the_workflow_s_jobs_take_as_long_as_the_deadline_counts() {
    // Given
    let (_, planned) = repository_plan();

    // When
    let jobs: BTreeMap<&str, std::time::Duration> = planned
        .iter()
        .map(|step| (step.job.as_str(), step.job_timeout))
        .collect();

    // Then
    assert_eq!(jobs.values().sum::<std::time::Duration>(), JOBS);
}

#[test]
fn a_gate_s_longest_run_is_its_jobs_two_holds_longest_waits_and_its_own_commands() {
    assert_eq!(HOLD_WAIT, std::time::Duration::from_mins(111));
    assert_eq!(OWN_COMMANDS, std::time::Duration::from_secs(150));
    assert_eq!(LONGEST, std::time::Duration::from_secs(266 * 60 + 30));
}

#[test]
fn the_gate_reads_as_many_checkouts_as_the_deadline_counts() {
    // Given
    let (settings, _) = repository_plan();

    // When
    let pinned = settings.entries_under("gate.pinned.").len();

    // Then
    assert_eq!(pinned + 1, usize::try_from(CHECKOUTS).expect("small"));
}
