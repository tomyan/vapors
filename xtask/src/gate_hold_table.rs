//! `[gate.shared]` and `[gate.exclusive]` in `.config/factory.toml`:
//! every step the gate runs, by its label, with the reason it may go
//! beside other work or must run alone (`gate_hold.rs`). A step in
//! neither or both, an entry that names no step, or one whose reason is
//! blank, refuses the gate before any step runs.

use std::collections::BTreeMap;

use crate::config::Config;
use crate::gate_hold::Hold;
use crate::gate_plan::PlannedStep;

const SHARED: &str = "gate.shared.";
const EXCLUSIVE: &str = "gate.exclusive.";

/// Each planned step's hold, by its label.
///
/// # Errors
///
/// A step in neither table or in both, an entry naming no planned step,
/// or one with a blank reason, naming it and the fix.
pub fn holds(config: &Config, planned: &[PlannedStep]) -> Result<BTreeMap<String, Hold>, String> {
    let shared = config.entries_under(SHARED);
    let exclusive = config.entries_under(EXCLUSIVE);
    for (entries, table) in [(&shared, "gate.shared"), (&exclusive, "gate.exclusive")] {
        refuse_unused(entries, table, planned)?;
        refuse_blank(entries, table)?;
    }
    planned
        .iter()
        .map(|step| {
            let hold = hold_of(&step.name, &shared, &exclusive)?;
            Ok((step.name.clone(), hold))
        })
        .collect()
}

/// The hold `name` is listed with, in exactly one table.
fn hold_of(
    name: &str,
    shared: &BTreeMap<String, String>,
    exclusive: &BTreeMap<String, String>,
) -> Result<Hold, String> {
    match (shared.contains_key(name), exclusive.contains_key(name)) {
        (true, false) => Ok(Hold::Shared),
        (false, true) => Ok(Hold::Exclusive),
        (true, true) => Err(format!(
            "`{name}` is in both [gate.shared] and [gate.exclusive]; \
             fix: keep it in one in .config/factory.toml, then rerun `cargo xtask gate`"
        )),
        (false, false) => Err(format!(
            "`{name}` is in neither [gate.shared] nor [gate.exclusive]; \
             fix: add `\"{name}\" = \"<why>\"` to one of them in .config/factory.toml, \
             then rerun `cargo xtask gate`"
        )),
    }
}

/// Refuse an entry of `table` whose reason is only spaces.
fn refuse_blank(entries: &BTreeMap<String, String>, table: &str) -> Result<(), String> {
    match entries.iter().find(|(_, reason)| reason.trim().is_empty()) {
        Some((name, _)) => Err(format!(
            "[{table}] `{name}` has no reason; \
             fix: say why it may share the machine or must run alone in .config/factory.toml"
        )),
        None => Ok(()),
    }
}

/// Refuse an entry of `table` that names no planned step.
fn refuse_unused(
    entries: &BTreeMap<String, String>,
    table: &str,
    planned: &[PlannedStep],
) -> Result<(), String> {
    let unused = entries
        .keys()
        .find(|name| !planned.iter().any(|step| &step.name == *name));
    match unused {
        Some(name) => Err(format!(
            "[{table}] `{name}` names no step of {}; \
             fix: rename it to the step's label or remove it in .config/factory.toml",
            crate::gate::WORKFLOW
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "gate_hold_table_tests.rs"]
mod tests;
